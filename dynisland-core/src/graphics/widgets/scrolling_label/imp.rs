use std::cell::RefCell;

use gdk::RGBA;
use glib_macros::Properties;
use gtk::{
    graphene::{Point, Rect},
    gsk::ColorStop,
    prelude::*,
    subclass::prelude::*,
};

use super::{local_css_context::ScrollingLabelLocalCssContext, ScrollingLabel};

//TODO implement vertical orientation

#[derive(Properties)]
#[properties(wrapper_type = ScrollingLabel)]
pub struct ScrollingLabelPriv {
    #[property(get, nick = "Local CSS Provider")]
    pub(super) local_css_context: RefCell<ScrollingLabelLocalCssContext>,
    bin: RefCell<gtk::Box>,
    #[property(get, nick = "Internal Label")]
    pub(super) label: RefCell<gtk::Label>,
    #[property(get, nick = "Active scrolling")]
    active: RefCell<bool>,
}

impl Default for ScrollingLabelPriv {
    fn default() -> Self {
        ScrollingLabelPriv {
            bin: RefCell::new(gtk::Box::new(gtk::Orientation::Horizontal, 0)),
            label: RefCell::new(gtk::Label::new(Some(""))),
            local_css_context: RefCell::new(ScrollingLabelLocalCssContext::default()),
            active: RefCell::new(false),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ScrollingLabelPriv {
    const NAME: &'static str = "ScrollingLabel";
    type Type = super::ScrollingLabel;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // klass.set_layout_manager_type::<BinLayout>();

        klass.set_css_name("scrolling-label");
    }
}

impl ScrollingLabelPriv {
    // fn set_active(&self, active: bool) {
    //     self.active.replace(active);
    // }
}

#[glib::derived_properties]
impl ObjectImpl for ScrollingLabelPriv {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj()
            .add_css_class(self.local_css_context.borrow().get_name());
        let bin = self.bin.borrow();

        let label = self.label.borrow().clone();
        // label.set_parent(self.obj().as_ref());
        label.set_wrap(false);
        label.set_halign(gtk::Align::Start);
        label.set_valign(gtk::Align::Center);
        label.add_css_class("inner-label");
        bin.append(&label);
        bin.set_parent(self.obj().as_ref());
    }

    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }

    fn dispose(&self) {
        let label = self.label.borrow();
        label.unparent();
    }
}

impl WidgetImpl for ScrollingLabelPriv {
    fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let bin = self.bin.borrow();
        let mut measure = bin.measure(orientation, for_size);
        match orientation {
            gtk::Orientation::Horizontal => {
                measure.0 = 0;
                measure.2 = measure.2.clamp(-1, measure.0);
            }
            gtk::Orientation::Vertical => {
                // measure.0= 10;
                // measure.2= 0;
            }
            _ => {}
        }
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
        if self.obj().width() < label.width() {
            self.local_css_context
                .borrow_mut()
                .set_active(true, bin.width());
        } else {
            self.local_css_context.borrow_mut().set_active(false, 0);
        }
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let bin = self.bin.borrow().clone();
        let active = self.local_css_context.borrow().get_active();
        if active {
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
        obj.snapshot_child(&bin, snapshot);
        if active {
            let width = bin.width();
            snapshot.save();
            snapshot.translate(&Point::new(width as f32, 0.0));
            obj.snapshot_child(&bin, snapshot);
            snapshot.restore();
            snapshot.pop();
        }
    }
}