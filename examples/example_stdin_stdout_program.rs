use std::env;
use std::io::{self, BufRead};

fn main() {
    let is_uppercase = env::args().any(|flag| flag.as_str() == "--uppercase");
    let intentional_crash = env::args().any(|flag| flag.as_str() == "--intentional-crash");
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
            for line in output.lines() {
                if intentional_crash && line.trim_end().ends_with("2") {
                    panic!("CRASH!!!");
                }
                let output = if is_uppercase {
                    std::borrow::Cow::Owned(line.to_uppercase())
                } else {
                    std::borrow::Cow::Borrowed(line)
                };
                if line.trim_end().ends_with("on stderr") {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    eprintln!("{output}");
                    std::thread::sleep(std::time::Duration::from_millis(50));
                } else {
                    println!("{output}");
                }
            }
            println!("end");
            buf.clear();
            continue;
        }

        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }

    // println!("Finished!");
}
