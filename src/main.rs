use spectra::{
    RunConfiguration, extract_tests, run_tests_under_path,
    runners::program::{Command, Commands},
    utilities::{filter, visit_specification_files},
};

use lahl::{
    CLI, Endpoint, NamedParameter, PositionalParameter, argument_result_or_out,
    command_result_or_out,
};
use std::path::Path;
use std::process::ExitCode;

static TEST_POSITIONAL_PARAMETERS: &[PositionalParameter] = &[
    PositionalParameter::single("markdown", "glob path to markdown files"),
    PositionalParameter::single("command", "command to test against"),
];

// TODO timeout & ignore error
static TEST_NAMED_PARAMETERS: &[NamedParameter] = &[
    NamedParameter::value("only", "only run tests with *value* in the name"),
    NamedParameter::value(
        "only-cs",
        "only run tests with *value* in the name (case-sensitive)",
    ),
    NamedParameter::value("skip", "skip tests with *value* in the name"),
    NamedParameter::value(
        "skip-cs",
        "skip tests with *value* in the name (case-sensitive)",
    ),
    NamedParameter::boolean(
        "interactive",
        "use stdin <-> stdout communication rather that spawning for each test",
    ),
    NamedParameter::boolean("dry-run", "?"),
    NamedParameter::boolean(
        "lists-as-expected",
        "use list blocks as the expected output",
    ),
];

static LIST_NAMED_PARAMETERS: &[PositionalParameter] = &[PositionalParameter::single(
    "path",
    "path to specification-markdown file",
)];

static LIST_PARAMETERS: &[NamedParameter] = &[
    NamedParameter::boolean("debug", "print more information"),
    NamedParameter::boolean("as-json", "print output as JSON"),
    NamedParameter::boolean(
        "lists-as-expected",
        "use list blocks as the expected output",
    ),
    NamedParameter::value("cases-with-splitter", "print cases with passed splitter"),
];

static ENDPOINTS: &[Endpoint] = &[
    Endpoint::new("info", "display information", &[], &[]),
    Endpoint::new(
        "test",
        "run tests",
        TEST_POSITIONAL_PARAMETERS,
        TEST_NAMED_PARAMETERS,
    ),
    Endpoint::new(
        "compare",
        "run two programs against tests",
        &[],
        TEST_NAMED_PARAMETERS,
    ),
    Endpoint::new("list", "list tests", LIST_NAMED_PARAMETERS, LIST_PARAMETERS),
    Endpoint::new_group(
        "install",
        "specification-test-in-cargo",
        "run tests",
        TEST_POSITIONAL_PARAMETERS,
        &[],
    ),
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => err,
    }
}

