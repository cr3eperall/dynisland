use std::collections::HashMap;

use dynisland_core::{graphics::activity_widget::ActivityWidget, module_abi::ActivityIdentifier};
use gtk::prelude::*;
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Context, Result};

use super::layout_manager_base::{LayoutDefinition, LayoutManager, LAYOUTS};

pub const NAME: &str = "SimpleLayout";

#[distributed_slice(LAYOUTS)]
static SIMPLE_LAYOUT: LayoutDefinition = (NAME, SimpleLayout::new);

pub struct SimpleLayout {
    widget_map: HashMap<ActivityIdentifier, ActivityWidget>,
    container: gtk::Box,
    focused: Option<ActivityIdentifier>,
    config: SimpleLayoutConfig,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "")]
pub enum Alignment {
    Start,
    Center,
    End,
}

impl Alignment {
    pub fn into_gtk(&self) -> gtk::Align {
        match self {
            Alignment::Start => gtk::Align::Start,
            Alignment::Center => gtk::Align::Center,
            Alignment::End => gtk::Align::End,
        }
    }
}

fn bool_true() -> bool {
    true
}
fn align_start() -> Alignment {
    Alignment::Start
}
fn align_center() -> Alignment {
    Alignment::Center
}
#[allow(dead_code)]
fn align_end() -> Alignment {
    Alignment::End
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimpleLayoutConfig {
    #[serde(default = "bool_true")]
    orientation_horizontal: bool,
    #[serde(default = "align_center")]
    halign: Alignment,
    #[serde(default = "align_start")]
    valign: Alignment,
    #[serde(default = "align_start")]
    child_align: Alignment,
}
impl Default for SimpleLayoutConfig {
    fn default() -> Self {
        Self {
            orientation_horizontal: true,
            halign: Alignment::Center,
            valign: Alignment::Start,
            child_align: Alignment::Center,
        }
    }
}

impl LayoutManager for SimpleLayout {
    fn new() -> Box<dyn LayoutManager> {
        Box::new(Self {
            widget_map: HashMap::new(),
            container: gtk::Box::new(gtk::Orientation::Horizontal, 5),
            focused: None,
            config: SimpleLayoutConfig::default(),
        })
    }
    fn parse_config(&mut self, config: ron::Value) -> Result<()> {
        self.config = config.into_rust()?;
        self.configure_container();
        for widget in self.widget_map.values() {
            self.configure_widget(widget);
        }
        log::debug!("SimpleLayout config parsed: {:#?}", self.config);

        Ok(())
    }
    fn get_name(&self) -> &'static str {
        NAME
    }
    fn get_primary_widget(&self) -> gtk::Widget {
        self.container.clone().upcast()
    }
    fn add_activity(&mut self, activity_id: &ActivityIdentifier, widget: ActivityWidget) {
        if self.widget_map.contains_key(activity_id) {
            return;
        }
        self.configure_widget(&widget);
        self.container.append(&widget.clone());
        self.widget_map.insert(activity_id.clone(), widget);
        if self.focused.is_none() {
            self.focused = Some(activity_id.clone());
        }
    }
    fn get_activity(&self, activity: &ActivityIdentifier) -> Option<&ActivityWidget> {
        self.widget_map.get(activity)
    }
    fn remove_activity(&mut self, activity: &ActivityIdentifier) {
        if let Some(widget) = self.widget_map.get(activity) {
            self.container.remove(widget);
            self.widget_map.remove(activity);
            if let Some(focused) = &self.focused {
                if focused == activity {
                    self.focused = self.widget_map.keys().next().cloned() //get one, doesn't matter which
                }
            }
        }
    }
    fn list_activities(&self) -> Vec<&ActivityIdentifier> {
        self.widget_map.keys().collect()
    }
    fn set_focus(&mut self, identifier: &ActivityIdentifier) -> Result<()> {
        let widget = self
            .widget_map
            .get(identifier)
            .with_context(|| format!("Activity {} not found", identifier))?;
        self.container.set_focus_child(Some(&widget.clone()));
        self.focused = Some(identifier.clone());
        Ok(())
    }
    fn get_focused(&self) -> Option<&ActivityIdentifier> {
        self.focused.as_ref()
    }
    fn cycle_focus_next(&mut self) -> Result<()> {
        let focused = self
            .focused
            .as_ref()
            .ok_or(anyhow!("No activity focused"))?;
        let focused_widget = self
            .widget_map
            .get(focused)
            .ok_or(anyhow!("No widget for focused activity"))?;
        let next_widget = focused_widget
            .next_sibling()
            .ok_or(anyhow!("No next widget to focus on"))?;
        self.container.set_focus_child(Some(&next_widget));
        Ok(())
    }
    fn cycle_focus_previous(&mut self) -> Result<()> {
        let focused = self
            .focused
            .as_ref()
            .ok_or(anyhow!("No activity focused"))?;
        let focused_widget = self
            .widget_map
            .get(focused)
            .ok_or(anyhow!("No widget for focused activity"))?;
        let next_widget = focused_widget
            .prev_sibling()
            .ok_or(anyhow!("No previous widget to focus on"))?;
        self.container.set_focus_child(Some(&next_widget));
        Ok(())
    }
}

impl SimpleLayout {
    fn configure_widget(&self, widget: &ActivityWidget) {
        match self.config.orientation_horizontal {
            true => {
                widget.set_valign(self.config.child_align.into_gtk());
            }
            false => {
                widget.set_halign(self.config.child_align.into_gtk());
            }
        }
    }
    fn configure_container(&self) {
        match self.config.orientation_horizontal {
            true => {
                self.container.set_orientation(gtk::Orientation::Horizontal);
                self.container.set_halign(self.config.halign.into_gtk());
                self.container.set_valign(self.config.valign.into_gtk());
            }
            false => {
                self.container.set_orientation(gtk::Orientation::Vertical);
                self.container.set_halign(self.config.halign.into_gtk());
                self.container.set_valign(self.config.valign.into_gtk());
            }
        }
    }
}
