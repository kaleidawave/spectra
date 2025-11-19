use crate::utilities::commands;
use crate::{Runner, Test};

use std::io::Write;
use std::{process, time};

// #[derive(Debug, Default)]
// pub struct CommandConfiguration {
//     pub stdin_stdout_communication: bool,
//     pub ignore_exit_code: bool,
// }

pub struct Running {
    stdin: process::ChildStdin,
    process: commands::Process,
}

pub struct Command {
    name: String,
    arguments: Vec<String>,
    _ignore_exit_code: bool,
    timeout: Option<time::Duration>,
    currently_running: Option<Running>,
}

impl Command {
    /// # Panics
    ///
    /// panics if `data` is empty
    #[must_use]
    pub fn new(argument: &str) -> Self {
        let mut iter = crate::utilities::ArgumentIter::new(argument);
        let name = iter.next().expect("no command name");
        let mut arguments: Vec<String> = iter.map(std::borrow::Cow::into_owned).collect();

        let mut stdin_stdout_communication = false;
        let mut _ignore_exit_code = false;
        let mut timeout = None;

        if let Some(idx) = arguments
            .iter()
            .position(|arg| matches!(arg.as_str(), "--stdin-stdout-communication" | "--rpc"))
        {
            arguments.remove(idx);
            stdin_stdout_communication = true;
        }

        if let Some(idx) = arguments
            .iter()
            .position(|arg| matches!(arg.as_str(), "--ignore-exit-code"))
        {
            arguments.remove(idx);
            _ignore_exit_code = true;
        }

        if let Some(idx) = arguments
            .iter()
            .position(|arg| matches!(arg.as_str(), "--timeout"))
        {
            // .expect("no timeout")
            let time: u64 = arguments
                .remove(idx + 1)
                .parse()
                .expect("expected millisecond");
            arguments.remove(idx);
            timeout = Some(time::Duration::from_millis(time));
        }

        let name = name.into_owned();
        let mut this = Self {
            name,
            arguments,
            _ignore_exit_code,
            currently_running: None,
            timeout,
        };

        // TODO bad
        if stdin_stdout_communication {
            let running = this.spawn();
            this.currently_running = Some(running);
        }
        this
    }

    pub(crate) fn spawn(&self) -> Running {
        let mut command = process::Command::new(&self.name);
        command.args(&self.arguments);

        command.stdin(process::Stdio::piped());

        let mut process = commands::Process::spawn(command).expect("could not spawn command");

        let child = process.get_child_mut();
        let stdin = child.stdin.take().expect("Failed to open stdin");

        if let Ok(Some(status)) = child.try_wait() {
            panic!("exited with: {status}");
        }

        // TODO duration temp
        let (prelude, result) = process.read_timeout(time::Duration::from_secs(10), Some("start"));

        // Any prelude messages
        for (channel, line) in prelude {
            println!("prelude over: {line} ({channel:?})");
        }

        assert_eq!(
            result.unwrap(),
            commands::ProcessStatus::Continuing,
            "process exected or timed-out"
        );

        Running { stdin, process }
    }
}

