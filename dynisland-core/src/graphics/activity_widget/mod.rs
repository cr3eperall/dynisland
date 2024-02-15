// pub mod allocate_and_draw;
pub mod imp;
pub mod layout_manager;
pub mod local_css_context;

use gtk::{prelude::*, subclass::prelude::*};

use self::imp::ActivityMode;

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
            1000, //needs to be higher than user proprity
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
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.minimal_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_minimal_height();
        let widget_size = util::get_final_widget_size(widget, self.mode(), min_height);
        if let ActivityMode::Minimal = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            // TODO fix
            // match widget.window() {
            //     //raise window associated to widget if it has one, this enables events on the active mode widget
            //     Some(window) => window.raise(),
            //     None => {
            //         // debug!("no window");
            //     }
            // }
        } else {
            let current_size = self
                .imp()
                .get_final_allocation_for_mode(self.mode(), min_height);
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
            content.style_context().remove_class("mode-compact");
        }
        widget.set_parent(self);
        widget.style_context().add_class("mode-compact");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.compact_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_minimal_height();
        let widget_size = util::get_final_widget_size(
            widget,
            self.mode(),
            self.local_css_context().get_minimal_height(),
        );
        if let ActivityMode::Compact = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            // TODO fix
            // match widget.window() {
            //     //raise window associated to widget if it has one, this enables events on the active mode widget
            //     Some(window) => window.raise(),
            //     None => {
            //         // debug!("no window");
            //     }
            // }
        } else {
            let current_size = self
                .imp()
                .get_final_allocation_for_mode(self.mode(), min_height);
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
            content.style_context().remove_class("mode-expanded");
        }
        widget.set_parent(self);
        widget.style_context().add_class("mode-expanded");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.expanded_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_minimal_height();
        let widget_size = util::get_final_widget_size(
            widget,
            self.mode(),
            self.local_css_context().get_minimal_height(),
        );
        if let ActivityMode::Expanded = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            // TODO fix
            // match widget.window() {
            //     //raise window associated to widget if it has one, this enables events on the active mode widget
            //     Some(window) => window.raise(),
            //     None => {
            //         // debug!("no window");
            //     }
            // }
        } else {
            let current_size = self
                .imp()
                .get_final_allocation_for_mode(self.mode(), min_height);
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
            content.style_context().remove_class("mode-overlay");
        }
        widget.set_parent(self);
        widget.style_context().add_class("mode-overlay");
        widget.set_overflow(gtk::Overflow::Hidden);
        priv_.overlay_mode_widget.replace(Some(widget.clone()));
        let min_height = self.local_css_context().get_minimal_height();
        let widget_size = util::get_final_widget_size(
            widget,
            self.mode(),
            self.local_css_context().get_minimal_height(),
        );
        if let ActivityMode::Overlay = self.mode() {
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size(widget_size);

            // TODO fix
            // match widget.window() {
            //     //raise window associated to widget if it has one, this enables events on the active mode widget
            //     Some(window) => window.raise(),
            //     None => {
            //         // debug!("no window");
            //     }
            // }
        } else {
            let current_size = self
                .imp()
                .get_final_allocation_for_mode(self.mode(), min_height);
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
            .set_minimal_height(height, module);
    }
    pub fn get_minimal_height(&self) -> i32 {
        self.imp().local_css_context.borrow().get_minimal_height()
    }

    pub fn set_blur_radius(&self, radius: f64, module: bool) {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_blur_radius(radius, module);
    }
    pub fn get_blur_radius(&self) -> f64 {
        self.imp().local_css_context.borrow().get_blur_radius()
    }
}

pub mod util {
    use gtk::prelude::WidgetExt;

    use super::imp::ActivityMode;

    pub(super) fn get_final_widget_size(
        widget: &gtk::Widget,
        mode: ActivityMode,
        minimal_height: i32,
    ) -> (i32, i32) {
        let force_height = matches!(mode, ActivityMode::Minimal | ActivityMode::Compact);
        let measured_width = widget.measure(
            gtk::Orientation::Horizontal,
            if force_height { minimal_height } else { -1 },
        );
        let measured_height = widget.measure(gtk::Orientation::Vertical, -1);
        let height = if force_height {
            minimal_height
        } else if widget.height_request() > 0 {
            widget.height_request()
        } else {
            measured_height.1
        };
        let width = if widget.width_request() > 0 {
            widget.width_request()
        } else {
            measured_width.1
        };
        (width.max(minimal_height), height.max(minimal_height))
    }

    pub(super) fn get_property_slice_for_mode_f64(
        mode: ActivityMode,
        mode_value: f64,
        other_values: f64,
    ) -> [f64; 4] {
        match mode {
            ActivityMode::Minimal => [mode_value, other_values, other_values, other_values],
            ActivityMode::Compact => [other_values, mode_value, other_values, other_values],
            ActivityMode::Expanded => [other_values, other_values, mode_value, other_values],
            ActivityMode::Overlay => [other_values, other_values, other_values, mode_value],
        }
    }
}
