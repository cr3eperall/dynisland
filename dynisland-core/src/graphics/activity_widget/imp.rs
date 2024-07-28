use rand::{distributions::Alphanumeric, Rng};
use std::cell::RefCell;

use glib::prelude::*;
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*};

use super::{
    boxed_activity_mode::ActivityMode, local_css_context::ActivityWidgetLocalCssContext, util,
    ActivityWidget,
};
#[derive(Properties)]
#[properties(wrapper_type = ActivityWidget)]
pub struct ActivityWidgetPriv {
    #[property(get, set, nick = "Change mode", blurb = "The Activity Mode")]
    pub(super) mode: RefCell<ActivityMode>,

    #[property(get, nick = "Local CSS Provider")]
    pub(super) local_css_context: RefCell<ActivityWidgetLocalCssContext>,

    #[property(get, set, nick = "Widget name")]
    pub(super) name: RefCell<String>,

    /// To be used by dynisland::app only
    #[property(set, nick = "Minimal height")]
    pub(super) config_minimal_height_app: RefCell<i32>,

    /// To be used by dynisland::app only
    #[property(set, nick = "Minimal height")]
    pub(super) config_minimal_width_app: RefCell<i32>,

    /// To be used by dynisland::app only
    #[property(set, nick = "Transition blur radius")]
    pub(super) config_blur_radius_app: RefCell<f64>,

    pub(super) last_mode: RefCell<ActivityMode>,

    // pub(super) transition_manager: RefCell<TransitionManager>,
    pub(super) background_widget: RefCell<Option<gtk::Widget>>,
    #[property(get, set, nick = "Minimal Mode Widget")]
    pub(super) minimal_mode_widget: RefCell<Option<gtk::Widget>>,
    #[property(get, set, nick = "Compact Mode Widget")]
    pub(super) compact_mode_widget: RefCell<Option<gtk::Widget>>,
    #[property(get, set, nick = "Expanded Mode Widget")]
    pub(super) expanded_mode_widget: RefCell<Option<gtk::Widget>>,
    #[property(get, set, nick = "Overlay Mode Widget")]
    pub(super) overlay_mode_widget: RefCell<Option<gtk::Widget>>,
}

