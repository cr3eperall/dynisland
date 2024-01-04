use std::{collections::HashMap, path::PathBuf};

use colored::Colorize;
use css_anim::soy::{self, Bezier};
use log::warn;
use ron::Value;
use serde::{Deserialize, Serialize};

pub const CONFIG_REL_PATH: &str = "dynisland/"; //TODO add cli override

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub module_config: HashMap<String, Value>,
    pub general_config: GeneralConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GeneralConfig {
    #[serde(default = "min_height")]
    pub minimal_height: u32,
    #[serde(default = "t_d_default")]
    pub transition_duration: u64,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_s_default"
    )]
    pub transition_size: Bezier, //TODO need to change all of these to EaseFunction and implement some standard parsing
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_bb_default"
    )]
    pub transition_bigger_blur: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_bs_default"
    )]
    pub transition_bigger_stretch: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_bo_default"
    )]
    pub transition_bigger_opacity: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_sb_default"
    )]
    pub transition_smaller_blur: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_ss_default"
    )]
    pub transition_smaller_stretch: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "t_so_default"
    )]
    pub transition_smaller_opacity: Bezier,
}

fn min_height() -> u32 {
    40
}

fn t_d_default() -> u64 {
    1000
}

fn t_s_default() -> Bezier {
    soy::LINEAR
}

fn t_bb_default() -> Bezier {
    soy::EASE_IN
}
fn t_bs_default() -> Bezier {
    soy::EASE_OUT
}
fn t_bo_default() -> Bezier {
    soy::cubic_bezier(0.2, 0.55, 0.15, 1.0)
}
fn t_sb_default() -> Bezier {
    soy::EASE_IN
}
fn t_ss_default() -> Bezier {
    soy::EASE_OUT
}
fn t_so_default() -> Bezier {
    soy::cubic_bezier(0.2, 0.55, 0.15, 1.0)
}

impl Default for Config {
    fn default() -> Self {
        let map = HashMap::<String, Value>::new();
        Self {
            module_config: map,
            general_config: GeneralConfig {
                minimal_height: min_height(),
                transition_duration: t_d_default(),
                transition_size: t_s_default(),
                transition_bigger_blur: t_bb_default(),
                transition_bigger_stretch: t_bs_default(),
                transition_bigger_opacity: t_bo_default(),
                transition_smaller_blur: t_sb_default(),
                transition_smaller_stretch: t_ss_default(),
                transition_smaller_opacity: t_so_default(),
                //TODO find a way to add scrolling label to settings
            },
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
    let ron: Config = ron::de::from_str(&content).unwrap_or_else(|err| {
        warn!(
            "{} {}",
            "failed to parse config, using default. Err:".red(),
            err.to_string().red()
        );
        Config::default()
    });
    ron
}
