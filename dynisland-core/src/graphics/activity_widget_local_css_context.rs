use anyhow::{Context, Result};
use gtk::{prelude::CssProviderExt, CssProvider};
use rand::{distributions::Alphanumeric, Rng};

use super::activity_widget::MINIMAL_HEIGHT;

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedActivityWidgetLocalCssContext")]
pub struct ActivityWidgetLocalCssContext {
    css_provider: CssProvider,
    name: String,

    size: (i32, i32),
    stretch_on_resize: bool,
    border_radius: i32,
    transition_duration: u64,
    transition_duration_set_by_module: bool,
}

impl ActivityWidgetLocalCssContext {
    pub fn new(name: &str) -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            name: name.to_string(),
            size: (MINIMAL_HEIGHT, MINIMAL_HEIGHT),
            stretch_on_resize: true,
            border_radius: 100,
            transition_duration: 0,
            transition_duration_set_by_module: false,
        }
    }

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
    pub fn get_transition_duration(&self) -> u64 {
        self.transition_duration
    }
    pub fn get_transition_duration_set_by_module(&self) -> bool {
        self.transition_duration_set_by_module
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
    pub fn set_transition_duration(
        // if the duration is set by the module it uses that, otherwise it uses the one in the comfig file
        &mut self,
        transition_duration: u64,
        module: bool,
    ) -> Result<()> {
        if self.transition_duration == transition_duration {
            return Ok(());
        }
        if module {
            self.transition_duration_set_by_module = true;
        } else if self.transition_duration_set_by_module {
            return Ok(());
        }
        self.transition_duration = transition_duration;
        self.update_provider()
    }

    fn update_provider(&self) -> Result<()> {
        let (w, h) = self.size;
        let border_radius = self.border_radius;
        let name = self.name.as_str();
        let transition_duration = self.transition_duration;
        let css = if self.stretch_on_resize {
            format!(
                r".{name} .activity-background{{ 
                    min-width: {w}px; 
                    min-height: {h}px; 
                    transition-property: min-width, min-height;
                    transition-duration: {transition_duration}ms;
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
                r".{name} .activity-background{{ 
                    min-width: {w}px; 
                    min-height: {h}px; 
                    transition-property: min-width, min-height;
                    transition-duration: {transition_duration}ms;
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
        // println!("{css}");
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