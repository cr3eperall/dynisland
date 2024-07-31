use std::{collections::HashMap, path::PathBuf};

use colored::Colorize;
use log::warn;
use ron::{extensions::Extensions, Value};
use serde::{Deserialize, Serialize};

pub const CONFIG_REL_PATH: &str = "dynisland/"; //TODO add cli override

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub loaded_modules: Vec<String>,
    pub layout: Option<String>,
    pub general_style_config: GeneralConfig,
    pub layout_configs: HashMap<String, Value>,
    pub module_config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct GeneralConfig {
    pub minimal_height: u32,
    pub minimal_width: u32,
    pub blur_radius: f64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            minimal_height: 40,
            minimal_width: 60,
            blur_radius: 6.0,
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
    let content = std::fs::read_to_string(config_path);
    let options = ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME);

    let ron: Config = match content {
        Ok(content) => options.from_str(&content).unwrap_or_else(|err| {
            warn!(
                "{} {}",
                "failed to parse config, using default. Err:".red(),
                err.to_string().red()
            );
            Config::default()
        }),
        Err(err) => {
            log::warn!("failed to parse config file, using default: {err}");
            Config::default()
        }
    };
    ron
}
