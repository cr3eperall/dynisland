use std::collections::HashMap;

use ron::Value;
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "/home/david/.config/dynisland/dynisland.ron";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub module_config: HashMap<String, Value>,
}
impl Default for Config {
    fn default() -> Self {
        let map = HashMap::<String, Value>::new();
        Self { module_config: map }
    }
}

pub fn get_config() -> Config {
    let content = std::fs::read_to_string(CONFIG_FILE).expect("failed to read config file");
    let ron: Config = ron::de::from_str(&content).expect("failed to parse config");
    ron
}
