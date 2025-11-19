use std::borrow::Cow;
use std::io;
use std::path::Path;

#[must_use]
pub fn is_equal_ignore_new_line_sequence(lhs: &str, rhs: &str) -> bool {
    // We should not care about trailing new lines here...
    let mut lhs = lhs.lines();
    let mut rhs = rhs.lines();
    loop {
        match (lhs.next(), rhs.next()) {
            (Some(lhs), Some(rhs)) => {
                if lhs != rhs {
                    return false;
                }
            }
            (Some(_), _) | (_, Some(_)) => {
                return false;
            }
            (None, None) => {
                return true;
            }
        }
    }
}

pub mod filter {
    pub trait Filter {
        fn should_skip(&self, s: &str) -> bool;
    }

    #[derive(Debug, Clone)]
    pub struct GlobPattern {
        pub matcher: glob::Pattern,
        pub positive: bool,
        pub case_sensitive: bool,
    }

    impl Filter for GlobPattern {
        fn should_skip(&self, name: &str) -> bool {
            let options = glob::MatchOptions {
                case_sensitive: self.case_sensitive,
                ..glob::MatchOptions::default()
            };
            let result = self.matcher.matches_with(name, options);

            if self.positive { !result } else { result }
        }
    }
}

pub mod commands {
    use std::io::{self, BufRead, BufReader};
    use std::{process, sync, thread, time};

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub enum Channel {
        Stdout,
        Stderr,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum ProcessNotification {
        Message(Channel, String),
        Completed, // (ExitStatus),
    }

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub enum ProcessStatus {
        Finished,
        Continuing,
    }

    pub struct Process {
        child: process::Child,
        stdout_handle: thread::JoinHandle<()>,
        stderr_handle: thread::JoinHandle<()>,
        receiver: sync::mpsc::Receiver<ProcessNotification>,
    }

    impl Process {
        pub fn spawn(mut command: process::Command) -> io::Result<Self> {
            let mut child = command
                .stdout(process::Stdio::piped())
                .stderr(process::Stdio::piped())
                .spawn()?;

            let stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
            let stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));

            let (sender, receiver) = sync::mpsc::sync_channel::<ProcessNotification>(0);

            // Thread to read `stdout`
            let sender_stdout = sender.clone();
            let stdout_handle = thread::spawn(move || {
                for line in stdout.lines().map_while(Result::ok) {
                    // TODO `expect` here
                    let _ = sender_stdout.send(ProcessNotification::Message(Channel::Stdout, line));
                }

                // TODO `expect` here
                let _ = sender_stdout.send(ProcessNotification::Completed);
            });

            // Thread to read `stderr`
            let sender_stderr = sender; // .clone();
            let stderr_handle = thread::spawn(move || {
                for line in stderr.lines().map_while(Result::ok) {
                    // TODO `expect` here
                    sender_stderr
                        .send(ProcessNotification::Message(Channel::Stderr, line))
                        .expect("Failed to send stderr");
                }
            });

            Ok(Self {
                child,
                stdout_handle,
                stderr_handle,
                receiver,
            })
        }

        pub fn read_timeout(
            &self,
            timeout: time::Duration,
            end_message: Option<&str>,
        ) -> (Vec<(Channel, String)>, io::Result<ProcessStatus>) {
            let mut messages = Vec::new();
            loop {
                let out = self.receiver.recv_timeout(timeout);
                match out {
                    Ok(item) => match item {
                        ProcessNotification::Message(channel, message) => {
                            // TODO channel
                            if end_message.is_some_and(|expected| expected == message) {
                                break;
                            }
                            messages.push((channel, message));
                        }
                        ProcessNotification::Completed => {
                            return (messages, Ok(ProcessStatus::Finished));
                        }
                    },
                    Err(_timeout) => {
                        let result = Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "timed out reading notification",
                        ));
                        return (messages, result);
                    }
                }
            }

            (messages, Ok(ProcessStatus::Continuing))
        }

        pub fn end(mut self) -> io::Result<process::ExitStatus> {
            let status = self.child.wait()?;
            self.stdout_handle.join().unwrap();
            self.stderr_handle.join().unwrap();
            Ok(status)
        }

        pub fn get_child_mut(&mut self) -> &mut process::Child {
            &mut self.child
        }

        pub fn is_running(&mut self) -> bool {
            if let Ok(status) = self.child.try_wait() {
                status.is_none()
            } else {
                // hmm
                true
            }
        }
    }
}

pub fn visit_specification_files(path: &Path, cb: &mut dyn FnMut(&Path)) -> io::Result<()> {
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_specification_files(&path, cb)?;
            } else {
                cb(&path);
            }
        }
    } else if path.is_file() {
        let skip = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !matches!(ext, "md" | "mdspec"));
        if !skip {
            cb(path);
        }
    } else if path.is_symlink() {
        let path = std::fs::read_link(path)?;
        visit_specification_files(&path, cb)?;
    }
    Ok(())
}

pub fn run_in_alternative_display<T: Sized>(cb: impl FnOnce() -> T) -> T {
    // Clear, ClearType
    use crossterm::{
        cursor::MoveToRow,
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    };

    execute!(std::io::stdout(), EnterAlternateScreen, MoveToRow(0)).unwrap();
    let result = cb();
    // Clear(ClearType::All)
    execute!(std::io::stdout(), LeaveAlternateScreen).unwrap();

    result
}

pub struct ArgumentIter<'a> {
    on: &'a str,
    last: usize,
}

impl<'a> ArgumentIter<'a> {
    #[must_use]
    pub fn new(on: &'a str) -> Self {
        // Trim?
        Self { on, last: 0 }
    }
}

impl<'a> Iterator for ArgumentIter<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.last;
        if let Some((idx, matched)) = self.on[self.last..].match_indices(&[' ', '\'', '"']).next() {
            match matched {
                " " => {
                    let end = self.last + idx;
                    self.last += idx + matched.len();
                    Some(Cow::Borrowed(self.on[start..end].trim()))
                }
                "\"" | "\'" => {
                    let rest = &self.on[self.last..][1..];
                    let (idx2, _) = rest
                        .match_indices(matched)
                        .find(|(idx, _)| !rest[..*idx].ends_with('\\'))
                        .expect("no end to quoted item");

                    self.last += idx + idx2 + 2;
                    if let Some(rest) = self.on.get(self.last..) {
                        self.last += rest.len() - rest.trim_start().len();
                    }
                    let content = &rest[..idx2];
                    if content.contains('\\') {
                        Some(Cow::Owned(content.replace('\\', "")))
                    } else {
                        Some(Cow::Borrowed(content))
                    }
                }
                item => unreachable!("{item}"),
            }
        } else if start < self.on.len() {
            self.last = self.on.len();
            Some(Cow::Borrowed(&self.on[start..]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        let on = "this is a test! 'with' \"things in quotes\" see";
        assert_eq!(
            ArgumentIter::new(on).collect::<Vec<_>>(),
            vec![
                "this",
                "is",
                "a",
                "test!",
                "with",
                "things in quotes",
                "see"
            ]
        );
    }

    #[test]
    fn escaping() {
        let on = "testing 'escaping \\'' \"with \\\" quote\"";
        assert_eq!(
            ArgumentIter::new(on).collect::<Vec<_>>(),
            vec!["testing", "escaping '", "with \" quote"]
        );
    }
}
