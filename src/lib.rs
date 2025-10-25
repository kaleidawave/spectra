pub mod runners;
pub mod utilities;

use utilities::{
    filter, is_equal_ignore_new_line_sequence, run_in_alternative_display,
    visit_specification_files,
};

use colored::Colorize as Colourise;
use std::io;

/// TODO vec of vecs
#[derive(Debug, Default)]
pub struct Test {
    pub section: String,
    pub name: String,
    pub options: String,
    pub case: String,
    pub expected: Option<String>,
    pub command: bool,
    pub merge_stderr: bool,
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
    pub lists_to_code_block: bool,
    pub no_colors: bool,
    pub filter: Option<Box<dyn filter::Filter>>,
    pub skip_print_test_results: bool,
}

pub struct Input {
    pub tests: Vec<Test>,
    pub expected_runner: Option<String>,
}

#[must_use]
pub fn extract_tests(content: &str, lists_to_code_block: bool) -> Input {
    use simple_markdown_parser::{CodeBlock, MarkdownElement, QuoteBlock, parse};

    let mut tests: Vec<Test> = Vec::new();
    let mut current_test = Test::default();
    let mut section = String::new();

    let mut expected_runner = None;
    // let mut total_options = String::new();

    let mut last_was_with = false;

    let result = parse::<()>(content, |element| {
        let mut is_with = false;
        match element {
            MarkdownElement::Heading { level, content } => {
                if level >= 3 {
                    if !current_test.case.is_empty() {
                        tests.push(std::mem::take(&mut current_test));
                    }
                    current_test.name = content.0.to_owned(); //.no_decoration();
                    section.clone_into(&mut current_test.section);
                } else {
                    section = content.0.to_owned(); // .no_decoration();
                }
            }
            MarkdownElement::Paragraph(content) => {
                if let Some(left) = content.0.strip_prefix("With `")
                    && let Some(options) = left.strip_suffix('`')
                {
                    options.clone_into(&mut current_test.options);
                } else {
                    is_with = content.0 == "With";
                }
            }
            MarkdownElement::List(list) if lists_to_code_block => {
                if !current_test.case.is_empty() && current_test.expected.is_none() {
                    let content = &list.0.0;
                    // TODO more efficient
                    let content = content.replace("\\<", "<").replace("\\\"", "\"");
                    let _ = current_test.expected.insert(content);
                }
            }
            MarkdownElement::CodeBlock(CodeBlock { code, .. }) => {
                if last_was_with {
                    code.clone_into(&mut current_test.options);
                } else if current_test.case.is_empty() {
                    code.clone_into(&mut current_test.case);
                } else if current_test.expected.is_none() {
                    let _ = current_test.expected.insert(code.to_owned());
                } else {
                    // create a new test
                    let next_name = format!("{} *", current_test.name);
                    tests.push(std::mem::take(&mut current_test));
                    current_test.name = next_name;
                }
            }
            MarkdownElement::Quote(QuoteBlock { inner, .. }) => {
                if inner.0.trim() == "> Merge `stderr` here" {
                    current_test.merge_stderr = true;
                }
            }
            MarkdownElement::Frontmatter(frontmatter) => {
                let result = frontmatter.parse_yaml(|keys, value| {
                    if let [simple_yaml_parser::YAMLKey::Slice("expected_runner")] = keys {
                        if let simple_yaml_parser::RootYAMLValue::String(value) = value {
                            // TODO this will be different in future right?
                            expected_runner = Some(value.to_owned());
                        } else {
                            panic!("expected runner to be string")
                        }
                    } else {
                        eprintln!("unknown {keys:?} {value:?}");
                    }
                });
                if let Err(err) = result {
                    eprintln!("{err:?}");
                }
            }
            _ => {}
        }

        last_was_with = is_with;
        Ok(())
    });

    assert!(result.is_ok(), "{result:?}");
    if !current_test.case.is_empty() {
        tests.push(current_test);
    }

    Input {
        tests,
        expected_runner,
    }
}

#[derive(Debug, Default)]
pub struct TestResults {
    pub count: usize,
    pub skipped: usize,
    // name, comparison, stderr
    pub failures: Vec<(String, String, String)>,
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
        let name = &test.name;

        let skip_test = configuration
            .filter
            .as_ref()
            .is_some_and(|filter| filter.should_skip(&test.name));

        if skip_test {
            results.skipped += 1;
        }

        let name: std::borrow::Cow<'_, str> = if name.contains(['*', '`']) {
            use colored::{Color, ColoredString, Styles};
            use simple_markdown_parser::{
                ContainerResidue, MarkdownPart, PartsIterator, TextDecoration,
            };

            let mut buf = String::new();
            for part in PartsIterator::new(name, ContainerResidue::default()) {
                let mut decorated: ColoredString = part.on.into();
                if let MarkdownPart::InlineCode = part.kind {
                    decorated.fgcolor = Some(Color::Black);
                    decorated.bgcolor = Some(Color::BrightBlack);
                }
                if part.decoration.contains(TextDecoration::EMPHASIS) {
                    decorated.style.add(Styles::Italic);
                }
                if part.decoration.contains(TextDecoration::BOLD) {
                    decorated.style.add(Styles::Bold);
                }

                std::fmt::Write::write_fmt(&mut buf, format_args!("{decorated}")).unwrap();
            }
            buf.into()
        } else {
            name.into()
        };

        if configuration.dry_run {
            // TODO should dry run print debug out
            if !skip_test {
                let result = runner.run(test);
                if configuration.interactive {
                    let should_break = run_in_alternative_display(|| {
                        match result {
                            Ok((output, _debug)) => eprintln!("Test {name}\nrecieved:\n{output}"),
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
                        Ok((output, _debug)) => eprintln!("Test {name}\nrecieved:\n{output}"),
                        Err(output) => eprintln!("Test {name}\nerrored: {output}"),
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
                    if let Some(ref expected) = test.expected {
                        if is_equal_ignore_new_line_sequence(&output, expected) {
                            Ok(())
                        } else {
                            let comparison =
                                pretty_assertions::StrComparison::new(expected, &output)
                                    .to_string();
                            Err((comparison, debug))
                        }
                    } else {
                        Ok(())
                    }
                }
                Err(err) => Err((String::default(), err)),
            };

            if !configuration.skip_print_test_results {
                if result.is_ok() {
                    println!("test {name} ... {result}", result = "pass".green()); // "passed"?
                } else {
                    println!("test {name} ... {result}", result = "fail".red()); // "failed" ?
                }
            }

            if let Err((output, debug)) = result {
                results
                    .failures
                    .push((test.name.to_string(), output, debug));
            }
        }
    }

    results
}

pub fn run_tests_under_path(
    path: &std::path::Path,
    mut runner: impl Runner,
    configuration: &RunConfiguration,
) -> Result<(), usize> {
    let now = std::time::Instant::now();
    let mut results = TestResults::default();

    let () = visit_specification_files(path, &mut |path| {
        let content = std::fs::read_to_string(path).unwrap();
        let input = extract_tests(&content, configuration.lists_to_code_block);
        let result = run_tests(&input.tests, &mut runner, configuration);
        results.append(result);
    })
    .expect("could not visit files");

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
    let input = extract_tests(content, configuration.lists_to_code_block);
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
