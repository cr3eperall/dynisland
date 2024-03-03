// pub mod allocate_and_draw;
pub mod boxed_activity_mode;
pub mod imp;
pub mod layout_manager;
pub mod local_css_context;
mod object_subclass_impl;

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

    pub fn get_widget_for_mode(&self, mode: ActivityMode) -> Option<gtk::Widget> {
        match mode {
            ActivityMode::Minimal => self.minimal_mode_widget(),
            ActivityMode::Compact => self.compact_mode_widget(),
            ActivityMode::Expanded => self.expanded_mode_widget(),
            ActivityMode::Overlay => self.overlay_mode_widget(),
        }
    }

    pub fn current_widget(&self) -> Option<gtk::Widget> {
        self.get_widget_for_mode(self.mode())
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
