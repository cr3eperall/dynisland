// pub mod allocate_and_draw;
pub mod boxed_activity_mode;
pub mod imp;
pub mod layout_manager;
pub mod local_css_context;
mod object_subclass_impl;

use gtk::prelude::*;

use self::boxed_activity_mode::ActivityMode;

use super::util;

glib::wrapper! {
    /// A Widget containing from 1 to 4 Widgets, one for each mode.
    /// It should contain at least the Minimal widget.
    ///
    /// It also stretches on drag if enabled
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
}
