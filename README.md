Run tests in markdown files. It is designed for writing bulk tests with similar functionality on different inputs. Using markdown is simple to write and easy to view so your tests act as documentation.

Given

````markdown
### *test name*

*any description stuff*

```
*input*
```

```
*the expected output*
```
````

You can run the test with

```shell
cargo run -- compare ./examples/specification.md \
	"./target/debug/examples/example_stdin_stdout_program --uppercase --rpc,./target/debug/examples/example_stdin_stdout_program --rpc" \
	--only other
```

### Features

- `test` runs tests
- `--only` and `--skip`
- `compare` for running multiple binaries

### Notes

> These are work-in-progress

- There is a runner that points to a specific binary as well as a communication via stdin-stdout output (see `examples/example_stdin_stdout_program.rs`)
	- Not only does the *rpc* runner allow
- Output formatting is designed to mirror that of the default Rust test harness
- Test names can contain any characters (👀 Rust)
- This is also enabled as a crate to directly use in the project

### TODO

- Complete the default runner addition (currently skipped)
- Option to emit results as markdown and JSON (also output file)
- Parse and send options along with input
