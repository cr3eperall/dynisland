use css_anim::{
    soy::EaseFunction,
    transition::{TransitionDef, TransitionManager},
};
use rand::{distributions::Alphanumeric, Rng};
use std::{cell::RefCell, time::Duration};

use anyhow::Result;
use glib::{object_subclass, prelude::*, wrapper};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*};

use super::{local_css_context::ActivityWidgetLocalCssContext, util, BLUR_RADIUS};

#[derive(Clone, glib::Boxed, Debug, Copy)]
#[boxed_type(name = "BoxedActivityMode")]
pub enum ActivityMode {
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
    Overlay = 3,
}

impl ToString for ActivityMode {
    fn to_string(&self) -> String {
        match self {
            ActivityMode::Minimal => "minimal".to_string(),
            ActivityMode::Compact => "compact".to_string(),
            ActivityMode::Expanded => "expanded".to_string(),
            ActivityMode::Overlay => "overlay".to_string(),
        }
    }
}

wrapper! {
    pub struct ActivityWidget(ObjectSubclass<ActivityWidgetPriv>)
    @extends gtk::Container, gtk::Widget;
}

#[derive(Properties)]
#[properties(wrapper_type = ActivityWidget)]
pub struct ActivityWidgetPriv {
    #[property(get, set, nick = "Change mode", blurb = "The Activity Mode")]
    pub(super) mode: RefCell<ActivityMode>,

    #[property(get, nick = "Local CSS Provider")]
    pub(super) local_css_context: RefCell<ActivityWidgetLocalCssContext>,

    #[property(get, set, nick = "Widget name")]
    pub(super) name: RefCell<String>,

    pub(super) last_mode: RefCell<ActivityMode>,

    pub(super) transition_manager: RefCell<TransitionManager>,

    pub(super) background_widget: RefCell<Option<gtk::Widget>>,

    pub(super) minimal_mode_widget: RefCell<Option<gtk::Widget>>,

    pub(super) compact_mode_widget: RefCell<Option<gtk::Widget>>,

    pub(super) expanded_mode_widget: RefCell<Option<gtk::Widget>>,

    pub(super) overlay_mode_widget: RefCell<Option<gtk::Widget>>,
}

//set properties
#[glib::derived_properties]
impl ObjectImpl for ActivityWidgetPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn notify(&self, pspec: &glib::ParamSpec) {
        self.parent_notify(pspec)
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "mode" => {
                self.last_mode.replace(*self.mode.borrow());
                self.mode.replace(value.get().unwrap());

                let mode = self.mode.borrow();
                let last_mode = self.last_mode.borrow();

                let mut prev_size = (
                    self.obj().allocated_width() as f64,
                    self.obj().allocated_height() as f64,
                );
                if let Some(widget) = &*self.get_mode_widget(*last_mode).borrow() {
                    let tmp = util::get_final_widget_size(
                        widget,
                        *self.last_mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    );
                    prev_size = (tmp.0 as f64, tmp.1 as f64);
                }
                let mut next_size = (
                    self.obj().allocated_width() as f64,
                    self.obj().allocated_height() as f64,
                );
                if let Some(widget) = &*self.get_mode_widget(*mode).borrow() {
                    let tmp = util::get_final_widget_size(
                        widget,
                        *self.mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    );
                    next_size = (tmp.0 as f64, tmp.1 as f64);
                }
                let mut css_context = self.local_css_context.borrow_mut();
                let bigger = next_size.0 > prev_size.0 || next_size.1 > prev_size.1;

                self.transition_opacity(false, bigger, &css_context, 1.0, 0.0);
                self.transition_opacity(true, bigger, &css_context, 0.0, 1.0);

                self.transition_blur(false, bigger, &css_context, 0.0, BLUR_RADIUS as f64);
                self.transition_blur(true, bigger, &css_context, BLUR_RADIUS as f64, 0.0);

                self.transition_stretch(
                    false,
                    bigger,
                    &css_context,
                    (1.0, 1.0),
                    (next_size.0 / prev_size.0, next_size.1 / prev_size.1),
                );
                self.transition_stretch(
                    true,
                    bigger,
                    &css_context,
                    (prev_size.0 / next_size.0, prev_size.1 / next_size.1),
                    (1.0, 1.0),
                );

                self.raise_windows();
                if self.get_mode_widget(*self.mode.borrow()).borrow().is_some() {
                    css_context
                        .set_size((next_size.0 as i32, next_size.1 as i32))
                        .expect("failed to set activity size");
                }
                self.obj().queue_draw(); // Queue a draw call with the updated value
            }
            // "transition-duration" => {
            //     self.transition_duration.replace(value.get().unwrap());
            // }
            "name" => {
                self.obj().style_context().remove_class(&self.name.borrow());

                self.name.replace(value.get().unwrap());
                self.local_css_context
                    .borrow_mut()
                    .set_name(value.get().unwrap())
                    .expect("failed to set activity name");
                self.obj().style_context().add_class(value.get().unwrap());
            }
            x => panic!("Tried to set inexistant property of ActivityWidget: {}", x),
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}

