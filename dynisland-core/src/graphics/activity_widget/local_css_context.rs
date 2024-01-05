use anyhow::{Context, Result};
use css_anim::{ease_functions::LinearEaseFunction, soy::EaseFunction};
use gtk::{prelude::CssProviderExt, CssProvider};
use rand::{distributions::Alphanumeric, Rng};

use crate::{graphics::config_variable::ConfigVariable, implement_get_set};

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedActivityWidgetLocalCssContext")]
pub struct ActivityWidgetLocalCssContext {
    css_provider: CssProvider,
    name: String,

    size: (i32, i32),
    stretch_on_resize: bool,
    border_radius: i32,

    minimal_height: ConfigVariable<i32>,
    transition_duration: ConfigVariable<u64>,

    transition_size: ConfigVariable<Box<dyn EaseFunction>>,

    transition_bigger_blur: ConfigVariable<Box<dyn EaseFunction>>,
    transition_smaller_blur: ConfigVariable<Box<dyn EaseFunction>>,
    transition_bigger_stretch: ConfigVariable<Box<dyn EaseFunction>>,
    transition_smaller_stretch: ConfigVariable<Box<dyn EaseFunction>>,
    transition_bigger_opacity: ConfigVariable<Box<dyn EaseFunction>>,
    transition_smaller_opacity: ConfigVariable<Box<dyn EaseFunction>>,
}

#[allow(unused_braces)]
impl ActivityWidgetLocalCssContext {
    pub fn new(name: &str) -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            name: name.to_string(),
            minimal_height: ConfigVariable::new(40),
            size: (40, 40),
            stretch_on_resize: true,
            border_radius: 100,
            transition_duration: ConfigVariable::new(0),
            transition_size: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
            transition_bigger_blur: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
            transition_smaller_blur: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
            transition_bigger_stretch: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
            transition_smaller_stretch: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
            transition_bigger_opacity: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
            transition_smaller_opacity: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
        }
    }

    implement_get_set!(pub, minimal_height, i32);
    implement_get_set!(pub, transition_duration, u64, self => {self.update_provider()});
    implement_get_set!(pub, transition_size, Box<dyn EaseFunction>, self => {self.update_provider()});
    implement_get_set!(pub, transition_bigger_blur, Box<dyn EaseFunction>);
    implement_get_set!(pub, transition_smaller_blur, Box<dyn EaseFunction>);
    implement_get_set!(pub, transition_bigger_stretch, Box<dyn EaseFunction>);
    implement_get_set!(pub, transition_smaller_stretch, Box<dyn EaseFunction>);
    implement_get_set!(pub, transition_bigger_opacity, Box<dyn EaseFunction>);
    implement_get_set!(pub, transition_smaller_opacity, Box<dyn EaseFunction>);

    pub fn get_css_provider(&self) -> CssProvider {
        self.css_provider.clone()
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_size(&self) -> (i32, i32) {
        self.size
    }
    pub fn get_stretch_on_resize(&self) -> bool {
        self.stretch_on_resize
    }
    pub fn get_border_radius(&self) -> i32 {
        self.border_radius
    }

    pub fn set_name(&mut self, name: &str) -> Result<()> {
        self.name = name.to_string();
        self.update_provider()
    }
    pub fn set_size(&mut self, size: (i32, i32)) -> Result<()> {
        if self.size == size {
            return Ok(());
        }
        self.size = size;
        self.update_provider()
    }
    pub fn set_stretch_on_resize(&mut self, stretch: bool) -> Result<()> {
        if self.stretch_on_resize == stretch {
            return Ok(());
        }
        self.stretch_on_resize = stretch;
        self.update_provider()
    }
    pub fn set_border_radius(&mut self, border_radius: i32) -> Result<()> {
        if self.border_radius == border_radius {
            return Ok(());
        }
        self.border_radius = border_radius;
        self.update_provider()
    }
    fn update_provider(&self) -> Result<()> {
        let (w, h) = self.size;
        let border_radius = self.border_radius;
        let name = self.name.as_str();
        let transition_duration = self.get_transition_duration();
        let size_timing_function = self.get_transition_size().to_string();
        // debug!("{size_timing_function}");
        let css = if self.stretch_on_resize {
            format!(
                r".{name} .activity-background, .{name} .activity-background * {{ 
                    min-width: {w}px; 
                    min-height: {h}px; 
                    transition-property: min-width, min-height;
                    transition-duration: {transition_duration}ms;
                    transition-timing-function: {size_timing_function};
                }}" // .{name} .mode-compact{{
                    //     border-radius: {border_radius}px;
                    // }}
                    // .{name} .mode-minimal{{
                    //     border-radius: {border_radius}px;
                    // }}
                    // .{name} .mode-expanded{{
                    //     border-radius: {border_radius}px;
                    // }}
                    // .{name} .mode-overlay{{
                    //     border-radius: {border_radius}px;
                    // }}"
            )
        } else {
            format!(
                r".{name} .activity-background, .{name} .activity-background * {{ 
                    min-width: {w}px; 
                    min-height: {h}px; 
                    transition-property: min-width, min-height;
                    transition-duration: {transition_duration}ms;
                    transition-timing-function: {size_timing_function};
                }}
                .{name} .mode-compact{{
                    border-radius: {border_radius}px;
                }}
                .{name} .mode-minimal{{
                    border-radius: {border_radius}px;
                }}
                .{name} .mode-expanded{{
                    border-radius: {border_radius}px;
                }}
                .{name} .mode-overlay{{
                    border-radius: {border_radius}px;
                }}"
            )
        };
        // trace!("{css}");
        self.css_provider
            .load_from_data(css.as_bytes())
            .with_context(|| "failed to update css provider data")
    }
}

impl Default for ActivityWidgetLocalCssContext {
    fn default() -> Self {
        Self::new(
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(6)
                .map(char::from)
                .collect::<String>()
                .as_str(),
        )
    }
}
