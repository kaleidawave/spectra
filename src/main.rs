use spectra::{
    RunConfiguration, parsing::extract_tests, run_tests_under_glob, runners, utilities::filter,
};

use lahl::{
    CLI, Endpoint, NamedParameter, PositionalParameter, argument_result_or_out,
    command_result_or_out,
};
use std::process::ExitCode;

static TEST_POSITIONAL_PARAMETERS: &[PositionalParameter] = &[
    PositionalParameter::single("pattern", "glob path to markdown files"),
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
    NamedParameter::boolean(
        "dry-run",
        "run and print output of test cases without comparison",
    ),
    NamedParameter::boolean("infill", "replace ??? expected blocks with command output"),
    NamedParameter::value("config", "YAML file with configuration for tests"),
];

static LIST_NAMED_PARAMETERS: &[PositionalParameter] = &[PositionalParameter::single(
    "pattern",
    "pattern to specification-markdown file",
)];

static LIST_PARAMETERS: &[NamedParameter] = &[
    NamedParameter::boolean("debug", "print more information"),
    NamedParameter::boolean("json", "print output as JSON"),
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

fn info() {
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or_default();
    let run_id: String = if let Some(run_id) = option_env!("GITHUB_RUN_ID") {
        format!(" (run: {run_id}")
    } else {
        String::new()
    };
    let last_commit: String = if let Some(commit) = option_env!("GIT_LAST_COMMIT") {
        let commit: &str = &commit[..7];
        format!(" (commit: {commit}")
    } else {
        String::new()
    };
    println!("spectra@{version}{last_commit}{run_id} (powered by 'simple-markdown-parser')");
}

fn run() -> Result<(), ExitCode> {
    let cli = CLI::new(ENDPOINTS, "spectra", Some("info"));
    let (binary_name, result) = cli.run();

    let (selected, arguments) = command_result_or_out(result, &binary_name)?;

    match selected.name {
        "info" => {
            info();
            Ok(())
        }
        "test" | "compare" => {
            let mut pattern = None;
            let mut command = None;
            let mut config_file = None;
            let mut run_configuration = RunConfiguration::default();

            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "pattern" => {
                        pattern = argument.value;
                    }
                    "command" => {
                        command = argument.value;
                    }
                    "config" => {
                        config_file = argument.value;
                    }
                    // skip and including options
                    name @ ("only" | "skip" | "only-cs" | "skip-cs") => {
                        let matcher = argument.value.unwrap();
                        let filter = filter::GlobPattern {
                            case_sensitive: name.ends_with("-cs"),
                            positive: name.starts_with("only"),
                            matcher: glob::Pattern::new(&matcher).expect("invalid glob pattern"),
                        };
                        run_configuration.filter = Some(Box::new(filter));
                    }
                    // run configuration
                    "interactive" => run_configuration.interactive = true,
                    "dry-run" => run_configuration.dry_run = true,
                    "infill" => run_configuration.infill = true,
                    // // command configuration
                    // "ignore-exit-code" => command_configuration.ignore_exit_code = true,
                    // "stdin-stdout-communication" => command_configuration.stdin_stdout_communication = true,
                    argument => unreachable!("{argument:?}"),
                }
            }

            let pattern = pattern.unwrap();

            if selected.name == "compare" {
                let command_pattern = command.unwrap();
                // command'S'
                let command_pattern = runners::program::Commands::new(&command_pattern);

                let result = run_tests_under_glob(
                    &pattern,
                    command_pattern,
                    &run_configuration,
                    config_file,
                );
                if result.is_ok() {
                    Ok(())
                } else {
                    Err(ExitCode::FAILURE)
                }
            } else {
                let command = command.unwrap();
                if let Some(after) = command.strip_prefix("rust:") {
                    let (path, name) = after.split_once("::").unwrap_or((after, "test"));
                    let runner = runners::compiled::rust::Rust::new(path, name);
                    match runner {
                        Ok(runner) => {
                            let result = run_tests_under_glob(
                                &pattern,
                                runner,
                                &run_configuration,
                                config_file,
                            );
                            if result.is_ok() {
                                Ok(())
                            } else {
                                Err(ExitCode::FAILURE)
                            }
                        }
                        Err(err) => {
                            eprintln!("Error building Rust, {err:?}");
                            return Err(ExitCode::FAILURE);
                        }
                    }
                } else {
                    let command = runners::program::Command::new(&command);
                    let result =
                        run_tests_under_glob(&pattern, command, &run_configuration, config_file);
                    if result.is_ok() {
                        Ok(())
                    } else {
                        Err(ExitCode::FAILURE)
                    }
                }
            }
        }
        "list" => {
            let mut pattern = None;
            let mut debug = false;
            let mut as_json = false;
            let mut case_splitter = None;

            // TODO filter

            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "pattern" => {
                        pattern = argument.value;
                    }
                    "debug" => {
                        debug = true;
                    }
                    "json" => {
                        as_json = true;
                    }
                    "cases-with-splitter" => {
                        case_splitter = argument.value;
                    }
                    argument => unreachable!("{argument}"),
                }
            }

            let pattern = pattern.unwrap();

            let mut count = 0;
            let mut files = 0;

            let mut json_buf: String = String::from("[");

            let paths = glob::glob(&pattern)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|path| path.is_file());

            for path in paths {
                let content = std::fs::read_to_string(&path).unwrap();
                let input = extract_tests(&content, &spectra::Options::default()).expect("TODO unwrap");
                if as_json {
                    for test in &input.tests {
                        if json_buf.len() > 1 {
                            json_buf.push(',');
                        }
                        // FUTURE json_builder_macro should support `Option`
                        // let transform = test.transform.as_option().map(|transform| format!("{:?}", test.transform));
                        let transform = format!("{:?}", test.transform);
                        json_buf.push_str(&json_builder_macro::json! {
                            name: test.name,
                            case: test.case,
                            expected: test.expected.0,
                            transform: transform,
                            wildcard_lines: test.wildcard_lines,
                            merge_stderr: test.merge_stderr,
                            skip: test.skip,
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
                            println!(
                                "{name}",
                                name = spectra::utilities::colour_test_name(&test.name)
                            );
                        }
                        count += 1;
                    }
                }
                files += 1;
            }

            if as_json {
                json_buf.push(']');
                println!("{json_buf}");
            } else {
                eprintln!("found {count} tests across {files} files");
            }
            Ok(())
        }
        "specification-test-in-cargo" => {
            use std::io::Write;

            let mut pattern = None;
            let mut command = None;

            for argument in arguments {
                let argument = argument_result_or_out(argument)?;
                match argument.name {
                    "pattern" => {
                        pattern = argument.value;
                    }
                    "command" => {
                        command = argument.value;
                    }
                    argument => unreachable!("{argument}"),
                }
            }

            let pattern = pattern.unwrap();
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
                writeln!(&mut test_file, "let output = Command::new(\"spectra\").arg(\"test\").arg(\"{pattern}\").arg(\"{command}\").status().unwrap();").unwrap();
                writeln!(&mut test_file, "if output.code().is_none_or(|item| item == 0) {{ ExitCode::SUCCESS }} else {{ ExitCode::FAILURE }}").unwrap();
                writeln!(&mut test_file, "}}").unwrap();
            }

            Ok(())
        }
        _ => unreachable!(),
    }
}
