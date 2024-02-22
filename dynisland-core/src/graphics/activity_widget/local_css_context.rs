use gtk::CssProvider;
use rand::{distributions::Alphanumeric, Rng};

use crate::{graphics::config_variable::ConfigVariable, implement_config_get_set};

use super::imp::ActivityMode;

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedActivityWidgetLocalCssContext")]
pub struct ActivityWidgetLocalCssContext {
    css_provider: CssProvider,
    name: String,

    size: (i32, i32),
    opacity: [f64; 4],
    stretch: [(f64, f64); 4],
    blur: [f64; 4],
    stretch_on_resize: bool,

    config_minimal_height: ConfigVariable<i32>,
    config_blur_radius: ConfigVariable<f64>,
}

#[allow(unused_braces)]
impl ActivityWidgetLocalCssContext {
    pub fn new(name: &str) -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            name: name.to_string(),
            size: (40, 40),
            opacity: [1.0, 0.0, 0.0, 0.0],
            stretch: [(1.0, 1.0), (1.0, 1.0), (1.0, 1.0), (1.0, 1.0)],
            blur: [0.0, 1.0, 1.0, 1.0],
            stretch_on_resize: true,

            config_minimal_height: ConfigVariable::new(40),
            config_blur_radius: ConfigVariable::new(6.0),
        }
    }

    implement_config_get_set!(pub, config_minimal_height, i32);
    implement_config_get_set!(pub, config_blur_radius, f64);

    // GET
    pub fn get_css_provider(&self) -> CssProvider {
        self.css_provider.clone()
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_size(&self) -> (i32, i32) {
        self.size
    }
    pub fn get_opacity(&self, mode: ActivityMode) -> f64 {
        self.opacity[mode as usize]
    }
    pub fn get_stretch(&self, mode: ActivityMode) -> (f64, f64) {
        self.stretch[mode as usize]
    }
    pub fn get_blur(&self, mode: ActivityMode) -> f64 {
        self.blur[mode as usize]
    }
    pub fn get_stretch_on_resize(&self) -> bool {
        self.stretch_on_resize
    }

    // SET
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.update_provider()
    }
    pub fn set_size(&mut self, size: (i32, i32)) {
        if self.size == size {
            return;
        }
        self.size = (
            i32::max(size.0, self.config_minimal_height.value),
            i32::max(size.1, self.config_minimal_height.value),
        );
        self.update_provider()
    }
    pub fn set_opacity(&mut self, mode: ActivityMode, opacity: f64) {
        if self.opacity[mode as usize] == opacity {
            return;
        }
        self.opacity[mode as usize] = opacity;
        self.update_provider()
    }
    pub fn set_opacity_all(&mut self, opacity: [f64; 4]) {
        if self.opacity == opacity {
            return;
        }
        self.opacity = opacity;
        self.update_provider()
    }
    pub fn set_stretch(&mut self, mode: ActivityMode, stretch: (f64, f64)) {
        if self.stretch[mode as usize] == stretch {
            return;
        }
        self.stretch[mode as usize] = stretch;
        self.update_provider()
    }
    pub fn set_stretch_all(&mut self, stretch: [(f64, f64); 4]) {
        if self.stretch == stretch {
            return;
        }
        self.stretch = stretch;
        self.update_provider()
    }
    pub fn set_blur(&mut self, mode: ActivityMode, blur: f64) {
        if self.blur[mode as usize] == blur {
            return;
        }
        self.blur[mode as usize] = blur;
        self.update_provider()
    }
    pub fn set_blur_all(&mut self, blur: [f64; 4]) {
        if self.blur == blur {
            return;
        }
        self.blur = blur;
        self.update_provider()
    }
    pub fn set_stretch_on_resize(&mut self, stretch: bool) {
        if self.stretch_on_resize == stretch {
            return;
        }
        self.stretch_on_resize = stretch;
        self.update_provider()
    }

    fn update_provider(&self) {
        let (w, h) = self.size;
        // let border_radius = self.border_radius;
        let name = self.name.as_str();
        let (min_opacity, com_opacity, exp_opacity, ove_opacity) = (
            self.opacity[0],
            self.opacity[1],
            self.opacity[2],
            self.opacity[3],
        );
        let stretches = self.stretch.map(|(x, y)| {
            let x = if !x.is_finite() { 1.0 } else { x };
            let y = if !y.is_finite() { 1.0 } else { y };
            (x, y)
        });
        let (min_stretch_x, com_stretch_x, exp_stretch_x, ove_stretch_x) = (
            stretches[0].0,
            stretches[1].0,
            stretches[2].0,
            stretches[3].0,
        );
        let (min_stretch_y, com_stretch_y, exp_stretch_y, ove_stretch_y) = (
            stretches[0].1,
            stretches[1].1,
            stretches[2].1,
            stretches[3].1,
        );
        let (min_blur, com_blur, exp_blur, ove_blur) =
            (self.blur[0], self.blur[1], self.blur[2], self.blur[3]);
        // debug!("{size_timing_function}");
        let css = if self.stretch_on_resize {
            format!(
                r".{name} .activity-background, .{name} .activity-background * {{ 
                    min-width: {w}px; 
                    min-height: {h}px;
                }}
                
                .{name} .mode-minimal{{
                    opacity: {min_opacity};
                    transform: scale({min_stretch_x}, {min_stretch_y});
                    filter: blur({min_blur}px);
                }}
                .{name} .mode-compact{{
                    opacity: {com_opacity};
                    transform: scale({com_stretch_x}, {com_stretch_y});
                    filter: blur({com_blur}px);
                }}
                .{name} .mode-expanded{{
                    opacity: {exp_opacity};
                    transform: scale({exp_stretch_x}, {exp_stretch_y});
                    filter: blur({exp_blur}px);
                }}
                .{name} .mode-overlay{{
                    opacity: {ove_opacity};
                    transform: scale({ove_stretch_x}, {ove_stretch_y});
                    filter: blur({ove_blur}px);
                }}"
            )
        } else {
            format!(
                r".{name} .activity-background, .{name} .activity-background * {{ 
                    min-width: {w}px; 
                    min-height: {h}px;
                }}

                .{name} .mode-minimal{{
                    opacity: {min_opacity};
                    filter: blur({min_blur}px);
                    transform: scale(1,1);
                }}
                .{name} .mode-compact{{
                    opacity: {com_opacity};
                    filter: blur({com_blur}px);
                    transform: scale(1,1);
                }}
                .{name} .mode-expanded{{
                    opacity: {exp_opacity};
                    filter: blur({exp_blur}px);
                    transform: scale(1,1);
                }}
                .{name} .mode-overlay{{
                    opacity: {ove_opacity};
                    filter: blur({ove_blur}px);
                    transform: scale(1,1);
                }}"
            )
        };
        // log::debug!("{css}");
        self.css_provider.load_from_string(&css);
    }
}

impl Default for ActivityWidgetLocalCssContext {
    fn default() -> Self {
        Self::new(
            "c".chars()
                .chain(
                    rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(6)
                        .map(char::from),
                )
                .collect::<String>()
                .as_str(),
        )
    }
}
