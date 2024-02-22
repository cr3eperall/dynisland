use std::{collections::HashMap, path::PathBuf};

use colored::Colorize;
use log::warn;
use ron::{extensions::Extensions, Value};
use serde::{Deserialize, Serialize};

pub const CONFIG_REL_PATH: &str = "dynisland/"; //TODO add cli override

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "Vec::new")]
    pub loaded_modules: Vec<String>,
    pub layout: Option<String>,
    #[serde(default = "GeneralConfig::default")]
    pub general_style_config: GeneralConfig,
    #[serde(default = "HashMap::new")]
    pub layout_configs: HashMap<String, Value>,
    #[serde(default = "HashMap::new")]
    pub module_config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GeneralConfig {
    #[serde(default = "min_height")]
    pub minimal_height: u32,
    #[serde(default = "blur_radius")]
    pub blur_radius: f64,
}

fn min_height() -> u32 {
    40
}
fn blur_radius() -> f64 {
    6.0
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            minimal_height: min_height(),
            blur_radius: blur_radius(),
            //TODO find a way to add scrolling label to settings
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let module_map = HashMap::<String, Value>::new();
        let layout_map = HashMap::<String, Value>::new();
        Self {
            module_config: module_map,
            layout_configs: layout_map,
            layout: None,
            general_style_config: GeneralConfig::default(),
            loaded_modules: vec!["all".to_string()],
        }
    }
}

pub fn get_config_path() -> PathBuf {
    glib::user_config_dir().join(CONFIG_REL_PATH)
}

pub fn get_config() -> Config {
    let config_path = glib::user_config_dir()
        .join(CONFIG_REL_PATH)
        .join("dynisland.ron");
    let content = std::fs::read_to_string(config_path).expect("failed to read config file");
    let options = ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME);

    let ron: Config = options.from_str(&content).unwrap_or_else(|err| {
        warn!(
            "{} {}",
            "failed to parse config, using default. Err:".red(),
            err.to_string().red()
        );
        Config::default()
    });
    ron
}