impl Runner for Command {
    fn run(&mut self, test: &Test) -> Result<(String, String), String> {
        if let Some(ref mut running) = self.currently_running {
            for line in test.case.as_str().lines() {
                // eprintln!("TEMP writing {line:?}");
                writeln!(running.stdin, "{line}").expect("could not write (early crash)");
            }

            writeln!(running.stdin, "end").expect("could not write (early crash)");

            let timeout = self.timeout.unwrap_or(time::Duration::MAX);
            let (messages, res) = running.process.read_timeout(timeout, Some("end"));

            // TODO?
            // let is_err = messages
            //     .last()
            //     .is_some_and(|(_, line)| line.starts_with("error: "));

            let mut timed_out = false;

            let command_no_longer_running: bool = match res {
                Ok(status) => status == commands::ProcessStatus::Finished,
                Err(err) => {
                    timed_out = err.kind() == std::io::ErrorKind::TimedOut;
                    if timed_out {
                        let _ = running.process.get_child_mut().kill();
                    }
                    timed_out
                }
            };

            if command_no_longer_running {
                // eprintln!("restarting after timeout or crash");
                let running = self.spawn();
                let _ = self.currently_running.insert(running);
            }

            let (stdout, stderr) = {
                use std::fmt::Write;

                let mut stdout = String::new();
                let mut stderr = String::new();

                if timed_out {
                    writeln!(&mut stderr, "PROCESS TIMED OUT").unwrap();
                }

                for (channel, message) in messages {
                    match channel {
                        commands::Channel::Stdout => {
                            writeln!(&mut stdout, "{message}").unwrap();
                        }
                        commands::Channel::Stderr => {
                            // TODO Hmm
                            if test.merge_stderr {
                                writeln!(&mut stdout, "[{message}]").unwrap();
                            } else if command_no_longer_running {
                                writeln!(&mut stdout, "* {message}").unwrap();
                            } else {
                                writeln!(&mut stderr, "{message}").unwrap();
                            }
                        }
                    }
                }

                stdout.truncate(stdout.trim_end().len());
                stderr.truncate(stderr.trim_end().len());

                (stdout, stderr)
            };

            if command_no_longer_running {
                Err(stderr)
            } else {
                Ok((stdout, stderr))
            }
        } else {
            let arguments = self.arguments.iter().map(|argument| {
                let argument = argument.as_str();
                if let "{content}" = argument {
                    // TODO should this be part of the markdown parser
                    test.case.as_str().trim_end()
                } else if let "{file}" = argument {
                    todo!("create file")
                } else {
                    argument
                }
            });

            let mut command = process::Command::new(&self.name);
            command.args(arguments);

            let command = commands::Process::spawn(command).unwrap();
            let timeout = self.timeout.unwrap_or(time::Duration::MAX);
            let (messages, res) = command.read_timeout(timeout, None);

            // TODO WIP
            // let is_err = messages
            //     .last()
            //     .is_some_and(|(_, line)| line.starts_with("error: "));

            match res {
                Ok(_) => {
                    if test.expected.is_none() && !messages.is_empty() {
                        eprintln!(
                            "Possibly unexpected stdout output {messages:?} from {name}",
                            name = test.name
                        );
                    }

                    let mut stdout = String::new();
                    let mut stderr = String::new();

                    for (channel, message) in messages {
                        use std::fmt::Write;

                        match channel {
                            commands::Channel::Stdout => {
                                writeln!(&mut stdout, "{message}").unwrap();
                            }
                            commands::Channel::Stderr => {
                                writeln!(&mut stderr, "{message}").unwrap();
                            }
                        }
                    }

                    stdout.truncate(stdout.trim_end().len());
                    stderr.truncate(stderr.trim_end().len());

                    Ok((stdout, stderr))
                }
                Err(err) => Err(format!("Command failed with {err:?}\n{messages:?}")),
            }
        }
    }

    fn close(self) {
        if let Some(Running { mut stdin, process }) = self.currently_running {
            // Send the close signal
            writeln!(stdin, "close").unwrap();

            // TODO other fields here
            let timeout = self.timeout.unwrap_or(time::Duration::MAX);
            let (rest, _) = process.read_timeout(timeout, None);
            for (channel, line) in rest {
                println!("left over: {line} ({channel:?})");
            }

            process.end().unwrap();
        }
    }
}

pub type Identifier = usize;

pub struct Timeout {
    pub sender: std::sync::mpsc::Sender<Identifier>,
    pub thread: std::thread::JoinHandle<()>,
}

impl Timeout {
    pub fn new<T>(duration: std::time::Duration, mut timeout_cb: T) -> Self
    where
        T: FnMut() + std::marker::Send + 'static,
    {
        let (sender, reciever) = std::sync::mpsc::channel::<Identifier>();
        let thread = std::thread::spawn(move || {
            loop {
                let expecting = reciever.recv().unwrap();

                let out = reciever.recv_timeout(duration);
                match out {
                    Ok(item) => {
                        if item == 0 {
                            break;
                        }
                        if expecting != item {
                            eprintln!("not matched {expecting} {item}");
                        }
                    }
                    Err(_timeout) => {
                        timeout_cb();
                    }
                }
            }
        });
        Self { sender, thread }
    }
}

pub struct Commands {
    commands: Vec<(String, Command)>,
}

impl Commands {
    #[must_use]
    pub fn new(data: &str) -> Self {
        let items = data.split(',');
        let commands = items
            .map(|item| (item.to_owned(), Command::new(item)))
            .collect();
        Self { commands }
    }
}

impl Runner for Commands {
    fn run(&mut self, test: &Test) -> Result<(String, String), String> {
        let mut buf = String::new();
        for (name, command) in &mut self.commands {
            let (out, _debug) = command.run(test)?;
            buf.push_str(name);
            buf.push_str(":\n");
            buf.push_str(&out);
            buf.push('\n');
        }
        Ok((buf, String::new()))
    }

    fn close(self) {
        self.commands
            .into_iter()
            .for_each(|(_, command)| command.close());
    }
}
