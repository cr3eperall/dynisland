use std::{collections::HashMap, time::Duration};

use abi_stable::{
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError, ROption,
        RResult::{self, RErr, ROk},
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
use gdk::prelude::*;
use glib::Cast;
use gtk::{prelude::*, EventController, StateFlags};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::layout_manager::window_position::{Alignment, WindowPosition};

pub struct SimpleLayout {
    app: gtk::Application,
    widget_map: HashMap<ActivityIdentifier, ActivityWidget>,
    container: Option<gtk::Box>,
    config: SimpleLayoutConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SimpleLayoutConfig {
    orientation_horizontal: bool,
    window_position: WindowPosition,
    child_align: Alignment,
    open_debugger: bool,
    auto_minimize_timeout: i32,
}
impl Default for SimpleLayoutConfig {
    fn default() -> Self {
        Self {
            orientation_horizontal: true,
            window_position: WindowPosition::default(),
            child_align: Alignment::Center,
            open_debugger: false,
            auto_minimize_timeout: 5000,
        }
    }
}

#[sabi_extern_fn]
pub fn new(app: SabiApplication) -> RResult<LayoutManagerType, RBoxError> {
    let app = app.try_into().unwrap();
    let this = SimpleLayout {
        app,
        widget_map: HashMap::new(),
        container: None,
        config: SimpleLayoutConfig::default(),
    };
    ROk(SabiLayoutManager_TO::from_value(this, TD_CanDowncast))
}

impl SabiLayoutManager for SimpleLayout {
    fn init(&mut self) {
        self.create_new_window();
        gtk::Window::set_interactive_debugging(self.config.open_debugger);
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();
        match conf.into_rust() {
            Ok(conf) => {
                self.config = conf;
            }
            Err(err) => {
                log::error!("Failed to parse config into struct: {:#?}", err);
            }
        }
        log::debug!("current config: {:#?}", self.config);

        self.configure_container();
        if let Some(window) = self.app.windows().first() {
            self.config.window_position.reconfigure_window(window);
        }

        for widget in self.widget_map.values() {
            self.configure_widget(widget);
        }

        ROk(())
    }
    fn default_config(&self) -> RResult<RString, RBoxError> {
        let conf = SimpleLayoutConfig::default();
        match ron::ser::to_string_pretty(&conf, PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
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
        self.container
            .as_ref()
            .expect("there should be a container")
            .append(&widget);
        self.widget_map.insert(activity_id.clone(), widget);
    }
    fn get_activity(&self, activity: &ActivityIdentifier) -> ROption<SabiWidget> {
        self.widget_map
            .get(activity)
            .map(|wid| SabiWidget::from(wid.clone().upcast::<gtk::Widget>()))
            .into()
    }

    fn remove_activity(&mut self, activity: &ActivityIdentifier) {
        if let Some(widget) = self.widget_map.remove(activity) {
            if let Some(container) = self.container.as_ref() {
                container.remove(&widget);
                if container.first_child().is_none() {
                    // update window, for some reason if there are no children
                    // in the container, the last child stays displayed
                    if let Some(win) = self.app.windows().first() {
                        win.close();
                        self.create_new_window();
                    }
                }
            }
        }
    }
    fn list_activities(&self) -> RVec<&ActivityIdentifier> {
        self.widget_map.keys().collect()
    }
    fn focus_activity(&self, activity: &ActivityIdentifier, mode_id: u8) {
        if let Some(widget) = self.widget_map.get(activity) {
            let mode = ActivityMode::try_from(mode_id).unwrap();
            widget.set_mode(mode);
            if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                return;
            }
            let timeout = self.config.auto_minimize_timeout;
            let widget = widget.clone();
            glib::timeout_add_local_once(
                Duration::from_millis(timeout.try_into().unwrap()),
                move || {
                    if !widget.state_flags().contains(StateFlags::PRELIGHT) && widget.mode() == mode
                    {
                        //mouse is not on widget and mode hasn't changed
                        widget.set_mode(ActivityMode::Compact);
                    }
                },
            );
        }
    }
}

impl SimpleLayout {
    fn configure_widget(&self, widget: &ActivityWidget) {
        match self.config.orientation_horizontal {
            true => {
                widget.set_valign(self.config.child_align.map_gtk());
            }
            false => {
                widget.set_halign(self.config.child_align.map_gtk());
            }
        }
        // remove old controllers
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

        // Minimal mode to Compact mode controller
        press_gesture.set_button(gdk::BUTTON_PRIMARY);
        press_gesture.connect_released(|gest, _, x, y| {
            let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
            if x < 0.0
                || y < 0.0
                || x > aw.size(gtk::Orientation::Horizontal).into()
                || y > aw.size(gtk::Orientation::Vertical).into()
            {
                return;
            }
            if let ActivityMode::Minimal = aw.mode() {
                aw.set_mode(ActivityMode::Compact);
                gest.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        widget.add_controller(press_gesture);

        // auto minimize (to Compact mode) controller
        if self.config.auto_minimize_timeout >= 0 {
            let timeout = self.config.auto_minimize_timeout;
            focus_in.connect_leave(move |evt| {
                let aw = evt.widget().downcast::<ActivityWidget>().unwrap();
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
        let container = if self.container.is_none() {
            return;
        } else {
            self.container.as_ref().unwrap()
        };
        if self.config.orientation_horizontal {
            container.set_orientation(gtk::Orientation::Horizontal);
        } else {
            container.set_orientation(gtk::Orientation::Vertical);
        }
        if !self.config.window_position.layer_shell {
            container.set_halign(self.config.window_position.h_anchor.map_gtk());
            container.set_valign(self.config.window_position.v_anchor.map_gtk());
        }
    }

    fn create_new_window(&mut self) {
        let window = gtk::ApplicationWindow::new(&self.app);

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        container.add_css_class("activity-container");
        self.container = Some(container);
        window.set_child(self.container.as_ref());
        self.config
            .window_position
            .init_window(&window.clone().upcast());
        //show window
        window.present();
    }
}
