use std::{collections::HashMap, time::Duration};

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
use gtk::{prelude::*, EventController, StateFlags, Window};
use gtk_layer_shell::LayerShell;
use serde::{Deserialize, Serialize};

pub const NAME: &str = "SimpleLayout";

pub struct SimpleLayout {
    app: gtk::Application,
    widget_map: HashMap<ActivityIdentifier, ActivityWidget>,
    container: gtk::Box,
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
    pub fn map_gtk(&self) -> gtk::Align {
        match self {
            Alignment::Start => gtk::Align::Start,
            Alignment::Center => gtk::Align::Center,
            Alignment::End => gtk::Align::End,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SimpleLayoutConfig {
    orientation_horizontal: bool,
    h_anchor: Alignment,
    v_anchor: Alignment,
    margin_x: i32,
    margin_y: i32,
    monitor: String,
    child_align: Alignment,
    open_debugger: bool,
    windowed: bool,
    auto_minimize_timeout: i32,
}
impl Default for SimpleLayoutConfig {
    fn default() -> Self {
        Self {
            orientation_horizontal: true,
            h_anchor: Alignment::Center,
            v_anchor: Alignment::Start,
            margin_x: 0,
            margin_y: 0,
            monitor: String::from(""),
            child_align: Alignment::Center,
            open_debugger: false,
            windowed: false,
            auto_minimize_timeout: 5000,
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
        config: SimpleLayoutConfig::default(),
    };
    ROk(SabiLayoutManager_TO::from_value(this, TD_CanDowncast))
}

impl SabiLayoutManager for SimpleLayout {
    fn init(&self) {
        let window = gtk::ApplicationWindow::new(&self.app);

        window.set_child(Some(&self.container));
        if !self.config.windowed {
            window.init_layer_shell();
            init_layer_shell(&window.clone().upcast(), &self.config.clone());
        }
        gtk::Window::set_interactive_debugging(self.config.open_debugger);

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
        log::debug!("current config: {:#?}", self.config);

        self.configure_container();
        if !self.config.windowed {
            if let Some(window) = self.app.windows().first() {
                init_layer_shell(&window.clone(), &self.config.clone());
            }
        }

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
                widget.set_valign(self.config.child_align.map_gtk());
                log::info!(
                    "{} {} {}",
                    widget.name(),
                    widget.valign(),
                    self.config.child_align.map_gtk()
                );
            }
            false => {
                widget.set_halign(self.config.child_align.map_gtk());
            }
        }

        for controller in widget
            .observe_controllers()
            .iter::<glib::Object>()
            .flatten()
            .flat_map(|c| c.downcast::<EventController>())
        {
            if let Some(name) = controller.name() {
                if name == "press_gesture" || name == "focus_controller" {
                    widget.remove_controller(&controller);
                }
            }
        }

        let press_gesture = gtk::GestureClick::new();
        press_gesture.set_name(Some("press_gesture"));

        let focus_in = gtk::EventControllerMotion::new();
        focus_in.set_name(Some("focus_controller"));

        press_gesture.set_button(gdk::BUTTON_PRIMARY);
        let aw = widget.clone();
        press_gesture.connect_released(move |gest, _, _, _| {
            if let ActivityMode::Minimal = aw.mode() {
                aw.set_mode(ActivityMode::Compact);
                gest.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        widget.add_controller(press_gesture);

        if self.config.auto_minimize_timeout >= 0 {
            let timeout = self.config.auto_minimize_timeout;
            let aw = widget.clone();
            focus_in.connect_leave(move |_| {
                let aw = aw.clone();
                let mode = aw.mode();
                if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                    return;
                }
                glib::timeout_add_local_once(
                    Duration::from_millis(timeout.try_into().unwrap()),
                    move || {
                        if !aw.state_flags().contains(StateFlags::PRELIGHT) && aw.mode() == mode {
                            //mouse is not on widget and mode hasn't changed
                            aw.set_mode(ActivityMode::Compact);
                        }
                    },
                );
            });
            widget.add_controller(focus_in);
        }
    }

    fn configure_container(&self) {
        if self.config.windowed {
            if self.config.orientation_horizontal {
                self.container.set_orientation(gtk::Orientation::Horizontal);
            } else {
                self.container.set_orientation(gtk::Orientation::Vertical);
            }
            self.container.set_halign(self.config.h_anchor.map_gtk());
            self.container.set_valign(self.config.v_anchor.map_gtk());
        }
    }
}

pub fn init_layer_shell(window: &Window, config: &SimpleLayoutConfig) {
    window.set_layer(gtk_layer_shell::Layer::Top);
    // window.set_anchor(gtk_layer_shell::Edge::Top, true);
    // window.set_anchor(gtk_layer_shell::Edge::Top, true);
    match config.v_anchor {
        Alignment::Start => {
            window.set_anchor(gtk_layer_shell::Edge::Top, true);
            window.set_anchor(gtk_layer_shell::Edge::Bottom, false);
            window.set_margin(gtk_layer_shell::Edge::Top, config.margin_y);
        }
        Alignment::Center => {
            window.set_anchor(gtk_layer_shell::Edge::Top, false);
            window.set_anchor(gtk_layer_shell::Edge::Bottom, false);
        }
        Alignment::End => {
            window.set_anchor(gtk_layer_shell::Edge::Top, false);
            window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
            window.set_margin(gtk_layer_shell::Edge::Bottom, config.margin_y);
        }
    }
    match config.h_anchor {
        Alignment::Start => {
            window.set_anchor(gtk_layer_shell::Edge::Left, true);
            window.set_anchor(gtk_layer_shell::Edge::Right, false);
            window.set_margin(gtk_layer_shell::Edge::Left, config.margin_x);
        }
        Alignment::Center => {
            window.set_anchor(gtk_layer_shell::Edge::Left, false);
            window.set_anchor(gtk_layer_shell::Edge::Right, false);
        }
        Alignment::End => {
            window.set_anchor(gtk_layer_shell::Edge::Left, false);
            window.set_anchor(gtk_layer_shell::Edge::Right, true);
            window.set_margin(gtk_layer_shell::Edge::Right, config.margin_x);
        }
    }
    let mut monitor = None;
    for mon in gdk::Display::default()
        .unwrap()
        .monitors()
        .iter::<gdk::Monitor>()
    {
        let mon = match mon {
            Ok(monitor) => monitor,
            Err(_) => {
                continue;
            }
        };
        if mon
            .connector()
            .unwrap()
            .eq_ignore_ascii_case(&config.monitor)
        {
            monitor = Some(mon);
            break;
        }
    }
    if let Some(monitor) = monitor {
        window.set_monitor(&monitor);
    }
    window.set_namespace("dynisland");
    window.set_exclusive_zone(-1); // TODO add to config
    window.set_resizable(false);
    window.queue_resize();
}