//default data
impl Default for ActivityWidgetPriv {
    fn default() -> Self {
        let name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect();
        let mut transition_manager = TransitionManager::new(false);
        util::init_transition_properties(&mut transition_manager);

        Self {
            mode: RefCell::new(ActivityMode::Minimal),
            // transition_duration: RefCell::new(0),
            local_css_context: RefCell::new(ActivityWidgetLocalCssContext::new(&name)),
            last_mode: RefCell::new(ActivityMode::Minimal),
            name: RefCell::new(name),
            transition_manager: RefCell::new(transition_manager),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

impl ActivityWidgetPriv {
    pub(super) fn get_mode_widget(&self, mode: ActivityMode) -> &RefCell<Option<gtk::Widget>> {
        match mode {
            ActivityMode::Minimal => &self.minimal_mode_widget,
            ActivityMode::Compact => &self.compact_mode_widget,
            ActivityMode::Expanded => &self.expanded_mode_widget,
            ActivityMode::Overlay => &self.overlay_mode_widget,
        }
    }

    pub(super) fn get_child_aligned_allocation(&self, child: &gtk::Widget) -> gdk::Rectangle {
        let parent_allocation = self.obj().allocation();
        // let x: i32;
        // let y: i32;
        // let mut width = child.preferred_width().0;
        // let mut height = child.preferred_height().0;
        // match child.halign() {
        //     gtk::Align::Start => {
        //         x = parent_allocation.x();
        //     }
        //     gtk::Align::End => {
        //         x = parent_allocation.x() + (parent_allocation.width() - width);
        //     }
        //     gtk::Align::Center => {
        //         x = parent_allocation.x()
        //             + ((parent_allocation.width() - width) as f32 / 2.0).ceil() as i32;
        //     }
        //     _ => {
        //         glib::g_warning!(
        //             "warning",
        //             "align set to FILL/BASELINE, this will break resizing"
        //         );
        //         x = parent_allocation.x();
        //         width = parent_allocation.width();
        //     }
        // }
        // match child.valign() {
        //     gtk::Align::Start => {
        //         y = parent_allocation.y();
        //     }
        //     gtk::Align::End => {
        //         y = parent_allocation.y() + (parent_allocation.height() - height);
        //     }
        //     gtk::Align::Center => {
        //         y = parent_allocation.y()
        //             + ((parent_allocation.height() - height) as f32 / 2.0).ceil() as i32;
        //     }
        //     _ => {
        //         glib::g_warning!(
        //             "warning",
        //             "align set to FILL/BASELINE,this will break resizing"
        //         );
        //         y = parent_allocation.y();
        //         height = parent_allocation.height();
        //     }
        // }
        // gtk::Allocation::new(x, y, width, height)
        gtk::Allocation::new(
            parent_allocation.x(),
            parent_allocation.y(),
            child.preferred_width().0,
            child.preferred_height().0,
        )
    }

    pub(super) fn raise_windows(&self) {
        if let Some(widget) = &*self.get_mode_widget(*self.mode.borrow()).borrow() {
            if let Some(widget) = &*self.get_mode_widget(*self.last_mode.borrow()).borrow() {
                match widget.window() {
                    //lower previous window associated to widget if it has one, this "disables" events on the last mode widget
                    Some(window) => window.lower(),
                    None => {
                        // debug!("no window");
                    }
                }
            }
            if let Some(widget) = &*self.background_widget.borrow() {
                match widget.window() {
                    //raise background widget's window if it has one, this creates a default if the new widget doesn't have a window
                    Some(window) => window.raise(),
                    None => {
                        // debug!("no window");
                    }
                }
            }
            match widget.window() {
                //raise window associated to widget if it has one, this "enables" events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
                }
            }
        }
    }

    pub(super) fn transition_opacity(
        &self,
        next_mode: bool,
        bigger: bool,
        css_context: &ActivityWidgetLocalCssContext,
        from: f64,
        to: f64,
    ) {
        let duration = Duration::from_millis(css_context.get_transition_duration());
        let opacity = if next_mode {
            self.mode.borrow()
        } else {
            self.last_mode.borrow()
        }
        .to_string()
            + "-opacity";
        self.transition_manager
            .borrow_mut()
            .set_value_with_transition_from(
                &opacity,
                from,
                to,
                &TransitionDef::new(
                    duration,
                    if bigger ^ next_mode {
                        css_context.get_transition_bigger_opacity()
                    } else {
                        css_context.get_transition_smaller_opacity()
                    },
                    Duration::ZERO,
                    false,
                ),
            );
    }
    pub(super) fn transition_blur(
        &self,
        next_mode: bool,
        bigger: bool,
        css_context: &ActivityWidgetLocalCssContext,
        from: f64,
        to: f64,
    ) {
        let duration = Duration::from_millis(css_context.get_transition_duration());
        let blur = if next_mode {
            self.mode.borrow()
        } else {
            self.last_mode.borrow()
        }
        .to_string()
            + "-blur";
        self.transition_manager
            .borrow_mut()
            .set_value_with_transition_from(
                &blur,
                from,
                to,
                &TransitionDef::new(
                    duration,
                    if bigger ^ next_mode {
                        css_context.get_transition_bigger_blur()
                    } else {
                        css_context.get_transition_smaller_blur()
                    },
                    Duration::ZERO,
                    false,
                ),
            );
    }
    pub(super) fn transition_stretch(
        &self,
        next_mode: bool,
        bigger: bool,
        css_context: &ActivityWidgetLocalCssContext,
        from: (f64, f64),
        to: (f64, f64),
    ) {
        let duration = Duration::from_millis(css_context.get_transition_duration());
        let stretch_x = if next_mode {
            self.mode.borrow()
        } else {
            self.last_mode.borrow()
        }
        .to_string()
            + "-stretch-x";
        let stretch_y = if next_mode {
            self.mode.borrow()
        } else {
            self.last_mode.borrow()
        }
        .to_string()
            + "-stretch-y";
        self.transition_manager
            .borrow_mut()
            .set_value_with_transition_from(
                &stretch_x,
                from.0,
                to.0,
                &TransitionDef::new(
                    duration,
                    if bigger ^ next_mode {
                        css_context.get_transition_bigger_stretch()
                    } else {
                        css_context.get_transition_smaller_stretch()
                    },
                    Duration::ZERO,
                    false,
                ),
            );
        self.transition_manager
            .borrow_mut()
            .set_value_with_transition_from(
                &stretch_y,
                from.1,
                to.1,
                &TransitionDef::new(
                    duration,
                    if bigger ^ next_mode {
                        css_context.get_transition_bigger_stretch()
                    } else {
                        css_context.get_transition_smaller_stretch()
                    },
                    Duration::ZERO,
                    false,
                ),
            );
    }
}

//init widget info
#[object_subclass]
impl ObjectSubclass for ActivityWidgetPriv {
    type ParentType = gtk::Container;
    type Type = ActivityWidget;

    const NAME: &'static str = "ActivityWidget";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("activity-widget");
        // unsafe {
        //     let widget_class = klass as *mut _ as *mut gtk::ffi::GtkWidgetClass;
        //     let mut n: c_uint=0;
        //     let out = gtk::ffi::gtk_widget_class_list_style_properties(widget_class, &mut n);
        //     let sl = std::slice::from_raw_parts_mut(out, n.try_into().unwrap());
        //     for prop in sl {
        //         let pspec=glib::ParamSpec::from_glib_ptr_borrow(&prop.cast_const());
        //      }

        //     gtk::ffi::gtk_widget_class_install_style_property(klass, pspec)
        // }
    }
}

impl Default for ActivityWidget {
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

//set mode widgets and get new instance
impl ActivityWidget {
    pub fn new(name: &str) -> Self {
        let wid = glib::Object::new::<Self>();
        wid.set_has_window(false);
        wid.set_name(name);

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().unwrap(),
            &wid.local_css_context().get_css_provider(),
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        wid
    }
    pub fn set_minimal_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.minimal_mode_widget.borrow() {
            content.unparent();
            content.style_context().remove_class("mode-minimal");
        }