//default data
impl Default for ActivityWidgetPriv {
    fn default() -> Self {
        let name: String = "c"
            .chars()
            .chain(
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(6)
                    .map(char::from),
            )
            .collect::<String>();

        let css_ctx = ActivityWidgetLocalCssContext::new(&name);
        let min_h = css_ctx.get_config_minimal_height();
        let min_w = css_ctx.get_config_minimal_width();
        let blur = css_ctx.get_config_blur_radius();
        Self {
            mode: RefCell::new(ActivityMode::Minimal),
            local_css_context: RefCell::new(css_ctx),
            config_minimal_height_app: RefCell::new(min_h),
            config_minimal_width_app: RefCell::new(min_w),
            config_blur_radius_app: RefCell::new(blur),
            last_mode: RefCell::new(ActivityMode::Minimal),
            name: RefCell::new(name),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for ActivityWidgetPriv {
    fn constructed(&self) {
        self.parent_constructed();

        let label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .build();
        let background = gtk::Box::builder()
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Center)
            .vexpand(true)
            .hexpand(true)
            .build();
        background.append(&label);
        background.add_css_class("activity-background");

        background.set_parent(&*self.obj());
        self.background_widget
            .replace(Some(background.upcast::<gtk::Widget>()));
    }

    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "mode" => {
                // Replace old values if the mode is valid
                let mode = value.get().unwrap();

                if self.get_mode_widget(mode).borrow().is_none() {
                    return;
                }
                self.last_mode.replace(*self.mode.borrow());
                self.mode.replace(mode);

                // let last_mode = *self.last_mode.borrow();

                let mut css_context = self.local_css_context.borrow_mut();
                let min_height = css_context.get_config_minimal_height();
                let min_width = css_context.get_config_minimal_width();

                let next_size = self.get_final_widget_size_for_mode(mode, min_height, min_width);
                // log::debug!("next_size: {:?}", next_size);
                // let prev_size=self.get_final_allocation_for_mode(last_mode, min_height);

                // TODO add css classes {active, bigger, smaller, last...} to the widgets accordingly
                // let bigger = next_size.0 > prev_size.0 || next_size.1 > prev_size.1;

                // Set properties to start the css transition

                css_context.set_opacity_all(util::get_property_slice_for_mode_f64(mode, 1.0, 0.0));

                let blur_radius = css_context.get_config_blur_radius();
                css_context.set_blur_all(util::get_property_slice_for_mode_f64(
                    mode,
                    0.0,
                    blur_radius,
                ));

                let stretches = self.get_stretches(next_size, min_height, min_width);
                log::trace!("stretches: {:?}", stretches);
                css_context.set_stretch_all(stretches);

                if let Some(next) = self.get_mode_widget(mode).borrow().as_ref() {
                    //put at the end so it recieves the inputs
                    next.insert_before(self.obj().as_ref(), Option::None::<&gtk::Widget>);
                    css_context.set_size((next_size.0 as i32, next_size.1 as i32));
                };
                self.obj().queue_draw(); // Queue a draw call with the updated value
            }
            "name" => {
                self.obj().remove_css_class(&self.name.borrow());

                self.name.replace(value.get().unwrap());
                self.local_css_context
                    .borrow_mut()
                    .set_name(value.get().unwrap());
                self.obj().add_css_class(value.get().unwrap());
            }
            "config-minimal-height-app" => {
                // self.config_minimal_height_app.replace(value.get().unwrap());
                self.local_css_context
                    .borrow_mut()
                    .set_config_minimal_height(value.get().unwrap(), false);
            }
            "config-minimal-width-app" => {
                // self.config_minimal_height_app.replace(value.get().unwrap());
                self.local_css_context
                    .borrow_mut()
                    .set_config_minimal_width(value.get().unwrap(), false);
            }
            "config-blur-radius-app" => {
                // self.config_blur_radius_app.replace(value.get().unwrap());
                self.local_css_context
                    .borrow_mut()
                    .set_config_blur_radius(value.get().unwrap(), false);
            }
            "minimal-mode-widget" => {
                let widget: Option<gtk::Widget> = value.get().unwrap();
                if let Some(content) = &*self.minimal_mode_widget.borrow() {
                    content.unparent();
                    content.remove_css_class("mode-minimal");
                }
                self.minimal_mode_widget.replace(widget);
                if let Some(widget) = self.minimal_mode_widget.borrow().as_ref() {
                    widget.set_parent(&self.obj().clone().upcast::<gtk::Widget>());
                    widget.add_css_class("mode-minimal");
                    widget.set_overflow(gtk::Overflow::Hidden);
                }

                self.obj().set_mode(self.obj().mode()); //update the size and the position of the widget
                self.obj().queue_draw(); // Queue a draw call with the updated widget
            }
            "compact-mode-widget" => {
                let widget: Option<gtk::Widget> = value.get().unwrap();
                if let Some(content) = &*self.compact_mode_widget.borrow() {
                    content.unparent();
                    content.remove_css_class("mode-compact");
                }
                self.compact_mode_widget.replace(widget);
                if let Some(widget) = self.compact_mode_widget.borrow().as_ref() {
                    widget.set_parent(&self.obj().clone().upcast::<gtk::Widget>());
                    widget.add_css_class("mode-compact");
                    widget.set_overflow(gtk::Overflow::Hidden);
                }

                self.obj().set_mode(self.obj().mode()); //update the size and the position of the widget
                self.obj().queue_draw(); // Queue a draw call with the updated widget
            }
            "expanded-mode-widget" => {
                let widget: Option<gtk::Widget> = value.get().unwrap();
                if let Some(content) = &*self.expanded_mode_widget.borrow() {
                    content.unparent();
                    content.remove_css_class("mode-expanded");
                }
                self.expanded_mode_widget.replace(widget);
                if let Some(widget) = self.expanded_mode_widget.borrow().as_ref() {
                    widget.set_parent(&self.obj().clone().upcast::<gtk::Widget>());
                    widget.add_css_class("mode-expanded");
                    widget.set_overflow(gtk::Overflow::Hidden);
                }

                self.obj().set_mode(self.obj().mode()); //update the size and the position of the widget
                self.obj().queue_draw(); // Queue a draw call with the updated widget
            }
            "overlay-mode-widget" => {
                let widget: Option<gtk::Widget> = value.get().unwrap();
                if let Some(content) = &*self.overlay_mode_widget.borrow() {
                    content.unparent();
                    content.remove_css_class("mode-overlay");
                }
                self.overlay_mode_widget.replace(widget);
                if let Some(widget) = self.overlay_mode_widget.borrow().as_ref() {
                    widget.set_parent(&self.obj().clone().upcast::<gtk::Widget>());
                    widget.add_css_class("mode-overlay");
                    widget.set_overflow(gtk::Overflow::Hidden);
                }

                self.obj().set_mode(self.obj().mode()); //update the size and the position of the widget
                self.obj().queue_draw(); // Queue a draw call with the updated widget
            }

            x => panic!("Tried to set inexistant property of ActivityWidget: {}", x),
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }

    fn dispose(&self) {
        if let Some(widget) = self.background_widget.borrow_mut().take() {
            widget.unparent();
        }
        if let Some(widget) = self.minimal_mode_widget.borrow_mut().take() {
            widget.unparent();
        }
        if let Some(widget) = self.compact_mode_widget.borrow_mut().take() {
            widget.unparent();
        }
        if let Some(widget) = self.expanded_mode_widget.borrow_mut().take() {
            widget.unparent();
        }
        if let Some(widget) = self.overlay_mode_widget.borrow_mut().take() {
            widget.unparent();
        }
    }
}

impl WidgetImpl for ActivityWidgetPriv {}

impl ActivityWidgetPriv {
    pub(super) fn get_mode_widget(&self, mode: ActivityMode) -> &RefCell<Option<gtk::Widget>> {
        match mode {
            ActivityMode::Minimal => &self.minimal_mode_widget,
            ActivityMode::Compact => &self.compact_mode_widget,
            ActivityMode::Expanded => &self.expanded_mode_widget,
            ActivityMode::Overlay => &self.overlay_mode_widget,
        }
    }

