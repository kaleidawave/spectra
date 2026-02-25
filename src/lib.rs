pub mod runners;
pub mod utilities;

use utilities::{
    SliceRange, filter, is_equal_ignore_new_line_sequence, run_in_alternative_display,
};

use colored::Colorize as Colourise;
use std::io;

#[derive(Debug)]
pub struct TextWithSource(pub String, pub SliceRange);

impl Default for TextWithSource {
    fn default() -> Self {
        Self(String::new(), 0..0)
    }
}

impl TextWithSource {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

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
    pub expected: TextWithSource,
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
    pub lists_to_code_block: bool,
    pub transform: options::TransformOutput,
    pub merge_stderr: bool,
    pub wildcard_lines: bool,
}

#[must_use]
pub fn extract_tests(content: &str, parent_options: &Options) -> Input {
    use simple_markdown_parser::{CodeBlock, MarkdownElement, QuoteBlock, parse};

    let mut tests: Vec<Test> = Vec::new();

    let mut expected_runner = None;

    let mut last_was_with = false;

    let mut in_block = 0;

    let mut file_options = Options::default();
    let mut test_options = Options::default();

    let mut section = String::new();
    let mut test_name = String::new();

    let mut test_case: String = String::new();
    let mut test_arguments: String = String::new();
    let mut test_expected: TextWithSource = TextWithSource::new();

    // TODO
    // let mut only_language: Option<String> = None;

    let result = parse::<()>(content, |element| {
        let mut is_with = false;

        // TODO
        // let _empty_blocks = false;

        let add_new = if let MarkdownElement::Heading { level, .. } = element {
            level >= 3 && !test_name.is_empty()
        } else if let MarkdownElement::CodeBlock(_) = element {
            !test_expected.is_empty()
        } else {
            false
        };

        if add_new {
            let skip = test_name.ends_with("(skip)");

            let name = if let MarkdownElement::Heading { .. } = element {
                if in_block > 0 {
                    in_block += 1;
                    format!("{test_name} ({in_block})")
                } else {
                    in_block = 0;
                    std::mem::take(&mut test_name)
                }
            } else {
                in_block += 1;
                let this_name = format!("{test_name} ({in_block})");
                test_name = this_name.clone();
                this_name
            };

            let test = Test {
                name,
                skip,
                section: section.clone(),
                arguments: std::mem::take(&mut test_arguments),
                case: std::mem::take(&mut test_case),
                expected: std::mem::take(&mut test_expected),
                wildcard_lines: parent_options.wildcard_lines
                    || file_options.wildcard_lines
                    || std::mem::take(&mut test_options.wildcard_lines),
                merge_stderr: parent_options.merge_stderr
                    || file_options.merge_stderr
                    || std::mem::take(&mut test_options.merge_stderr),
                transform: parent_options
                    .transform
                    .clone()
                    .as_option()
                    .unwrap_or(file_options.transform.clone()),
            };

            tests.push(test);
        }

        match element {
            MarkdownElement::Frontmatter(frontmatter) => {
                let result = frontmatter.parse_yaml(|keys, value| {
                    use simple_yaml_parser::RootYAMLValue;
                    use simple_yaml_parser::YAMLKey::Slice;
                    // dbg!(&keys);

                    match keys {
                        [Slice("expected_runner")] => {
                            if let simple_yaml_parser::RootYAMLValue::String(value) = value {
                                // TODO this will be different in future right?
                                expected_runner = Some(value.to_owned());
                            } else {
                                panic!("expected runner to be string")
                            }
                        }
                        [Slice("include-language-as-option")] => {
                            // prepends *language*\n---\n
                            todo!();
                        }
                        [Slice("transform")] => {
                            // removes lines that end in some substring
                            file_options.transform = match value {
                                // TODO RootYAMLValue::Null => TransformOutput::None,
                                RootYAMLValue::String("basic") => options::TransformOutput::Basic,
                                RootYAMLValue::String("falsy") => options::TransformOutput::Falsy,
                                value => {
                                    panic!(
                                        "unknown {value:?}. expected 'null', 'basic' or 'falsy'"
                                    );
                                }
                            };
                        }
                        [Slice("merge-stderr")] => {
                            // TODO
                            // removes lines that end in some substring
                            file_options.merge_stderr = true;
                        }
                        [Slice("wildcard-lines")] => {
                            // TODO
                            // removes lines that end in some substring
                            file_options.wildcard_lines = true;
                        }
                        [Slice("lists-to-code-blocks")] => {
                            // TODO
                            file_options.lists_to_code_block = true;
                            // if let RootYAMLValue::Boolean(value) = value {
                            // } else {
                            //     eprintln!("expected boolean");
                            // }
                        }
                        keys => {
                            eprintln!("unknown {keys:?} {value:?}");
                        }
                    }
                });
                if let Err(err) = result {
                    eprintln!("error parsing yaml {err:?}");
                }
            }
            MarkdownElement::Heading { level, content } => {
                // TODO
                if level >= 3 {
                    test_name = content.0.to_owned(); //.no_decoration();
                } else {
                    section = content.0.to_owned(); // .no_decoration();
                }
            }
            MarkdownElement::Paragraph(inner) => {
                if let Some(left) = inner.0.strip_prefix("With `")
                    && let Some(options) = left.strip_suffix('`')
                {
                    options.clone_into(&mut test_arguments);
                } else {
                    is_with = inner.0 == "With";
                }
            }
            MarkdownElement::List(list) => {
                let should_add = (parent_options.lists_to_code_block
                    || file_options.lists_to_code_block)
                    && (!test_case.is_empty() && test_expected.is_empty());
                if should_add {
                    let inner = &list.0.0;
                    let start = inner.as_ptr() as usize - content.as_ptr() as usize;
                    let position = start..(start + inner.len());
                    // TODO more efficient
                    let inner = inner.replace("\\<", "<").replace("\\\"", "\"");
                    test_expected = TextWithSource(inner, position);
                }
            }
            MarkdownElement::CodeBlock(CodeBlock { raw_code, .. }) => {
                if test_name.is_empty() {
                    dbg!("unnamed test {raw_code:?}");
                    return Ok(());
                }

                // if last_was_with {
                //     raw_code.clone_into(&mut current_test.options);
                // } else
                if test_case.is_empty() {
                    raw_code.clone_into(&mut test_case);
                } else if test_expected.is_empty() {
                    let inner = raw_code;
                    let start = inner.as_ptr() as usize - content.as_ptr() as usize;
                    test_expected = TextWithSource(inner.to_owned(), start..(start + inner.len()));
                }
            }
            MarkdownElement::Quote(QuoteBlock { inner, .. }) => {
                if inner.0.trim() == "> Merge `stderr` here" {
                    test_options.merge_stderr = true;
                }
            }
            _ => {}
        }

        last_was_with = is_with;
        Ok(())
    });

    assert!(result.is_ok(), "{result:?}");
    {
        let skip = test_name.ends_with("(skip)");

        let name = if in_block > 0 {
            in_block += 1;
            format!("{test_name} ({in_block})")
        } else {
            test_name
        };

        // TODO as macro
        let test = Test {
            name,
            skip,
            section: section.clone(),
            arguments: test_arguments,
            case: test_case,
            expected: test_expected,
            wildcard_lines: parent_options.wildcard_lines
                || file_options.wildcard_lines
                || test_options.wildcard_lines,
            merge_stderr: parent_options.merge_stderr
                || file_options.merge_stderr
                || test_options.merge_stderr,
            // TODO test options
            transform: parent_options
                .transform
                .clone()
                .as_option()
                .unwrap_or(file_options.transform.clone()),
        };

        tests.push(test);
    }

    Input {
        tests,
        expected_runner,
    }
}

#[must_use]
pub fn colour_test_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.contains(['*', '`']) {
        use colored::{Color, ColoredString, Styles};
        use simple_markdown_parser::{MarkdownPart, PartsIterator, TextDecoration};

        let mut buf = String::new();
        for part in PartsIterator::new(name) {
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
    }
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

        let name: std::borrow::Cow<'_, str> = colour_test_name(&test.name);

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
                    }
                    Err(output) => eprintln!("Test {name}\nerrored: {output}"),
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
) -> Result<(), usize> {
    let now = std::time::Instant::now();
    let mut results = TestResults::default();

    let paths = glob::glob(pattern)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|path| path.is_file());

    for path in paths {
        let content = std::fs::read_to_string(&path).unwrap();
        let input = extract_tests(&content, &Options::default());
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
    let input = extract_tests(content, &Options::default());
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
