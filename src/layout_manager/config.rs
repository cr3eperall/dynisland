use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::window_position::{DeWindowPosition, WindowPosition};

pub const DEFAULT_AUTO_MINIMIZE_TIMEOUT: i32 = 5000;

// TODO cleanup

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct FallbackLayoutConfigMain {
    pub(crate) orientation_horizontal: bool,
    pub(crate) window_position: WindowPosition,
    pub(crate) auto_minimize_timeout: i32,
    pub(crate) windows: HashMap<String, FallbackLayoutConfig>,
}

impl Default for FallbackLayoutConfigMain {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("".to_string(), FallbackLayoutConfig::default());
        Self {
            orientation_horizontal: true,
            window_position: WindowPosition::default(),
            auto_minimize_timeout: DEFAULT_AUTO_MINIMIZE_TIMEOUT,
            windows: map,
        }
    }
}
impl FallbackLayoutConfigMain {
    pub fn default_conf(&self) -> FallbackLayoutConfig {
        FallbackLayoutConfig {
            orientation_horizontal: self.orientation_horizontal,
            window_position: self.window_position.clone(),
            auto_minimize_timeout: self.auto_minimize_timeout,
        }
    }
    pub fn get_for_window(&self, window: &str) -> FallbackLayoutConfig {
        match self.windows.get(window) {
            Some(conf) => conf.clone(),
            None => self.default_conf(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct FallbackLayoutConfig {
    pub(crate) orientation_horizontal: bool,
    pub(crate) window_position: WindowPosition,
    pub(crate) auto_minimize_timeout: i32,
}
impl Default for FallbackLayoutConfig {
    fn default() -> Self {
        Self {
            orientation_horizontal: true,
            window_position: WindowPosition::default(),
            auto_minimize_timeout: DEFAULT_AUTO_MINIMIZE_TIMEOUT,
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DeFallbackLayoutConfigMain {
    orientation_horizontal: bool,
    window_position: WindowPosition,
    auto_minimize_timeout: i32,
    windows: HashMap<String, DeFallbackLayoutConfig>,
}

impl Default for DeFallbackLayoutConfigMain {
    fn default() -> Self {
        Self {
            orientation_horizontal: true,
            window_position: WindowPosition::default(),
            auto_minimize_timeout: DEFAULT_AUTO_MINIMIZE_TIMEOUT,
            windows: HashMap::new(),
        }
    }
}
impl DeFallbackLayoutConfigMain {
    pub fn into_main_config(self) -> FallbackLayoutConfigMain {
        let mut windows = HashMap::new();
        for (name, opt_conf) in self.windows {
            let window_pos = match opt_conf.window_position {
                Some(opt_window_pos) => WindowPosition {
                    layer: opt_window_pos
                        .layer
                        .unwrap_or(self.window_position.layer.clone()),
                    h_anchor: opt_window_pos
                        .h_anchor
                        .unwrap_or(self.window_position.h_anchor.clone()),
                    v_anchor: opt_window_pos
                        .v_anchor
                        .unwrap_or(self.window_position.v_anchor.clone()),
                    margin_x: opt_window_pos
                        .margin_x
                        .unwrap_or(self.window_position.margin_x),
                    margin_y: opt_window_pos
                        .margin_y
                        .unwrap_or(self.window_position.margin_y),
                    exclusive_zone: opt_window_pos
                        .exclusive_zone
                        .unwrap_or(self.window_position.exclusive_zone),
                    monitor: opt_window_pos
                        .monitor
                        .unwrap_or(self.window_position.monitor.clone()),
                    layer_shell: opt_window_pos
                        .layer_shell
                        .unwrap_or(self.window_position.layer_shell),
                },
                None => self.window_position.clone(),
            };
            let conf = FallbackLayoutConfig {
                orientation_horizontal: opt_conf
                    .orientation_horizontal
                    .unwrap_or(self.orientation_horizontal),
                window_position: window_pos,
                auto_minimize_timeout: opt_conf
                    .auto_minimize_timeout
                    .unwrap_or(self.auto_minimize_timeout),
            };

            windows.insert(name, conf);
        }
        let mut main_conf = FallbackLayoutConfigMain {
            orientation_horizontal: self.orientation_horizontal,
            window_position: self.window_position,
            auto_minimize_timeout: self.auto_minimize_timeout,
            windows,
        };
        if main_conf.windows.is_empty() {
            let default = main_conf.default_conf();
            main_conf.windows.insert("".to_string(), default);
        }
        main_conf
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DeFallbackLayoutConfig {
    orientation_horizontal: Option<bool>,
    window_position: Option<DeWindowPosition>,
    auto_minimize_timeout: Option<i32>,
}