        widget.set_parent(self);
        widget.style_context().add_class("mode-minimal");
        priv_.minimal_mode_widget.replace(Some(widget.clone()));
        if let ActivityMode::Minimal = self.mode() {
            let (width, height) = util::get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger = width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(
                true,
                bigger,
                &self.local_css_context(),
                (
                    self.allocated_width() as f64 / width as f64,
                    self.allocated_height() as f64 / height as f64,
                ),
                (1.0, 1.0),
            );
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
                }
            }
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }
    pub fn set_compact_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.compact_mode_widget.borrow() {
            content.unparent();
            widget.style_context().remove_class("mode-compact");
        }
        widget.set_parent(self);
        widget.style_context().add_class("mode-compact");
        priv_.compact_mode_widget.replace(Some(widget.clone()));
        if let ActivityMode::Compact = self.mode() {
            let (width, height) = util::get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger = width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(
                true,
                bigger,
                &self.local_css_context(),
                (
                    self.allocated_width() as f64 / width as f64,
                    self.allocated_height() as f64 / height as f64,
                ),
                (1.0, 1.0),
            );
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
                }
            }
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }
    pub fn set_expanded_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.expanded_mode_widget.borrow() {
            content.unparent();
            widget.style_context().remove_class("mode-expanded");
        }
        widget.set_parent(self);
        widget.style_context().add_class("mode-expanded");
        priv_.expanded_mode_widget.replace(Some(widget.clone()));
        if let ActivityMode::Expanded = self.mode() {
            let (width, height) = util::get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger = width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(
                true,
                bigger,
                &self.local_css_context(),
                (
                    self.allocated_width() as f64 / width as f64,
                    self.allocated_height() as f64 / height as f64,
                ),
                (1.0, 1.0),
            );
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
                }
            }
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }
    pub fn set_overlay_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.overlay_mode_widget.borrow() {
            content.unparent();
            widget.style_context().remove_class("mode-overlay");
        }
        widget.set_parent(self);
        widget.style_context().add_class("mode-overlay");
        priv_.overlay_mode_widget.replace(Some(widget.clone()));
        if let ActivityMode::Overlay = self.mode() {
            let (width, height) = util::get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger = width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(
                true,
                bigger,
                &self.local_css_context(),
                (
                    self.allocated_width() as f64 / width as f64,
                    self.allocated_height() as f64 / height as f64,
                ),
                (1.0, 1.0),
            );
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
                }
            }
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }

    pub fn minimal_mode(&self) -> Option<gtk::Widget> {
        self.imp().minimal_mode_widget.borrow().clone()
    }
    pub fn compact_mode(&self) -> Option<gtk::Widget> {
        self.imp().compact_mode_widget.borrow().clone()
    }
    pub fn expanded_mode(&self) -> Option<gtk::Widget> {
        self.imp().expanded_mode_widget.borrow().clone()
    }
    pub fn overlay_mode(&self) -> Option<gtk::Widget> {
        self.imp().overlay_mode_widget.borrow().clone()
    }

    pub fn get_widget_for_mode(&self, mode: ActivityMode) -> Option<gtk::Widget> {
        match mode {
            ActivityMode::Minimal => self.minimal_mode().clone(),
            ActivityMode::Compact => self.compact_mode().clone(),
            ActivityMode::Expanded => self.expanded_mode().clone(),
            ActivityMode::Overlay => self.overlay_mode().clone(),
        }
    }
    pub fn current_widget(&self) -> Option<gtk::Widget> {
        match self.mode() {
            ActivityMode::Minimal => self.minimal_mode().clone(),
            ActivityMode::Compact => self.compact_mode().clone(),
            ActivityMode::Expanded => self.expanded_mode().clone(),
            ActivityMode::Overlay => self.overlay_mode().clone(),
        }
    }

    pub fn set_transition_duration(&self, duration_millis: u64, module: bool) -> Result<()> {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_transition_duration(duration_millis, module)
    }
    pub fn set_minimal_height(&self, height: i32, module: bool) -> Result<()> {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_minimal_height(height, module)
    }

    crate::implement_set_transition!(pub, local_css_context, transition_size);
    crate::implement_set_transition!(
        pub,
        local_css_context,
        transition_bigger_blur,
        [
            "minimal-blur",
            "compact-blur",
            "expanded-blur",
            "overlay-blur"
        ]
    );
    crate::implement_set_transition!(
        pub,
        local_css_context,
        transition_bigger_stretch,
        [
            "minimal-stretch-x",
            "minimal-stretch-y",
            "compact-stretch-x",
            "compact-stretch-y",
            "expanded-stretch-x",
            "expanded-stretch-y",
            "overlay-stretch-x",
            "overlay-stretch-y"
        ]
    );
    crate::implement_set_transition!(
        pub,
        local_css_context,
        transition_bigger_opacity,
        [
            "minimal-opacity",
            "compact-opacity",
            "expanded-opacity",
            "overlay-opacity"
        ]
    );
    crate::implement_set_transition!(pub, local_css_context, transition_smaller_blur);
    crate::implement_set_transition!(pub, local_css_context, transition_smaller_stretch);
    crate::implement_set_transition!(pub, local_css_context, transition_smaller_opacity);
}

