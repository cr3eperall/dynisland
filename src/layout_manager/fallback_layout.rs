use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use abi_stable::{
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError, ROption,
        RResult::{self, RErr, ROk},
        RString, RVec,
    },
};
use anyhow::Result;
use dynisland_core::{
    abi::{
        abi_stable, gdk, glib, gtk,
        layout::{LayoutManagerType, SabiLayoutManager, SabiLayoutManager_TO},
        log,
        module::ActivityIdentifier,
        SabiApplication, SabiWidget,
    },
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
    ron,
};
use gdk::prelude::*;
use glib::SourceId;
use gtk::{prelude::*, ApplicationWindow, EventController, StateFlags};
use ron::ser::PrettyConfig;

use crate::layout_manager::{
    self,
    config::{FallbackLayoutConfigMain, FallbackLayoutConfigMainOptional},
};

pub struct FallbackLayout {
    app: gtk::Application,
    windows_containers: HashMap<String, (ApplicationWindow, gtk::Box)>,
    widget_map: HashMap<ActivityIdentifier, ActivityWidget>,
    cancel_minimize: Rc<RefCell<HashMap<ActivityIdentifier, SourceId>>>,
    config: FallbackLayoutConfigMain,
}

#[sabi_extern_fn]
pub extern "C" fn new(app: SabiApplication) -> RResult<LayoutManagerType, RBoxError> {
    let app = app.try_into().unwrap();
    let this = FallbackLayout {
        app,
        windows_containers: HashMap::new(),
        widget_map: HashMap::new(),
        cancel_minimize: Rc::new(RefCell::new(HashMap::new())),
        config: FallbackLayoutConfigMain::default(),
    };
    ROk(SabiLayoutManager_TO::from_value(this, TD_CanDowncast))
}

