pub mod imp;
pub mod local_css_context;
mod object_subclass_impl;

glib::wrapper! {
    /// A Label with a max width (`width_request`) that scrolls when the inner label exceeds the max width
    /// NOTE: i don't know why but this widget makes gtk crash if it's in an horizontal box with another widget
    pub struct ScrollingLabel(ObjectSubclass<imp::ScrollingLabelPriv>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}

impl Default for ScrollingLabel {
    fn default() -> Self {
        // sel.set_overflow(gtk::Overflow::Hidden);
        glib::Object::new::<Self>()
    }
}

impl ScrollingLabel {
    pub fn new() -> Self {
        Self::default()
        // label.imp().label.borrow().set_text(text.unwrap_or(""));
    }
    // pub fn set_fade_size(&self, size: CssSize, user: bool) {
    //     self.imp()
    //         .local_css_context
    //         .borrow_mut()
    //         .set_config_fade_size(size, user);
    // }
    // pub fn set_scroll_speed(&self, speed: f32, user: bool) {
    //     self.imp()
    //         .local_css_context
    //         .borrow_mut()
    //         .set_config_speed(speed, user);
    // }
    // pub fn set_delay(&self, delay: u64, user: bool) {
    //     self.imp()
    //         .local_css_context
    //         .borrow_mut()
    //         .set_config_delay(delay, user);
    // }
}
