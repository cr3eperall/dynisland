use std::fmt::Display;

use crate::module::ActivityIdentifier;

impl ActivityIdentifier {
    pub fn new(module_name: &str, activity_name: &str) -> Self {
        Self {
            module: module_name.to_string().into(),
            activity: activity_name.to_string().into(),
        }
    }
    pub fn module(&self) -> String {
        self.module.clone().into()
    }

    pub fn activity(&self) -> String {
        self.activity.clone().into()
    }
}

impl Display for ActivityIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.activity, self.module)
    }
}