impl SabiLayoutManager for FallbackLayout {
    fn init(&mut self) {
        self.update_windows();
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        // let conf = ron::from_str::<ron::Value>(&config)
        //     .with_context(|| "failed to parse config to value")
        //     .unwrap();
        let mut conf_opt = FallbackLayoutConfigMainOptional::default();
        match serde_json::from_str(&config) {
            Ok(conf) => {
                conf_opt = conf;
            }
            Err(err) => {
                log::warn!(
                    "Failed to parse config into struct, using default: {:#?}",
                    err
                );
            }
        }

        self.config = conf_opt.into_main_config();
        log::trace!("current config: {:#?}", self.config);

        if self.app.windows().first().is_some() {
            self.update_windows();
            for (window_name, window_config) in self.config.windows.iter() {
                let window = &self.windows_containers.get(window_name).unwrap().0;
                window_config
                    .window_position
                    .reconfigure_window(&window.clone().upcast());
            }
        }
        self.configure_containers();

        for (id, widget) in self.widget_map.iter() {
            self.configure_widget(id, widget);
        }

        ROk(())
    }
    fn default_config(&self) -> RResult<RString, RBoxError> {
        let mut conf = FallbackLayoutConfigMain::default();
        conf.windows.clear();
        match ron::ser::to_string_pretty(&conf, PrettyConfig::default()) {
            Ok(map) => ROk(RString::from(map)),
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
        self.configure_widget(activity_id, &widget);
        self.add_activity_to_container(activity_id, &widget);
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
            if self
                .remove_activity_from_container(activity, widget)
                .is_err()
            {
                return;
            }
        }
    }
    fn list_activities(&self) -> RVec<ActivityIdentifier> {
        self.widget_map.keys().cloned().collect()
    }
    fn list_windows(&self) -> RVec<RString> {
        self.windows_containers
            .keys()
            .map(|s| RString::from(s.clone()))
            .collect()
    }
    fn activity_notification(
        &self,
        activity: &ActivityIdentifier,
        mode_id: u8,
        duration: ROption<u64>,
    ) {
        if let Some(widget) = self.widget_map.get(activity) {
            let mode = ActivityMode::try_from(mode_id).unwrap();
            widget.set_mode(mode);
            if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                return;
            }
            let default_timeout = self
                .config
                .get_for_window(&activity.metadata().window_name().unwrap_or_default())
                .auto_minimize_timeout;
            let timeout = duration.unwrap_or(default_timeout as u64);
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

impl FallbackLayout {
    fn get_window_name(&self, activity_id: &ActivityIdentifier) -> String {
        let requested_window = activity_id.metadata().window_name().unwrap_or_default();
        if self.windows_containers.contains_key(&requested_window) {
            requested_window
        } else {
            "".to_string()
        }
    }
    fn configure_widget(&self, id: &ActivityIdentifier, widget: &ActivityWidget) {
        let config = self
            .config
            .get_for_window(&id.metadata().window_name().unwrap_or_default());
        widget.set_valign(config.window_position.v_anchor.map_gtk());
        widget.set_halign(config.window_position.h_anchor.map_gtk());
        // remove old controllers
        let mut controllers = vec![];
        for controller in widget
            .observe_controllers()
            .iter::<glib::Object>()
            .flatten()
            .flat_map(|c| c.downcast::<EventController>())
        {
            if let Some(name) = controller.name() {
                if name == "press_gesture" || name == "focus_controller" {
                    controllers.push(controller);
                }
            }
        }
        for controller in controllers.iter() {
            widget.remove_controller(controller);
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
        if config.auto_minimize_timeout >= 0 {
            let cancel_minimize = self.cancel_minimize.clone();
            let timeout = config.auto_minimize_timeout;
            let activity_id = id.clone();
            focus_in.connect_leave(move |evt| {
                let aw = evt.widget().downcast::<ActivityWidget>().unwrap();
                let mode = aw.mode();
                if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                    return;
                }
                let id = glib::timeout_add_local_once(
                    Duration::from_millis(timeout.try_into().unwrap()),
                    move || {
                        if !aw.state_flags().contains(StateFlags::PRELIGHT) && aw.mode() == mode {
                            //mouse is not on widget and mode hasn't changed
                            aw.set_mode(ActivityMode::Compact);
                        }
                    },
                );
                let mut cancel_minimize = cancel_minimize.borrow_mut();
                if let Some(source) = cancel_minimize.remove(&activity_id) {
                    if glib::MainContext::default()
                        .find_source_by_id(&source)
                        .is_some()
                    {
                        source.remove();
                    }
                }

                cancel_minimize.insert(activity_id.clone(), id);
            });
            widget.add_controller(focus_in);
        }
    }

    fn configure_containers(&self) {
        for (window_name, (_, container)) in self.windows_containers.iter() {
            let config = self.config.get_for_window(window_name);
            if config.orientation_horizontal {
                container.set_orientation(gtk::Orientation::Horizontal);
            } else {
                container.set_orientation(gtk::Orientation::Vertical);
            }
            if !config.window_position.layer_shell {
                container.set_halign(config.window_position.h_anchor.map_gtk());
                container.set_valign(config.window_position.v_anchor.map_gtk());
            }
            container.set_spacing(0);
        }
    }
    // FIXME: this is terribly inefficient
    fn update_windows(&mut self) {
        let mut orphan_widgets: Vec<ActivityIdentifier> = Vec::new();
        // remove orphaned windows
        for window in self.app.windows() {
            let window_name = window.title().unwrap_or_default();
            if !self
                .windows_containers
                .contains_key(&window_name.to_string())
            {
                for child in window
                    .child()
                    .unwrap()
                    .observe_children()
                    .iter::<glib::Object>()
                    .flatten()
                {
                    let widget = child.downcast::<gtk::Widget>().unwrap();
                    if let Some((id, _)) = self.widget_map.iter().find(|(_, w)| *w == &widget) {
                        orphan_widgets.push(id.clone());
                    }
                }
                window.close();
                log::warn!("removing orphaned window {}", window_name);
            }
        }
        // remove windows that are no longer in the config
        let mut windows_to_remove: Vec<String> = Vec::new();
        for (window_name, (window, _)) in self.windows_containers.iter() {
            if !self.config.windows.contains_key(&window_name.to_string()) {
                for child in window
                    .child()
                    .unwrap()
                    .observe_children()
                    .iter::<glib::Object>()
                    .flatten()
                {
                    let widget = child.downcast::<gtk::Widget>().unwrap();
                    if let Some((id, _)) = self.widget_map.iter().find(|(_, w)| *w == &widget) {
                        orphan_widgets.push(id.clone());
                    }
                }
                windows_to_remove.push(window_name.clone());
                window.close();
            }
        }
        for window_name in windows_to_remove {
            self.windows_containers.remove(&window_name);
            log::trace!("removing window no longer in config {}", window_name);
        }
        // create new windows
        let existing_windows: Vec<String> = self.windows_containers.keys().cloned().collect();
        let mut windows_to_create: Vec<String> = Vec::new();
        for window_name in self.config.windows.keys() {
            if !existing_windows.contains(window_name) {
                windows_to_create.push(window_name.clone());
            }
        }
        for window_name in windows_to_create {
            log::trace!("creating new window {}", window_name);
            self.create_new_window(&window_name);
        }
        for widget_id in orphan_widgets {
            let widget = self.widget_map.get(&widget_id).unwrap().clone();
            self.add_activity_to_container(&ActivityIdentifier::new("", ""), &widget);
            log::trace!("readding orphaned widget {}", widget_id);
        }
        let mut to_update = Vec::new();
        for (id, widget) in self.widget_map.iter() {
            let parent = widget.parent().unwrap().downcast::<gtk::Box>().unwrap();
            if let Some((current_window, (_, _))) = self
                .windows_containers
                .iter()
                .find(move |(_, (_, container))| &parent == container)
            {
                if let Some(desired_window) = id.metadata().window_name() {
                    if desired_window != *current_window
                        && self.config.windows.contains_key(&desired_window)
                    {
                        to_update.push(id.clone());
                    }
                }
            }
        }
        for id in to_update {
            let widget = self.widget_map.get(&id).unwrap().clone();
            self.remove_activity_from_container(&id, widget.clone())
                .unwrap();
            self.add_activity_to_container(&id, &widget);
            log::trace!("moving widget {} to correct window", id);
        }
        log::debug!("updated windows");
    }

    fn create_new_window(&mut self, window_name: &str) {
        if self.windows_containers.contains_key(window_name) {
            return;
        }
        if !self.config.windows.contains_key(window_name) {
            return;
        }
        let window = gtk::ApplicationWindow::new(&self.app);
        window.set_title(Some(window_name));
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        container.add_css_class("activity-container");
        window.set_child(Some(&container));
        self.configure_containers();
        self.config
            .get_for_window(window_name)
            .window_position
            .init_window(&window.clone().upcast());
        //show window
        window.present();
        self.windows_containers
            .insert(window_name.to_string(), (window, container));
        log::trace!("created new window {}", window_name);
    }

    fn add_activity_to_container(
        &mut self,
        activity_id: &ActivityIdentifier,
        widget: &ActivityWidget,
    ) {
        let window_name = self.get_window_name(activity_id);
        let container = &self
            .windows_containers
            .get(&window_name)
            .expect(&format!(
                "there should be a default container for {}",
                window_name
            ))
            .1;
        container.append(widget);
    }

    fn remove_activity_from_container(
        &mut self,
        activity: &ActivityIdentifier,
        widget: ActivityWidget,
    ) -> Result<()> {
        let widget_container = match widget.parent().unwrap().downcast::<gtk::Box>() {
            Ok(parent) => parent,
            Err(_) => {
                log::warn!(
                    "Error removing {activity:?} from {}: parent is not a Box",
                    layout_manager::NAME
                );
                anyhow::bail!(
                    "Error removing {activity:?} from {}: parent is not a Box",
                    layout_manager::NAME
                );
            }
        };
        let name = if let Some((name, (window, container))) = self
            .windows_containers
            .iter()
            .find(move |(_, (_, container))| &widget_container == container)
        {
            container.remove(&widget);
            if container.first_child().is_some() {
                return Ok(());
            }
            window.close();
            name.clone()
        } else {
            return Ok(());
        };
        self.windows_containers.remove(&name.clone());
        log::debug!("removing empty window {}", name);
        self.create_new_window(&name.clone());
        Ok(())
    }
}
