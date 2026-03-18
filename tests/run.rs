use spectra::{
    Options, RunConfiguration, parsing::extract_tests, run_tests, runners::program::Command,
};

static SPECIFICATION_UPPERCASE: &str = include_str!("../include/specification.uppercase.md");
static SPECIFICATION_LIST: &str = include_str!("../include/specification.lists.md");
static SPECIFICATION_OPTIONS: &str = include_str!("../include/specification.options.md");

/// test output during testing can get confusing
fn no_output_run_configuration() -> RunConfiguration {
    RunConfiguration {
        skip_print_test_results: true,
        ..RunConfiguration::default()
    }
}

#[test]
fn pass() {
    let input = extract_tests(SPECIFICATION_UPPERCASE, &Options::default());

    let mut runner = Command::new("bun run examples/example_program.js {content} --uppercase");
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    assert!(results.failures.is_empty());

    let mut runner = Command::new("bun run examples/example_program.js {content}");
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    assert_eq!(results.failures.len(), 3);
}

#[test]
fn pass_stdout_stderr() {
    let input = extract_tests(SPECIFICATION_UPPERCASE, &Options::default());

    let mut runner =
        Command::new("bun run examples/example_program.js --uppercase --rpc --interactive");
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    assert!(results.failures.is_empty());

    let mut runner = Command::new("bun run examples/example_program.js --rpc --interactive");
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    assert_eq!(results.failures.len(), 3);
}

#[test]
fn pass_lists() {
    let options = Options {
        lists_as_expected: true,
        ..Options::default()
    };
    let input = extract_tests(SPECIFICATION_LIST, &options);

    let mut runner =
        Command::new("bun run examples/example_program.js {content} --uppercase --use-lists");
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    if !results.failures.is_empty() {
        for (test, _, out) in &results.failures {
            println!("{test}\n{out}");
        }
        panic!("not empty")
    }

    let mut runner = Command::new("bun run examples/example_program.js --use-lists");
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    assert_eq!(results.failures.len(), 3);
}

#[test]
fn program_crash() {
    let input = extract_tests(SPECIFICATION_UPPERCASE, &Options::default());

    let commands: &[&str] = &[
        "bun run examples/example_program.js --uppercase --rpc --intentional-crash --interactive",
        "cargo run --example example_program -- --uppercase --rpc --intentional-crash --interactive",
    ];

    for mut runner in commands.iter().copied().map(Command::new) {
        let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
        assert_eq!(results.failures.len(), 1);
        assert_eq!(
            results.failures.get(0).map(|(lhs, _, _)| lhs.as_str()),
            Some("Test 2")
        );
    }
}

#[test]
fn program_timeout() {
    let input = extract_tests(SPECIFICATION_UPPERCASE, &Options::default());

    let mut runner = Command::new(
        "bun run examples/example_program.js --uppercase --rpc --interactive --intentional-timeout --timeout 1000",
    );
    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    // test 2 does not run in under 1000 ms
    assert_eq!(
        &results.failures,
        &[(
            "Test 2".into(),
            String::default(),
            "PROCESS TIMED OUT".into()
        )]
    );

    let mut runner = Command::new(
        "bun run examples/example_program.js --uppercase --rpc --interactive --intentional-timeout --timeout 5000",
    );

    let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
    assert!(results.failures.is_empty());
}

#[test]
fn program_options() {
    let input = extract_tests(SPECIFICATION_OPTIONS, &Options::default());

    let commands: &[&str] = &[
        "bun run examples/example_program.js --uppercase --rpc --interactive",
        "cargo r --example example_program -- --uppercase --rpc --interactive",
    ];

    for mut runner in commands.iter().copied().map(Command::new) {
        let results = run_tests(&input.tests, &mut runner, &no_output_run_configuration());
        assert!(
            &results.failures.is_empty(),
            "found failures {failures:#?}",
            failures = &results.failures
        );
    }
}
