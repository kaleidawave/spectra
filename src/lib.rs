pub mod parsing;
pub mod runners;
pub mod utilities;

use utilities::{filter, is_equal_ignore_new_line_sequence, run_in_alternative_display};

use colored::Colorize as Colourise;

use std::io;

#[derive(Debug)]
pub struct Test {
    /// a grouping for the test
    pub section: String,
    /// the exact name
    pub name: String,
    /// options / arguments passed to the type
    pub arguments: String,
    /// the actual case
    pub case: String,
    /// the expected output
    pub expected: utilities::TextWithSource,
    /// has (skip) in title. We add it to the list of tests to make user aware
    pub skip: bool,
    /// whether to merge stderr stream into the expected test out
    pub merge_stderr: bool,
    /// whether to allow `?` in expected to match anything
    pub wildcard_lines: bool,
    /// a function to run the output through before equating with expected
    pub transform: options::TransformOutput,
}

pub trait Runner: Sized {
    /// Returns `Ok((*stdout*, *stderr*))`
    ///
    /// # Errors
    /// if test failed on runner, return a `Err` with some message about why it failed
    fn run(&mut self, test: &Test) -> Result<(String, String), String>;

    /// Cleanup
    fn close(self) {}
}

#[derive(Default)]
pub struct RunConfiguration {
    pub interactive: bool,
    pub dry_run: bool,
    pub infill: bool,
    pub no_colors: bool,
    pub filter: Option<Box<dyn filter::Filter>>,
    pub skip_print_test_results: bool,
}

pub struct Input {
    pub tests: Vec<Test>,
    pub expected_runner: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Options {
    pub lists_as_code_blocks: bool,
    pub transform: options::TransformOutput,
    pub merge_stderr: bool,
    pub wildcard_lines: bool,
}

#[derive(Debug, Default)]
pub struct TestResults {
    pub count: usize,
    pub skipped: usize,
    /// name, comparison, stderr
    pub failures: Vec<(String, String, String)>,
    /// for infill
    pub changes: utilities::changes::Changes,
}

impl TestResults {
    pub fn append(&mut self, mut new: TestResults) {
        self.count += new.count;
        self.skipped += new.skipped;
        self.failures.append(&mut new.failures);
    }
}

pub fn run_tests(
    tests: &[Test],
    runner: &mut impl Runner,
    configuration: &RunConfiguration,
) -> TestResults {
    let mut results = TestResults::default();

    for test in tests {
        results.count += 1;

        let skip_test = test.skip
            || configuration
                .filter
                .as_ref()
                .is_some_and(|filter| filter.should_skip(&test.name));

        if skip_test {
            results.skipped += 1;
        }

        let name: std::borrow::Cow<'_, str> = utilities::colour_test_name(&test.name);

        if configuration.dry_run {
            // TODO should dry run print debug out
            if !skip_test {
                let result = runner.run(test);
                if configuration.interactive {
                    let should_break = run_in_alternative_display(|| {
                        match result {
                            Ok((output, _debug)) => {
                                let output = test.transform.transform(output);
                                eprintln!("Test {name}\nrecieved:\n{output}");
                            }
                            Err(output) => eprintln!("Test {name}\nerrored: {output}"),
                        }

                        let mut input = String::new();
                        io::stdin()
                            .read_line(&mut input)
                            .expect("Failed to read line");

                        matches!(input.as_str().trim(), "exit" | "e" | "quit" | "q")
                    });
                    if should_break {
                        break;
                    }
                } else {
                    match result {
                        Ok((output, _debug)) => {
                            let output = test.transform.transform(output);
                            eprintln!("Test {name}\nrecieved:\n{output}");
                        }
                        Err(output) => eprintln!("Test {name}\nerrored: {output}"),
                    }
                }
            }
        } else if configuration.infill && test.expected.0 == "???" {
            if !skip_test {
                let result = runner.run(test);
                match result {
                    Ok((output, _debug)) => {
                        let output = test.transform.transform(output);
                        results.changes.push((test.expected.1.clone(), output));
                        println!("test {name} ... {result}", result = "infilled".purple());
                    }
                    Err(output) => {
                        println!(
                            "test {name} ... {result} {output}",
                            result = "errored".red()
                        );
                    }
                }
            }
        } else if skip_test {
            if !configuration.skip_print_test_results {
                println!("test {name} ... {result}", result = "skipped".blue());
            }
        } else {
            let result = runner.run(test);
            let result = match result {
                Ok((output, debug)) => {
                    let output = test.transform.transform(output);
                    if is_equal_ignore_new_line_sequence(
                        &output,
                        &test.expected.0,
                        test.wildcard_lines,
                    ) {
                        Ok(())
                    } else {
                        let comparison =
                            pretty_assertions::StrComparison::new(&test.expected.0, &output)
                                .to_string();
                        Err((comparison, debug))
                    }
                }
                Err(err) => Err((String::default(), err)),
            };

            if !configuration.skip_print_test_results {
                if result.is_ok() {
                    println!("test {name} ... {result}", result = "ok".green()); // "passed"?
                } else {
                    println!("test {name} ... {result}", result = "fail".red()); // "failed" ?
                }
            }

            if let Err((output, debug)) = result {
                results.failures.push((test.name.clone(), output, debug));
            }
        }
    }

    results
}

pub fn run_tests_under_glob(
    pattern: &str,
    mut runner: impl Runner,
    configuration: &RunConfiguration,
    config_file: Option<String>,
) -> Result<(), usize> {
    let now = std::time::Instant::now();
    let mut results = TestResults::default();

    let paths = glob::glob(pattern)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|path| path.is_file());