    pub(super) fn get_final_widget_size_for_mode(
        &self,
        mode: ActivityMode,
        min_height: i32,
        min_width: i32,
    ) -> (f64, f64) {
        if let Some(widget) = &*self.get_mode_widget(mode).borrow() {
            let tmp =
                util::get_final_widget_size(widget, *self.mode.borrow(), min_height, min_width);
            (tmp.0 as f64, tmp.1 as f64)
        } else {
            (
                // default
                self.obj().width() as f64,
                self.obj().height() as f64,
            )
        }
    }

    pub(super) fn get_stretches(
        &self,
        next_size: (f64, f64),
        min_height: i32,
        min_width: i32,
    ) -> [(f64, f64); 4] {
        let mut mode = ActivityMode::Minimal;
        let min_stretch = if matches!(*self.mode.borrow(), ActivityMode::Minimal) {
            (1.0, 1.0)
        } else {
            let min_alloc = if let Some(widget) = &*self.get_mode_widget(mode).borrow() {
                let mut measure = util::get_child_aligned_allocation(
                    (next_size.0 as i32, next_size.1 as i32, -1),
                    widget,
                    mode,
                    min_height,
                );
                if measure.0 == 0 {
                    measure.0 = next_size.0 as i32;
                }
                if measure.1 == 0 {
                    measure.1 = next_size.1 as i32;
                }
                (measure.0 as f64, measure.1 as f64)
            } else {
                self.get_final_widget_size_for_mode(mode, min_height, min_width)
            };
            // log::debug!("min get_size: {:?}, alloc: {:?}", min_alloc, min_alloc);
            (next_size.0 / min_alloc.0, next_size.1 / min_alloc.1)
        };

        mode = ActivityMode::Compact;
        let com_stretch = if matches!(*self.mode.borrow(), ActivityMode::Compact) {
            (1.0, 1.0)
        } else {
            let com_alloc = if let Some(widget) = &*self.get_mode_widget(mode).borrow() {
                let mut measure = util::get_child_aligned_allocation(
                    (next_size.0 as i32, next_size.1 as i32, -1),
                    widget,
                    mode,
                    min_height,
                );
                if measure.0 == 0 {
                    measure.0 = next_size.0 as i32;
                }
                if measure.1 == 0 {
                    measure.1 = next_size.1 as i32;
                }
                (measure.0 as f64, measure.1 as f64)
            } else {
                self.get_final_widget_size_for_mode(mode, min_height, min_width)
            };
            // log::debug!("min get_size: {:?}, alloc: {:?}", min_alloc, min_alloc);
            (next_size.0 / com_alloc.0, next_size.1 / com_alloc.1)
        };

        mode = ActivityMode::Expanded;
        let exp_stretch = if matches!(*self.mode.borrow(), ActivityMode::Expanded) {
            (1.0, 1.0)
        } else {
            let exp_alloc = if let Some(widget) = &*self.get_mode_widget(mode).borrow() {
                let mut measure = util::get_child_aligned_allocation(
                    (next_size.0 as i32, next_size.1 as i32, -1),
                    widget,
                    mode,
                    min_height,
                );
                if measure.0 == 0 {
                    measure.0 = next_size.0 as i32;
                }
                if measure.1 == 0 {
                    measure.1 = next_size.1 as i32;
                }
                (measure.0 as f64, measure.1 as f64)
            } else {
                self.get_final_widget_size_for_mode(mode, min_height, min_width)
            };
            // log::debug!("min get_size: {:?}, alloc: {:?}", min_alloc, min_alloc);
            (next_size.0 / exp_alloc.0, next_size.1 / exp_alloc.1)
        };

        mode = ActivityMode::Overlay;
        let ove_stretch = if matches!(*self.mode.borrow(), ActivityMode::Overlay) {
            (1.0, 1.0)
        } else {
            let ove_alloc = if let Some(widget) = &*self.get_mode_widget(mode).borrow() {
                let mut measure = util::get_child_aligned_allocation(
                    (next_size.0 as i32, next_size.1 as i32, -1),
                    widget,
                    mode,
                    min_height,
                );
                if measure.0 == 0 {
                    measure.0 = next_size.0 as i32;
                }
                if measure.1 == 0 {
                    measure.1 = next_size.1 as i32;
                }
                (measure.0 as f64, measure.1 as f64)
            } else {
                self.get_final_widget_size_for_mode(mode, min_height, min_width)
            };
            // log::debug!("min get_size: {:?}, alloc: {:?}", min_alloc, min_alloc);
            (next_size.0 / ove_alloc.0, next_size.1 / ove_alloc.1)
        };

        [
            (min_stretch.0, min_stretch.1),
            (com_stretch.0, com_stretch.1),
            (exp_stretch.0, exp_stretch.1),
            (ove_stretch.0, ove_stretch.1),
        ]
    }
}