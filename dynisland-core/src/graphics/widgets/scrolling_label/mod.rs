pub mod imp;
pub mod local_css_context;

use gtk::{prelude::*, subclass::prelude::*};

use crate::graphics::util::CssSize;

glib::wrapper! {
    pub struct ScrollingLabel(ObjectSubclass<imp::ScrollingLabelPriv>)
        @extends gtk::Widget;
}

impl Default for ScrollingLabel {
    fn default() -> Self {
        let sel = glib::Object::new::<Self>();
        sel.set_overflow(gtk::Overflow::Hidden);
        sel
    }
}

impl ScrollingLabel {
    pub fn new(text: Option<&str>) -> Self {
        let label = Self::default();
        label.imp().label.borrow().set_text(text.unwrap_or(""));
        label
    }
    pub fn set_fade_size(&self, size: CssSize, user: bool) {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_config_fade_size(size, user);
    }
    pub fn set_scroll_speed(&self, speed: f32, user: bool) {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_config_speed(speed, user);
    }
    pub fn set_delay(&self, delay: u64, user: bool) {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_config_delay(delay, user);
    }
}