    let test_options = if let Some(config_file) = config_file {
        let file = std::fs::read_to_string(config_file).unwrap();
        parsing::parse_yaml_config(&file)
    } else {
        Options::default()
    };

    for path in paths {
        let content = std::fs::read_to_string(&path).unwrap();
        let input = parsing::extract_tests(&content, &test_options);
        let mut result = run_tests(&input.tests, &mut runner, configuration);
        if !result.changes.is_empty() {
            utilities::changes::apply_changes(
                &mut std::fs::File::create(path).unwrap(),
                &content,
                std::mem::take(&mut result.changes),
            );
        }
        results.append(result);
    }

    runner.close();

    let elapsed = now.elapsed();
    if configuration.dry_run {
        Ok(())
    } else {
        let failures = results.failures.len();
        if !configuration.skip_print_test_results {
            print_test_results(results, configuration, elapsed);
        }
        if failures == 0 { Ok(()) } else { Err(failures) }
    }
}

/// Runs tests with runner and configuration, printing errors to stdout and stderr.
/// The output (should) mirror Rust's default test harness
///
/// # Errors
/// returns the number of failed tests
pub fn run_tests_under_content(
    content: &str,
    mut runner: impl Runner,
    configuration: &RunConfiguration,
) -> Result<(), usize> {
    let input = parsing::extract_tests(content, &Options::default());
    let count = input.tests.len();

    println!("\nrunning {count} tests");

    let now = std::time::Instant::now();

    let results = run_tests(&input.tests, &mut runner, configuration);
    let elapsed = now.elapsed();
    if configuration.dry_run {
        Ok(())
    } else {
        let failures = results.failures.len();
        if !configuration.skip_print_test_results {
            print_test_results(results, configuration, elapsed);
        }
        if failures == 0 { Ok(()) } else { Err(failures) }
    }
}

pub fn print_test_results(
    results: TestResults,
    configuration: &RunConfiguration,
    elapsed: std::time::Duration,
) {
    let TestResults {
        count,
        failures,
        skipped,
        changes: _,
    } = results;

    if !failures.is_empty() {
        eprintln!("\nfailures:\n");

        if configuration.interactive {
            run_in_alternative_display(|| {
                for (name, message, stdout) in &failures {
                    eprintln!("test {name} failed\n{message}\n{stdout}");

                    {
                        let mut input = String::new();
                        io::stdin()
                            .read_line(&mut input)
                            .expect("Failed to read line");

                        if let "exit" | "e" | "quit" | "q" = input.as_str().trim() {
                            break;
                        }
                    }
                }
            });
        } else {
            for (name, message, stdout) in &failures {
                eprintln!("test {name} failed\n{message}\n{stdout}");
            }
        }

        // TODO on single line?
        eprintln!("\nfailures:");
        for (name, ..) in &failures {
            eprintln!("\t{name}");
        }
    }

    let result = if failures.is_empty() { "ok" } else { "err" };
    let passed = count - (failures.len() + skipped);
    let failed = failures.len();

    // FUTURE will we support these?
    let ignored = 0;
    let measured = 0;
    let filtered_out = skipped;

    eprintln!(
        "\ntest result: {result}. {passed} passed; {failed} failed; {ignored} ignored; {measured} measured; {filtered_out} filtered out; finished in {elapsed:?}"
    );
}

pub mod options {
    #[derive(Debug, Default, Clone)]
    pub enum TransformOutput {
        #[default]
        None,
        Basic,
        Falsy,
    }

    impl TransformOutput {
        #[must_use]
        pub fn transform(&self, on: String) -> String {
            match self {
                Self::None => on,
                Self::Basic | Self::Falsy => {
                    let mut buf = String::new();
                    for line in on.lines() {
                        let tline = line.strip_suffix(',').unwrap_or(line);
                        // WIP
                        let skip = tline.ends_with("None")
                            || tline.ends_with("\"\"")
                            || tline.ends_with("[]")
                            || tline.ends_with("()")
                            || tline.ends_with("{}")
                            || {
                                if let Self::Falsy = self {
                                    tline.ends_with("false")
                                } else {
                                    false
                                }
                            };

                        /* || tline.ends_with("]") || tline.ends_with("}") || tline.ends_with(")"); */

                        if !skip {
                            if !buf.is_empty() {
                                buf.push('\n');
                            }
                            buf.push_str(line);
                        }
                    }
                    buf
                }
            }
        }

        #[must_use]
        pub fn as_option(self) -> Option<Self> {
            if let Self::None = self {
                None
            } else {
                Some(self)
            }
        }
    }
}