// add/remove bg_widget and expose info to GTK debugger
impl ContainerImpl for ActivityWidgetPriv {
    fn add(&self, widget: &gtk::Widget) {
        if let Some(bg_widget) = &*self.background_widget.borrow() {
            bg_widget
                .style_context()
                .remove_class("activity-background");
            bg_widget.unparent();
        }
        widget.set_parent(self.obj().as_ref());
        widget.style_context().add_class("activity-background");
        self.background_widget.replace(Some(widget.clone()));
    }

    fn remove(&self, widget: &gtk::Widget) {
        if let Some(bg_widget) = &*self.background_widget.borrow() {
            if bg_widget != widget {
                glib::g_warning!("warning", "{widget} was not inside this container");
            } else {
                bg_widget
                    .style_context()
                    .remove_class("activity-background");
                bg_widget.unparent();
            }
        }
    }

    fn forall(&self, _: bool, callback: &gtk::subclass::container::Callback) {
        if let Some(content) = &*self.background_widget.borrow() {
            callback.call(content);
        }
        if let Some(content) = &*self.minimal_mode_widget.borrow() {
            callback.call(content);
        }
        if let Some(content) = &*self.compact_mode_widget.borrow() {
            callback.call(content);
        }
        if let Some(content) = &*self.expanded_mode_widget.borrow() {
            callback.call(content);
        }
        if let Some(content) = &*self.overlay_mode_widget.borrow() {
            callback.call(content);
        }
    }

    fn child_type(&self) -> glib::Type {
        match &*self.background_widget.borrow() {
            Some(_) => glib::Type::UNIT,
            None => gtk::Widget::static_type(),
        }
    }
}
