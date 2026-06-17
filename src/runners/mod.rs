pub mod compiled;
pub mod program;
pub mod shell;

pub trait Runner {
    /// Returns `Ok((*stdout*, *stderr*))`
    ///
    /// # Errors
    /// if test failed on runner, return a `Err` with some message about why it failed
    fn run(&mut self, test: &super::Test) -> Result<(String, String), String>;

    /// Cleanup
    fn close(&mut self) {}
}

// rust:
// stdin-stdout:
// shell:

#[derive(Default)]
pub struct Runners {
    others: std::collections::HashMap<String, Box<dyn Runner>>,
}

impl Runners {
    pub fn new(default: Box<dyn Runner>) -> Self {
        Self {
            others: std::collections::HashMap::from_iter([(String::default(), default)]),
        }
    }

    pub fn add(&mut self, other: String, runner: Box<dyn Runner>) {
        self.others.insert(other, runner);
    }

    // TODO better
    pub fn get(&mut self, name: &str) -> Option<&mut Box<dyn Runner>> {
        self.others.get_mut(name)
    }
}
