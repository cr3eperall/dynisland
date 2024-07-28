use std::collections::HashMap;

use abi_stable::{
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError, ROption,
        RResult::{self, ROk},
        RString, RVec,
    },
};
use anyhow::Context;
use dynisland_abi::{
    layout::{LayoutManagerType, SabiLayoutManager, SabiLayoutManager_TO},
    module::ActivityIdentifier,
    SabiApplication, SabiWidget,
};
use dynisland_core::graphics::activity_widget::{
    boxed_activity_mode::ActivityMode, ActivityWidget,
};
use gtk::{prelude::*, GestureClick, StateFlags, Window};
use gtk_layer_shell::LayerShell;
use serde::{Deserialize, Serialize};

pub const NAME: &str = "SimpleLayout";

pub struct SimpleLayout {
    app: gtk::Application,
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
fn bool_false() -> bool {
    false
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
    #[serde(default = "bool_false")]
    debugging: bool,
}
impl Default for SimpleLayoutConfig {
    fn default() -> Self {
        Self {
            orientation_horizontal: true,
            halign: Alignment::Center,
            valign: Alignment::Start,
            child_align: Alignment::Center,
            debugging: false,
        }
    }
}
#[sabi_extern_fn]
pub fn new(app: SabiApplication) -> RResult<LayoutManagerType, RBoxError> {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    container.add_css_class("activity-container");
    let app = app.try_into().unwrap();
    let this = SimpleLayout {
        app,
        widget_map: HashMap::new(),
        container,
        focused: None,
        config: SimpleLayoutConfig::default(),
    };
    ROk(SabiLayoutManager_TO::from_value(this, TD_CanDowncast))
}

impl SabiLayoutManager for SimpleLayout {
    fn init(&self) {
        let window = gtk::ApplicationWindow::new(&self.app);
        // window.set_resizable(false);
        window.set_child(Some(&self.container));
        // init_layer_shell(&window.clone().upcast());
        gtk::Window::set_interactive_debugging(self.config.debugging);

        //show window
        window.connect_destroy(|_| std::process::exit(0));
        window.present();
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();
        let old_config = self.config.clone();
        self.config = conf
            .into_rust()
            .with_context(|| "failed to parse config to struct")
            .unwrap_or_else(|err| {
                log::error!("parsing error: {:#?}", err);
                old_config
            });

        self.configure_container();

        for widget in self.widget_map.values() {
            self.configure_widget(widget);
        }

        ROk(())
    }

    fn add_activity(&mut self, activity_id: &ActivityIdentifier, widget: SabiWidget) {
        if self.widget_map.contains_key(activity_id) {
            return;
        }
        let widget: gtk::Widget = widget.try_into().unwrap();
        let widget = match widget.downcast::<ActivityWidget>() {
            Ok(widget) => widget,
            Err(_) => {
                log::error!("widget {} is not an ActivityWidget", activity_id);
                return;
            }
        };
        self.configure_widget(&widget);
        self.container.append(&widget.clone());
        self.widget_map.insert(activity_id.clone(), widget);
        if self.focused.is_none() {
            self.focused = Some(activity_id.clone());
        }
    }
    fn get_activity(&self, activity: &ActivityIdentifier) -> ROption<SabiWidget> {
        self.widget_map
            .get(activity)
            .map(|wid| SabiWidget::from(wid.clone().upcast::<gtk::Widget>()))
            .into()
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
    fn list_activities(&self) -> RVec<&ActivityIdentifier> {
        self.widget_map.keys().collect()
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

        let press_gesture = gtk::GestureClick::new();
        press_gesture.set_button(gdk::BUTTON_PRIMARY);

        let aw = widget.clone();
        press_gesture.connect_released(move |gest, _, _, _| {
            if let ActivityMode::Minimal = aw.mode() {
                // log::info!("set compact");
                aw.set_mode(ActivityMode::Compact);
                gest.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        widget.add_controller(press_gesture);

        let focus_in = gtk::EventControllerMotion::new();
        let aw = widget.clone();
        focus_in.connect_leave(move|_|{
            let aw = aw.clone();
            let mode = aw.mode();
            if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact){
                return;
            }
            glib::timeout_add_seconds_local_once(5, move ||{
                log::info!("time");
                if !aw.state_flags().contains(StateFlags::PRELIGHT) && aw.mode()==mode {//mouse is not on widget and mode hasn't changed
                    aw.set_mode(ActivityMode::Compact);
                    log::info!("updated");
                }
            });
        });
        widget.add_controller(focus_in);
        
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

pub fn init_layer_shell(window: &Window) {
    window.init_layer_shell();
    window.set_layer(gtk_layer_shell::Layer::Top);
    window.set_anchor(gtk_layer_shell::Edge::Top, true);
    window.set_anchor(gtk_layer_shell::Edge::Top, true);
    window.set_margin(gtk_layer_shell::Edge::Top, 3);
    window.set_namespace("dynisland");
    window.set_exclusive_zone(-1); // TODO add to config
    window.set_resizable(false);
    window.queue_resize();
}
