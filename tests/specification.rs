use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    println!();
    let output = Command::new("cargo")
        .arg("r")
        .arg("test")
        .arg("tests/examples.md")
        .arg("rust:.::parse_tests")
        .status()
        .unwrap();

    if output.code().is_none_or(|item| item == 0) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
