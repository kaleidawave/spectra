use std::env;
use std::io::{self, BufRead};

fn main() {
    let mut is_uppercase = false;
    let mut intentional_crash = false;
    let mut use_lists = false;
    let mut interactive = false;
    for arg in env::args().skip(1) {
        if arg == "--uppercase" {
            is_uppercase = true;
        } else if arg == "--interactive" {
            interactive = true;
        } else if arg == "--intentional-crash" {
            intentional_crash = true;
        } else if arg == "--use-lists" {
            use_lists = true;
        }
    }

    if interactive {
        let stdin = io::stdin();
        let mut buf = Vec::new();

        println!("start");

        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };

            if line == "close" {
                if !buf.is_empty() {
                    eprintln!("no end to message {buf:?}");
                }
                break;
            }

            if line == "end" {
                let output = String::from_utf8_lossy(&buf);
                {
                    for line in output.lines() {
                        if intentional_crash && line.trim_end().ends_with("2") {
                            panic!("CRASH!!!");
                        }
                        let output = if is_uppercase {
                            std::borrow::Cow::Owned(line.to_uppercase())
                        } else {
                            std::borrow::Cow::Borrowed(line)
                        };
                        if use_lists {
                            print!("- ");
                        }
                        if line.trim_end().ends_with("on stderr") {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            eprintln!("{output}");
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        } else {
                            println!("{output}");
                        }
                    }
                }
                println!("end");
                buf.clear();
                continue;
            }

            buf.extend_from_slice(line.as_bytes());
            buf.push(b'\n');
        }
    } else {
        let value = env::args().nth(1).unwrap();
        for line in value.lines() {
            let output = if is_uppercase {
                std::borrow::Cow::Owned(line.to_uppercase())
            } else {
                std::borrow::Cow::Borrowed(line)
            };
            if use_lists {
                print!("- ");
            }
            println!("{output}");
        }
    }
}
