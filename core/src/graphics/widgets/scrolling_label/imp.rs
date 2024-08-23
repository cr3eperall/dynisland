use std::{cell::RefCell, str::FromStr};

use gdk::RGBA;
use glib_macros::Properties;
use gtk::{
    graphene::{Point, Rect},
    gsk::ColorStop,
    prelude::*,
    subclass::prelude::*,
};

use crate::graphics::util::CssSize;

use super::{local_css_context::ScrollingLabelLocalCssContext, ScrollingLabel};

//TODO implement vertical orientation and builder

#[derive(Properties)]
#[properties(wrapper_type = ScrollingLabel)]
pub struct ScrollingLabelPriv {
    // #[property(get, nick = "Local CSS Provider")]
    pub(super) local_css_context: RefCell<ScrollingLabelLocalCssContext>,

    bin: RefCell<gtk::Box>,

    #[property(get, set, nick = "Internal Label")]
    pub(super) label: RefCell<gtk::Label>,

    #[property(get, set, nick = "Text")]
    pub(super) text: RefCell<String>,

    #[property(get, nick = "Active scrolling")]
    pub(super) active: RefCell<bool>,

    #[property(get, set, nick = "Side fade size in px or %")]
    pub(super) config_fade_size: RefCell<String>,

    #[property(get, set, nick = "Scrolling speed")]
    pub(super) config_scroll_speed: RefCell<f32>,

    #[property(
        get,
        set,
        nick = "Time from when the animation stops to when it begins again"
    )]
    pub(super) config_delay: RefCell<u64>,
}

impl Default for ScrollingLabelPriv {
    fn default() -> Self {
        ScrollingLabelPriv {
            bin: RefCell::new(gtk::Box::new(gtk::Orientation::Horizontal, 0)),
            label: RefCell::new(gtk::Label::new(Some(""))),
            text: RefCell::new(String::new()),
            local_css_context: RefCell::new(ScrollingLabelLocalCssContext::default()),
            active: RefCell::new(false),
            config_fade_size: RefCell::new("4%".to_string()),
            config_scroll_speed: RefCell::new(40.0),
            config_delay: RefCell::new(5000),
        }
    }
}

impl ScrollingLabelPriv {}

#[glib::derived_properties]
impl ObjectImpl for ScrollingLabelPriv {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj()
            .add_css_class(self.local_css_context.borrow().get_name());
        let bin = self.bin.borrow();

        let label = self.label.borrow();
        let label: &gtk::Label = label.as_ref();
        // label.set_parent(self.obj().as_ref());
        label.set_wrap(false);
        label.set_halign(gtk::Align::Start);
        label.set_valign(gtk::Align::Center);
        label.add_css_class("inner-label");
        label.set_text("");
        bin.append(label);
        bin.set_parent(self.obj().as_ref());

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            self.local_css_context.borrow().get_css_provider(),
            gtk::STYLE_PROVIDER_PRIORITY_USER - 1,
        );
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "label" => {
                let old_label = &*self.label.borrow();
                self.bin.borrow_mut().remove(old_label);
                old_label.remove_css_class("inner-label");

                let new_label: gtk::Label = value.get().unwrap();
                new_label.add_css_class("inner-label");
                new_label.set_wrap(false);
                new_label.set_halign(gtk::Align::Start);
                new_label.set_valign(gtk::Align::Center);

                self.bin.borrow_mut().append(&new_label);
                self.label.replace(new_label);

                self.obj().queue_draw();
            }
            "text" => {
                let label = self.label.borrow();
                let text: String = value.get().unwrap();
                if label.label() == text {
                    return;
                }
                label.set_text(&text);
                self.text.replace(text);

                self.obj().queue_allocate();
                self.obj().queue_draw();
            }
            "config-fade-size" => {
                let value: String = value.get().unwrap();
                self.local_css_context.borrow_mut().set_config_fade_size(
                    CssSize::from_str(&value).unwrap_or(CssSize::Percent(4.0)),
                    false,
                );
                self.config_fade_size.replace(value);
            }
            "config-scroll-speed" => {
                let value = value.get().unwrap();
                self.config_scroll_speed.replace(value);
                self.local_css_context
                    .borrow_mut()
                    .set_config_speed(value, false);
            }
            "config-delay" => {
                let value = value.get().unwrap();
                self.config_delay.replace(value);
                self.local_css_context
                    .borrow_mut()
                    .set_config_delay(value, false);
            }
            x => panic!("Tried to set inexistant property of ScrollingLabel: {}", x),
        }
    }

    fn dispose(&self) {
        let label = self.label.borrow();
        label.unparent();
        let bin = self.bin.borrow();
        bin.unparent();
    }
}

