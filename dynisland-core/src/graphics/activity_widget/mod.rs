// pub mod allocate_and_draw;
pub mod boxed_activity_mode;
pub mod imp;
pub mod layout_manager;
pub mod local_css_context;

use gtk::{prelude::*, subclass::prelude::*};

use self::boxed_activity_mode::ActivityMode;

use super::util;

glib::wrapper! {
    pub struct ActivityWidget(ObjectSubclass<imp::ActivityWidgetPriv>)
        @extends gtk::Widget;
        // @implements gtk::Accessible;
}

impl Default for ActivityWidget {
    fn default() -> Self {
        let sel = glib::Object::new::<Self>();
        sel.set_overflow(gtk::Overflow::Hidden);

        sel
    }
}

impl ActivityWidget {
    pub fn new(name: &str) -> Self {
        let wid = Self::default();
        // wid.set_has_window(false);
        wid.set_name(name);

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &wid.local_css_context().get_css_provider(),
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1, //needs to be higher than user proprity
        );
        wid
    }

    pub fn set_minimal_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.minimal_mode_widget.borrow() {
            content.unparent();
            content.remove_css_class("mode-minimal");
        }

        widget.set_parent(self);
        widget.add_css_class("mode-minimal");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.minimal_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_config_minimal_height();
        let widget_size = util::get_final_widget_size(widget, self.mode(), min_height);
        if let ActivityMode::Minimal = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            widget.insert_before(self, Option::None::<&gtk::Widget>); //put at the end of the list so it recieves the inputs
        } else {
            let current_size = self
                .imp()
                .get_final_widget_size_for_mode(self.mode(), min_height);
            self.local_css_context().set_stretch(
                ActivityMode::Minimal,
                (
                    current_size.0 / widget_size.0 as f64,
                    current_size.1 / widget_size.1 as f64,
                ),
            );
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }

    pub fn set_compact_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.compact_mode_widget.borrow() {
            content.unparent();
            content.remove_css_class("mode-compact");
        }
        widget.set_parent(self);
        widget.add_css_class("mode-compact");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.compact_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_config_minimal_height();
        let widget_size = util::get_final_widget_size(widget, self.mode(), min_height);
        if let ActivityMode::Compact = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            widget.insert_before(self, Option::None::<&gtk::Widget>); //put at the end of the list so it recieves the inputs
        } else {
            let current_size = self
                .imp()
                .get_final_widget_size_for_mode(self.mode(), min_height);
            self.local_css_context().set_stretch(
                ActivityMode::Compact,
                (
                    current_size.0 / widget_size.0 as f64,
                    current_size.1 / widget_size.1 as f64,
                ),
            );
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }

    pub fn set_expanded_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.expanded_mode_widget.borrow() {
            content.unparent();
            content.remove_css_class("mode-expanded");
        }
        widget.set_parent(self);
        widget.add_css_class("mode-expanded");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.expanded_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_config_minimal_height();
        let widget_size = util::get_final_widget_size(widget, self.mode(), min_height);
        if let ActivityMode::Expanded = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            widget.insert_before(self, Option::None::<&gtk::Widget>); //put at the end of the list so it recieves the inputs
        } else {
            let current_size = self
                .imp()
                .get_final_widget_size_for_mode(self.mode(), min_height);
            self.local_css_context().set_stretch(
                ActivityMode::Expanded,
                (
                    current_size.0 / widget_size.0 as f64,
                    current_size.1 / widget_size.1 as f64,
                ),
            );
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }

    pub fn set_overlay_mode(&self, widget: &gtk::Widget) {
        let priv_ = self.imp();
        if let Some(content) = &*priv_.overlay_mode_widget.borrow() {
            content.unparent();
            content.remove_css_class("mode-overlay");
        }
        widget.set_parent(self);
        widget.add_css_class("mode-overlay");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.overlay_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_config_minimal_height();
        let widget_size = util::get_final_widget_size(widget, self.mode(), min_height);
        if let ActivityMode::Overlay = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            widget.insert_before(self, Option::None::<&gtk::Widget>); //put at the end of the list so it recieves the inputs
        } else {
            let current_size = self
                .imp()
                .get_final_widget_size_for_mode(self.mode(), min_height);
            self.local_css_context().set_stretch(
                ActivityMode::Overlay,
                (
                    current_size.0 / widget_size.0 as f64,
                    current_size.1 / widget_size.1 as f64,
                ),
            );
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

    pub fn set_minimal_height(&self, height: i32, module: bool) {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_config_minimal_height(height, module);
        self.imp().config_minimal_height_app.replace(height);
    }
    pub fn get_minimal_height(&self) -> i32 {
        self.imp()
            .local_css_context
            .borrow()
            .get_config_minimal_height()
    }

    pub fn set_blur_radius(&self, radius: f64, module: bool) {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_config_blur_radius(radius, module);
        self.imp().config_blur_radius_app.replace(radius);
    }
    pub fn get_blur_radius(&self) -> f64 {
        self.imp()
            .local_css_context
            .borrow()
            .get_config_blur_radius()
    }
}
