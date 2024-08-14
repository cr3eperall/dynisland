use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
};

use colored::Colorize;
use log::warn;
use ron::{extensions::Extensions, ser::PrettyConfig, Value};
use serde::{Deserialize, Serialize};

pub const CONFIG_REL_PATH: &str = "dynisland/"; //TODO add cli override

// ron sucks, ~~i need to switch to pkl~~
// nvm, there are no good pkl crates

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub loaded_modules: Vec<String>,
    pub layout: Option<String>,
    pub general_style_config: GeneralConfig,
    pub layout_configs: HashMap<String, Value>,
    pub module_config: HashMap<String, Value>,
    pub debug: Option<DebugConfig>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebugConfig {
    pub runtime_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct GeneralConfig {
    pub minimal_height: u32,
    pub minimal_width: u32,
    pub blur_radius: f64,
    pub enable_drag_stretch: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            minimal_height: 40,
            minimal_width: 60,
            blur_radius: 6.0,
            enable_drag_stretch: false,
            //TODO find a way to add scrolling label to settings
        }
    }
}

// TODO add api for getting default config from modules
impl Default for Config {
    fn default() -> Self {
        let module_map = HashMap::<String, Value>::new();
        let layout_map = HashMap::<String, Value>::new();
        Self {
            module_config: module_map,
            layout_configs: layout_map,
            layout: Some("SimpleLayout".to_string()),
            general_style_config: GeneralConfig::default(),
            loaded_modules: vec!["all".to_string()],
            debug: None,
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let options = ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME);
        let res = options
            .to_string_pretty(self, PrettyConfig::default())
            .unwrap_or("unable to parse config".to_string());
        write!(f, "{}", res)
    }
}

impl Config {
    pub fn get_runtime_dir(&self) -> PathBuf {
        self.debug
            .clone()
            .map(|debug| PathBuf::from(debug.runtime_path))
            .unwrap_or(get_default_runtime_path())
    }
}

pub fn get_default_config_path() -> PathBuf {
    glib::user_config_dir().join(CONFIG_REL_PATH)
}
fn get_default_runtime_path() -> PathBuf {
    glib::user_runtime_dir().join(CONFIG_REL_PATH)
}

pub fn get_config(config_dir: &Path) -> Config {
    let config_path = config_dir.join("dynisland.ron");
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
