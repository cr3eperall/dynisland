use std::fmt::Display;

use crate::base_module::ActivityIdentifier;

impl ActivityIdentifier {
    pub fn new(module_name: &str, activity_name: &str) -> Self {
        Self {
            module: module_name.to_string(),
            activity: activity_name.to_string(),
        }
    }
    pub fn module(&self) -> String {
        self.module.clone()
    }
    pub fn activity(&self) -> String {
        self.activity.clone()
    }
}

impl Display for ActivityIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.module, self.activity)
    }
}
