use std::collections::HashMap;

use dynisland_core::graphics::animations::soy::{Bezier, self};
use ron::Value;
use serde::{Deserialize, Serialize};
use colored::Colorize;

pub const CONFIG_FILE: &str = "/home/david/.config/dynisland/dynisland.ron"; //TODO add cli override

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub module_config: HashMap<String, Value>,
    pub general_config: GeneralConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GeneralConfig {
    #[serde(default="t_d_default")]
    pub transition_duration: u64,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_s_default")]
    pub transition_size: Bezier,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_bb_default")]
    pub transition_bigger_blur: Bezier,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_bs_default")]
    pub transition_bigger_stretch: Bezier,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_bo_default")]
    pub transition_bigger_opacity: Bezier,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_sb_default")]
    pub transition_smaller_blur: Bezier,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_ss_default")]
    pub transition_smaller_stretch: Bezier,
    #[serde(deserialize_with = "Bezier::from_string_or_struct",default="t_so_default")]
    pub transition_smaller_opacity: Bezier,
}

fn t_d_default()->u64{
    1000
}

fn t_s_default()-> Bezier{
    soy::LINEAR
}

fn t_bb_default()->Bezier{
    soy::EASE_IN
}
fn t_bs_default()->Bezier{
    soy::EASE_OUT
}
fn t_bo_default()->Bezier{
    soy::cubic_bezier(0.2, 0.55, 0.15, 1.0)
}
fn t_sb_default()->Bezier{
    soy::EASE_IN
}
fn t_ss_default()->Bezier{
    soy::EASE_OUT
}
fn t_so_default()->Bezier{
    soy::cubic_bezier(0.2, 0.55, 0.15, 1.0)
}

impl Default for Config {
    fn default() -> Self {
        let map = HashMap::<String, Value>::new();
        Self {
            module_config: map,
            general_config: GeneralConfig {
                transition_duration: t_d_default(),
                transition_size: t_s_default(),
                transition_bigger_blur: t_bb_default(),
                transition_bigger_stretch: t_bs_default(),
                transition_bigger_opacity: t_bo_default(),
                transition_smaller_blur: t_sb_default(),
                transition_smaller_stretch: t_ss_default(),
                transition_smaller_opacity: t_so_default(),
                //TODO find a way to add scrolling label defaults
            },
        }
    }
}

pub fn get_config() -> Config {
    let content = std::fs::read_to_string(CONFIG_FILE).expect("failed to read config file");
    let ron: Config = ron::de::from_str(&content).unwrap_or_else(|err|{
        println!("{} {}","failed to parse config:".red(), err.to_string().red());
        Config::default()
    });
    ron
}