impl WidgetImpl for ScrollingLabelPriv {
    /// If width_request is specified, that becomes the max width of the widget
    fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let bin = self.bin.borrow();
        let mut measure = bin.measure(orientation, for_size);
        match orientation {
            gtk::Orientation::Horizontal => {
                measure.0 = 0;
                if self.obj().width_request() > 0 {
                    measure.1 = measure.1.clamp(0, self.obj().width_request());
                }
                measure.2 = measure.2.clamp(-1, measure.0);
            }
            gtk::Orientation::Vertical => {
                // measure.0= 10;
                // measure.2= 0;
            }
            _ => {}
        }
        // log::info!("min: {}, nat: {}", measure.0, measure.1);
        measure
    }

    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        // self.obj().allocate(width, height, baseline, None);
        let label = self.label.borrow();
        let bin = self.bin.borrow();
        let (bin_min_width, bin_nat_width, _, _) = bin.measure(gtk::Orientation::Horizontal, -1);
        bin.allocate(
            width.clamp(bin_min_width, bin_nat_width),
            height,
            baseline,
            None,
        );

        // log::debug!("sw: {}, bw: {}, lw: {}", self.obj().width(), bin.width(), label.width());
        let width = self.obj().width();
        let fade_size = self
            .local_css_context
            .borrow()
            .get_config_fade_size()
            .get_for_size(width as f32) as i32;
        if width - 2 * fade_size < label.width() {
            self.local_css_context
                .borrow_mut()
                .set_active(true, bin.width());
            self.active.replace(true);
        } else {
            self.local_css_context.borrow_mut().set_active(false, 0);
            self.active.replace(false);
        }
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let bin = self.bin.borrow();
        let bin: &gtk::Box = bin.as_ref();
        let active = self.local_css_context.borrow().get_active();
        if active && obj.width() > 0 {
            snapshot.push_mask(gtk::gsk::MaskMode::Alpha);
            let fade_size = self.local_css_context.borrow().get_config_fade_size();
            let stop_1 = fade_size.get_for_size(obj.width() as f32);
            let stop_2 = obj.width() as f32 - stop_1;
            snapshot.append_linear_gradient(
                &Rect::new(0.0, 0.0, stop_1, obj.height() as f32),
                &Point::new(0.0, 0.0),
                &Point::new(stop_1, 0.0),
                &[
                    ColorStop::new(0.0, RGBA::BLACK.with_alpha(0.0)),
                    ColorStop::new(1.0, RGBA::BLACK.with_alpha(1.0)),
                ],
            );
            snapshot.append_color(
                &RGBA::BLACK.with_alpha(1.0),
                &Rect::new(stop_1, 0.0, stop_2 - stop_1, obj.height() as f32),
            );
            snapshot.append_linear_gradient(
                &Rect::new(stop_2, 0.0, obj.width() as f32, obj.height() as f32),
                &Point::new(stop_2, 0.0),
                &Point::new(obj.width() as f32, 0.0),
                &[
                    ColorStop::new(0.0, RGBA::BLACK.with_alpha(1.0)),
                    ColorStop::new(1.0, RGBA::BLACK.with_alpha(0.0)),
                ],
            );
            snapshot.pop();
        }
        obj.snapshot_child(bin, snapshot);
        if active {
            let width = bin.width();
            snapshot.save();
            snapshot.translate(&Point::new(width as f32, 0.0));
            obj.snapshot_child(bin, snapshot);
            snapshot.restore();
            snapshot.pop();
        }
    }
}
