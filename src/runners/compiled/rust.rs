use crate::{Runner, Test};

// FUTURE this may have problems with FFI
type FunctionType = unsafe extern "Rust" fn(&str) -> Result<String, String>;

pub struct Rust {
    /// we need to hold this so that the function is valid
    _library: libloading::Library,
    /// function callback
    function: libloading::Symbol<'static, FunctionType>,
}

// TODO after `compiler-artifact`?
fn get_output_name_from_json(json_output: &str) -> Option<&str> {
    let prefix = "\"filenames\":[\"";
    let start: usize = json_output.rfind(prefix)? + prefix.len();
    let after: &str = &json_output[start..];
    let end = after.find('\"')?;
    Some(&after[..end])
}

#[cfg(test)]
#[test]
fn test() {
    let out = get_output_name_from_json(
        r#"{"reason":"compiler-artifact","package_id":"path+file:///Users/benjamin/Projects/spectra/examples/rust-runner#0.1.0","manifest_path":"/Users/benjamin/Projects/spectra/examples/rust-runner/Cargo.toml","target":{"kind":["cdylib"],"crate_types":["cdylib"],"name":"rust_runner","src_path":"/Users/benjamin/Projects/spectra/examples/rust-runner/lib.rs","edition":"2024","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":["/Users/benjamin/Projects/spectra/examples/rust-runner/target/debug/librust_runner.dylib"],"executable":null,"fresh":true}"#,
    );
    assert_eq!(
        out,
        Some(
            "/Users/benjamin/Projects/spectra/examples/rust-runner/target/debug/librust_runner.dylib"
        )
    );
}

impl Rust {
    #[allow(clippy::missing_transmute_annotations, clippy::used_underscore_binding)]
    pub fn new(path: &str, name: &str, example: Option<&str>) -> Result<Self, String> {
        let target_args: &[&str] = if let Some(example) = example {
            &["--example", example]
        } else {
            &["--lib"]
        };

        // cargo rustc --crate-type cdylib (--lib or --example)
        let output = std::process::Command::new("cargo")
            .arg("rustc")
            .arg("--crate-type")
            .arg("cdylib")
            .arg("--message-format")
            .arg("json")
            .args(target_args)
            .stderr(std::process::Stdio::inherit())
            .current_dir(path)
            .output();

        let Ok(output) = output else {
            // Run the compiler again but print the errors to reader usable form.
            // Not sure if there is a better way?
            let _output = std::process::Command::new("cargo")
                .arg("rustc")
                .arg("--crate-type")
                .arg("cdylib")
                .args(target_args)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .current_dir(path)
                .output();

            return Err(format!(
                "could not build library ({name:?}) in {path:?} (error running command)"
            ));
        };
        if !output.status.success() {
            // Run the compiler again but print the errors to reader usable form.
            // Not sure if there is a better way?
            let _output = std::process::Command::new("cargo")
                .arg("rustc")
                .arg("--crate-type")
                .arg("cdylib")
                .args(target_args)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .current_dir(path)
                .output();
            return Err(String::new());
        }

        let out_json = str::from_utf8(&output.stdout).unwrap();
        let Some(artifact_name) = get_output_name_from_json(out_json) else {
            return Err(format!("JSON does not contain artifact: {out_json}"));
        };

        unsafe {
            let Ok(_library) = libloading::Library::new(artifact_name) else {
                return Err(format!("library {artifact_name:?} does not exist"));
            };
            let Ok(function): Result<libloading::Symbol<'_, FunctionType>, _> =
                _library.get(name.as_bytes())
            else {
                return Err(format!(
                    "library {artifact_name:?} does not have export {name}"
                ));
            };

            // Promote to higher lifetime as library is owned
            let function = std::mem::transmute(function);
            Ok(Self { _library, function })
        }
    }
}

impl Runner for Rust {
    fn run(&mut self, test: &Test) -> Result<(String, String), String> {
        // let thread =
        //     std::thread::scope(move |s| {
        //         s.spawn(move ||  })
        //     });
        //     = thread.join();
        // TODO wrap in thread spawn?
        let case: &str = &test.case;
        let result: Result<_, ()> = Ok(unsafe { (self.function)(case) });
        match result {
            Ok(Ok(out)) => {
                // FUTURE collect stderr with technique
                Ok((out, String::new()))
            }
            Ok(Err(out)) => Err(out),
            Err(_) => Err(format!("panicked!!")),
        }
    }
}
