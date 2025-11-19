// TODO this may have problems with FFI

use crate::{Runner, Test};

type FunctionType = unsafe extern "Rust" fn(&str) -> Result<String, String>;

pub struct Rust {
    /// we need to hold this so that the function is valid
    _library: libloading::Library,
    /// function callback
    function: libloading::Symbol<'static, FunctionType>,
}

fn get_output_name_from_json(json_output: &str) -> Option<&str> {
    let prefix = "\"filenames\":[\"";
    let start: usize = json_output.find(prefix)? + prefix.len();
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
    pub fn new(path: String, name: String) -> Result<Self, String> {
        let output = std::process::Command::new("cargo")
            .arg("rustc")
            .arg("--crate-type")
            .arg("cdylib")
            .arg("--message-format")
            .arg("json")
            .current_dir(&path)
            .output();
        let Ok(output) = output else {
            return Err("could not build library".to_owned());
        };
        let out_json = str::from_utf8(&output.stdout).unwrap();
        let Some(artifact_name) = get_output_name_from_json(out_json) else {
            return Err(format!("JSON does not contain artifact: {out_json}"));
        };
        unsafe {
            let Ok(_library) = libloading::Library::new(artifact_name) else {
                return Err("library does not exist".to_owned());
            };
            let Ok(function): Result<libloading::Symbol<'_, FunctionType>, _> =
                _library.get(name.as_bytes())
            else {
                return Err(format!("library does not have export {name}"));
            };
            // because library is owned this is fine?
            let function = std::mem::transmute(function);
            Ok(Self { _library, function })
        }
    }
}

impl Runner for Rust {
    fn run(&mut self, test: &Test) -> Result<(String, String), String> {
        let out = unsafe { (self.function)(&test.case) };
        match out {
            Ok(out) => {
                // TODO collect stderr with technique
                Ok((out, String::new()))
            }
            Err(out) => Err(out),
        }
    }

    fn close(self) {}
}