fn run() -> Result<(), ExitCode> {
    let cli = CLI::new(ENDPOINTS, "spectra", Some("info"));
    let (binary_name, result) = cli.run();

    let (selected, arguments) = command_result_or_out(result, &binary_name)?;

    match selected.name {
        "info" => {
            let run_id = option_env!("GITHUB_RUN_ID");
            let date = option_env!("GIT_LAST_COMMIT").unwrap_or_default();
            let after = run_id
                .map(|commit| format!(" (commit {commit} {date})"))
                .unwrap_or_default();

            println!("spectra (WIP){after} (powered by 'simple-markdown-parser')");
        }
        "test" => {
            let mut markdown = None;
            let mut command = None;
            let mut run_configuration = RunConfiguration::default();

            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "markdown" => {
                        markdown = argument.value;
                    }
                    "command" => {
                        command = argument.value;
                    }
                    // skip and including options
                    name @ ("only" | "skip" | "only-cs" | "skip-cs") => {
                        let matcher = argument.value.unwrap();
                        let filter = filter::StringMatch {
                            case_sensitive: name.ends_with("-cs"),
                            positive: name.starts_with("only"),
                            matcher: matcher.split(',').map(ToOwned::to_owned).collect(),
                        };
                        run_configuration.filter = Some(Box::new(filter));
                    }
                    // run configuration
                    "interactive" => run_configuration.interactive = true,
                    "dry-run" => run_configuration.dry_run = true,
                    "lists-as-expected" => run_configuration.lists_to_code_block = true,
                    // // command configuration
                    // "ignore-exit-code" => command_configuration.ignore_exit_code = true,
                    // "stdin-stdout-communication" => command_configuration.stdin_stdout_communication = true,
                    argument => unreachable!("{argument:?}"),
                }
            }

            let markdown = markdown.unwrap();
            let markdown = Path::new(&markdown);
            let command = command.unwrap();
            let command = Command::new(&command);

            let result = run_tests_under_path(markdown, command, &run_configuration);
            if result.is_err() {
                return Err(ExitCode::FAILURE);
            }
        }
        "compare" => {
            let mut markdown = None;
            let mut command_pattern = None;

            let mut run_configuration = spectra::RunConfiguration {
                dry_run: true,
                ..Default::default()
            };
            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "markdown" => {
                        markdown = argument.value;
                    }
                    "command" => {
                        command_pattern = argument.value;
                    }
                    // skip and including options
                    name @ ("only" | "skip" | "only-cs" | "skip-cs") => {
                        let matcher = argument.value.unwrap();
                        let filter = filter::StringMatch {
                            case_sensitive: name.ends_with("-cs"),
                            positive: name.starts_with("only"),
                            matcher: matcher.split(',').map(ToOwned::to_owned).collect(),
                        };
                        run_configuration.filter = Some(Box::new(filter));
                    }
                    // run configuration
                    "interactive" => run_configuration.interactive = true,
                    "dry-run" => run_configuration.dry_run = true,
                    "lists-as-expected" => run_configuration.lists_to_code_block = true,
                    // // command configuration
                    // "ignore-exit-code" => command_configuration.ignore_exit_code = true,
                    // "stdin-stdout-communication" => command_configuration.stdin_stdout_communication = true,
                    argument => unreachable!("{argument:?}"),
                }
            }

            let markdown = markdown.unwrap();
            let markdown = Path::new(&markdown);

            let command_pattern = command_pattern.unwrap();
            // command'S'
            let command_pattern = Commands::new(&command_pattern);

            let result = run_tests_under_path(markdown, command_pattern, &run_configuration);
            if result.is_err() {
                return Err(ExitCode::FAILURE);
            }
        }
        "list" => {
            let mut path = None;
            let mut debug = false;
            let mut as_json = false;
            let mut lists_to_code_block = false;
            let mut case_splitter = None;

            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "path" => {
                        path = argument.value;
                    }
                    "debug" => {
                        debug = true;
                    }
                    "as-json" => {
                        as_json = true;
                    }
                    "lists-as-expected" => {
                        lists_to_code_block = true;
                    }
                    "cases-with-splitter" => {
                        case_splitter = argument.value;
                    }
                    argument => unreachable!("{argument}"),
                }
            }

            let path = path.unwrap();
            let path = Path::new(&path);

            let mut count = 0;
            let mut files = 0;

            let mut json_buf: String = String::from("[");

            visit_specification_files(path, &mut |path| {
                let content = std::fs::read_to_string(path).unwrap();
                let input = extract_tests(&content, lists_to_code_block);
                if as_json {
                    for test in &input.tests {
                        if json_buf.len() > 1 {
                            json_buf.push(',');
                        }
                        // FUTURE json_builder_macro should support `Option`
                        let expected = test.expected.as_deref().unwrap_or_default();
                        json_buf.push_str(&json_builder_macro::json! {
                            name: test.name, case: test.case, expected: expected
                        });
                    }
                } else {
                    if case_splitter.is_none() {
                        println!("--- {path} ---", path = path.display());
                    }
                    for test in &input.tests {
                        if debug {
                            println!("{test:?}");
                        } else if let Some(splitter) = &case_splitter {
                            if count > 0 {
                                println!("{splitter}");
                            }
                            println!("{case}", case = test.case);
                        } else {
                            println!("{name}", name = test.name);
                        }
                        count += 1;
                    }
                }
                files += 1;
            })
            .expect("could not walk files");

            if as_json {
                json_buf.push(']');
                println!("{json_buf}");
            } else {
                eprintln!("found {count} tests across {files} files");
            }
        }
        "specification-test-in-cargo" => {
            use std::io::Write;

            let mut markdown = None;
            let mut command = None;

            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "markdown" => {
                        markdown = argument.value;
                    }
                    "command" => {
                        command = argument.value;
                    }
                    argument => unreachable!("{argument}"),
                }
            }

            let markdown = markdown.unwrap();
            let command = command.unwrap();

            {
                let mut cargo_toml = std::fs::File::options()
                    .append(true)
                    .open("Cargo.toml")
                    .expect("Cannot open Cargo.toml");

                writeln!(&mut cargo_toml).unwrap();
                writeln!(&mut cargo_toml, "[[test]]").unwrap();
                writeln!(&mut cargo_toml, "name = \"specification\"").unwrap();
                writeln!(&mut cargo_toml, "harness = false").unwrap();
            }

            {
                let _ = std::fs::create_dir("tests");
                let mut test_file = std::fs::File::create_new("tests/specification.rs")
                    .expect("Cannot create tests/specification.rs");

                writeln!(&mut test_file, "use std::process::{{Command, ExitCode}};").unwrap();
                writeln!(&mut test_file, "fn main() -> ExitCode {{").unwrap();
                writeln!(&mut test_file, "let output = Command::new(\"spectra\").arg(\"test\").arg(\"{markdown}\").arg(\"{command}\").status().unwrap();").unwrap();
                writeln!(&mut test_file, "if output.code().is_none_or(|item| item == 0) {{ ExitCode::SUCCESS }} else {{ ExitCode::FAILURE }}").unwrap();
                writeln!(&mut test_file, "}}").unwrap();
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
