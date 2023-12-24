use rand::{distributions::Alphanumeric, Rng};
use std::{
    cell::RefCell,
    f64::consts::PI,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use glib::{object_subclass, prelude::*, wrapper};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*, StyleProperties};

use crate::filters::filter::FilterBackend;

use super::{
    activity_widget_local_css_context::ActivityWidgetLocalCssContext, animations::{transition::Transition, soy::{Bezier, Lerper}},
};

pub const MINIMAL_HEIGHT: i32 = 40; //TODO move to config

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedActivityMode")]
pub enum ActivityMode {
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
    Overlay = 3,
}

wrapper! {
    pub struct ActivityWidget(ObjectSubclass<ActivityWidgetPriv>)
    @extends gtk::Container, gtk::Widget;
}

#[derive(Properties)]
#[properties(wrapper_type = ActivityWidget)]
pub struct ActivityWidgetPriv {
    #[property(get, set, nick = "Change mode", blurb = "The Activity Mode")]
    mode: RefCell<ActivityMode>,

    #[property(get, nick = "Local CSS Provider")]
    local_css_context: RefCell<ActivityWidgetLocalCssContext>, // TODO change in favor of StateTransition

    #[property(get, set, nick = "Widget name")]
    name: RefCell<String>,

    last_mode: RefCell<ActivityMode>,

    transition: RefCell<Transition>,

    background_widget: RefCell<Option<gtk::Widget>>,

    minimal_mode_widget: RefCell<Option<gtk::Widget>>,

    compact_mode_widget: RefCell<Option<gtk::Widget>>,

    expanded_mode_widget: RefCell<Option<gtk::Widget>>,

    overlay_mode_widget: RefCell<Option<gtk::Widget>>,
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
                self.last_mode.replace(self.mode.borrow().clone());
                self.mode.replace(value.get().unwrap());
                let start: Instant;
                let duration: Duration;
                if self.transition.borrow().is_active() {
                    duration = Duration::from_millis(
                        self.local_css_context.borrow().get_transition_duration(),
                    ) + self.transition.borrow().duration_to_end();
                    start = Instant::now()
                        .checked_sub(self.transition.borrow().duration_to_end())
                        .expect("time error");
                } else {
                    start = Instant::now();
                    duration = Duration::from_millis(
                        self.local_css_context.borrow().get_transition_duration(),
                    );
                }

                self.transition.replace(Transition::new(start, duration));

                if let Some(widget) = &*self.get_mode_widget(self.mode.borrow().clone()).borrow() {
                    if let Some(widget) = &*self.get_mode_widget(self.last_mode.borrow().clone()).borrow() {
                        match widget.window() {
                            //lower previous window associated to widget if it has one, this disables events on the last mode widget
                            Some(window) => window.lower(),
                            None => {
                                // println!("no window");
                            }
                        }
                    }
                    if let Some(widget) = &*self.background_widget.borrow() {
                        match widget.window() {
                            //lower previous window associated to widget if it has one, this disables events on the last mode widget
                            Some(window) => window.raise(),
                            None => {
                                // println!("no window");
                            }
                        }
                    }
                    match widget.window() {
                        //raise window associated to widget if it has one, this enables events on the active mode widget
                        Some(window) => window.raise(),
                        None => {
                            // println!("no window");
                        }
                    }
                    let height = match *self.mode.borrow() {
                        ActivityMode::Minimal | ActivityMode::Compact => MINIMAL_HEIGHT,
                        ActivityMode::Expanded | ActivityMode::Overlay => {
                            if widget.height_request() != -1 {
                                widget.height_request()
                            } else {
                                widget.allocation().height()
                            }
                        }
                    };
                    self.local_css_context
                        .borrow_mut()
                        .set_size((
                            if widget.width_request() != -1 {
                                widget.width_request()
                            } else {
                                widget.allocation().width()
                            },
                            height,
                        ))
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
        Self {
            mode: RefCell::new(ActivityMode::Minimal),
            // transition_duration: RefCell::new(0),
            local_css_context: RefCell::new(ActivityWidgetLocalCssContext::new(&name)),
            last_mode: RefCell::new(ActivityMode::Minimal),
            name: RefCell::new(name),
            transition: RefCell::new(Transition::new(Instant::now(), Duration::ZERO)),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

impl ActivityWidgetPriv {
    fn get_mode_widget(&self, mode: ActivityMode) -> &RefCell<Option<gtk::Widget>> {
        match mode {
            ActivityMode::Minimal => &self.minimal_mode_widget,
            ActivityMode::Compact => &self.compact_mode_widget,
            ActivityMode::Expanded => &self.expanded_mode_widget,
            ActivityMode::Overlay => &self.overlay_mode_widget,
        }
    }

    fn get_child_aligned_allocation(&self, child: &gtk::Widget) -> gdk::Rectangle {
        let parent_allocation = self.obj().allocation();
        let x: i32;
        let y: i32;
        let mut width = child.preferred_width().0;
        let mut height = child.preferred_height().0;
        match child.halign() {
            gtk::Align::Start => {
                x = parent_allocation.x();
            }
            gtk::Align::End => {
                x = parent_allocation.x() + (parent_allocation.width() - width);
            }
            gtk::Align::Center => {
                x = parent_allocation.x()
                    + ((parent_allocation.width() - width) as f32 / 2.0).ceil() as i32;
            }
            _ => {
                glib::g_warning!(
                    "warning",
                    "align set to FILL/BASELINE, this will break resizing"
                );
                x = parent_allocation.x();
                width = parent_allocation.width();
            }
        }
        match child.valign() {
            gtk::Align::Start => {
                y = parent_allocation.y();
            }
            gtk::Align::End => {
                y = parent_allocation.y() + (parent_allocation.height() - height);
            }
            gtk::Align::Center => {
                y = parent_allocation.y()
                    + ((parent_allocation.height() - height) as f32 / 2.0).ceil() as i32;
            }
            _ => {
                glib::g_warning!(
                    "warning",
                    "align set to FILL/BASELINE,this will break resizing"
                );
                y = parent_allocation.y();
                height = parent_allocation.height();
            }
        }
        gtk::Allocation::new(x, y, width, height)
    }
    fn timing_functions(&self, progress: f32, timing_for: TimingFunction) -> f32 {
        // TODO add information on bigger or smaller prev and next

        match timing_for {
            TimingFunction::BiggerBlur => {
                self.local_css_context.borrow().get_transition_bigger_blur().calculate(progress)
                // soy::EASE_IN.calculate(progress)
            }
            TimingFunction::BiggerStretch => {
                self.local_css_context.borrow().get_transition_bigger_stretch().calculate(progress)
                // soy::EASE_OUT.calculate(progress)
            }
            TimingFunction::BiggerOpacity => {
                self.local_css_context.borrow().get_transition_bigger_opacity().calculate(progress)
                // soy::cubic_bezier(0.2, 0.55, 0.15, 1.0).calculate(progress)
            }
            TimingFunction::SmallerBlur => {
                self.local_css_context.borrow().get_transition_smaller_blur().calculate(progress)
                // soy::EASE_IN.calculate(progress)
            }
            TimingFunction::SmallerStretch => {
                self.local_css_context.borrow().get_transition_smaller_stretch().calculate(progress)
                // soy::EASE_OUT.calculate(progress)
            }
            TimingFunction::SmallerOpacity => {
                self.local_css_context.borrow().get_transition_smaller_opacity().calculate(progress)
                // soy::cubic_bezier(0.2, 0.55, 0.15, 1.0).calculate(progress)
            }
        }
    }
}

enum TimingFunction {
    BiggerBlur,
    BiggerStretch,
    BiggerOpacity,
    SmallerBlur,
    SmallerStretch,
    SmallerOpacity,
}

//init widget info
#[object_subclass]
impl ObjectSubclass for ActivityWidgetPriv {
    type ParentType = gtk::Container;
    type Type = ActivityWidget;

    const NAME: &'static str = "ActivityWidget";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("activity-widget");
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
            gtk::STYLE_PROVIDER_PRIORITY_SETTINGS,
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
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((
                    if widget.width_request() != -1 {
                        widget.width_request()
                    } else {
                        widget.allocation().width()
                    },
                    MINIMAL_HEIGHT,
                ))
                .expect("failed to set activity size");
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // println!("no window");
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
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((
                    if widget.width_request() != -1 {
                        widget.width_request()
                    } else {
                        widget.allocation().width()
                    },
                    MINIMAL_HEIGHT,
                ))
                .expect("failed to set activity size");
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // println!("no window");
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
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((
                    if widget.width_request() != -1 {
                        widget.width_request()
                    } else {
                        widget.allocation().width()
                    },
                    if widget.height_request() != -1 {
                        widget.height_request()
                    } else {
                        widget.allocation().height()
                    },
                ))
                .expect("failed to set activity size");
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // println!("no window");
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
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((
                    if widget.width_request() != -1 {
                        widget.width_request()
                    } else {
                        widget.allocation().width()
                    },
                    if widget.height_request() != -1 {
                        widget.height_request()
                    } else {
                        widget.allocation().height()
                    },
                ))
                .expect("failed to set activity size");
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // println!("no window");
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

    crate::implement_set_transition!(pub, transition_size);
    crate::implement_set_transition!(pub, transition_bigger_blur);
    crate::implement_set_transition!(pub, transition_bigger_stretch);
    crate::implement_set_transition!(pub, transition_bigger_opacity);
    crate::implement_set_transition!(pub, transition_smaller_blur);
    crate::implement_set_transition!(pub, transition_smaller_stretch);
    crate::implement_set_transition!(pub, transition_smaller_opacity);
    

}

#[macro_export]
macro_rules! implement_set_transition{
    ($vis:vis, $val:tt) => {
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&self, transition: Bezier, module: bool) -> Result<()> {
                self.imp()
                    .local_css_context
                    .borrow_mut()
                    .name(transition, module)
            }
        });
    };
}

//add/remove bg_widget and expose info to GTK debugger
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

//size allocation and draw
impl WidgetImpl for ActivityWidgetPriv {
    fn preferred_width_for_height(&self, height: i32) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_width_for_height(height), (height, height))
            }
            _ => (MINIMAL_HEIGHT, MINIMAL_HEIGHT),
        }
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_height_for_width(width), (0, width))
            }
            _ => (MINIMAL_HEIGHT, MINIMAL_HEIGHT),
        }
    }

    fn preferred_height(&self) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_height(),
            _ => (MINIMAL_HEIGHT, MINIMAL_HEIGHT),
        }
    }

    fn preferred_width(&self) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_width(),
            _ => (MINIMAL_HEIGHT, MINIMAL_HEIGHT),
        }
    }

    fn size_allocate(&self, allocation: &gdk::Rectangle) {
        // println!("activity allocate: ({}, {})", allocation.width(), allocation.height());

        if let Some(content) = &*self.background_widget.borrow() {
            content.size_allocate(allocation);
            self.obj().set_allocation(&content.allocation());
        } else {
            self.obj().set_allocation(allocation);
        }

        if let Some(content) = &*self.minimal_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.compact_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.expanded_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.overlay_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }

        if let Some(widget) = &*self.get_mode_widget(self.mode.borrow().clone()).borrow() {
            let height = match *self.mode.borrow() {
                ActivityMode::Minimal | ActivityMode::Compact => MINIMAL_HEIGHT,
                ActivityMode::Expanded | ActivityMode::Overlay => {
                    if widget.height_request() != -1 {
                        widget.height_request()
                    } else {
                        widget.allocation().height()
                    }
                }
            };
            self.local_css_context
                .borrow_mut()
                .set_size((
                    if widget.width_request() != -1 {
                        widget.width_request()
                    } else {
                        widget.allocation().width()
                    },
                    height,
                ))
                .expect("failed to set activity size");
        }
        // println!("css_size: {:?}",self.local_css_context.borrow().get_size());
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        // FIXME probably need to fix margins like in scrolling_label
        let mut logs: Vec<String> = vec![];
        let start = Instant::now();
        let mut time = Instant::now();
        let res: Result<()> = try {
            let bg_color: gdk::RGBA = self //TODO keep only bg_widget as background, this is only for testing purposes
                .obj()
                .style_context()
                .style_property_for_state("background-color", self.obj().state_flags())
                .get()?;
            cr.save()?;

            cr.move_to(
                self.obj().allocation().x() as f64,
                self.obj().allocation().y() as f64,
            );

            let border_radius: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-radius", self.obj().state_flags())
                .get()?;
            let border_radius = border_radius as f64;
            let radius = f64::min(
                border_radius,
                f64::min(
                    self.obj().allocated_height() as f64 / 2.0,
                    self.obj().allocated_width() as f64 / 2.0,
                ),
            );
            self.local_css_context
                .borrow_mut()
                .set_border_radius(radius as i32)
                .expect("failed to set activity border-radius");

            //draw background
            cr.rectangle(
                0.0,
                0.0,
                self.obj().allocated_width() as f64,
                self.obj().allocated_height() as f64,
            );
            cr.set_source_rgba(
                bg_color.red(),
                bg_color.green(),
                bg_color.blue(),
                bg_color.alpha(),
            ); //TODO should always be transparent
            cr.fill()?;

            //setup clip
            cr.arc(radius, radius, radius, PI * 1.0, PI * 1.5); //top left //WHY are the angles rotated by 90 degrees
            cr.line_to(self.obj().allocated_width() as f64 - radius, 0.0);
            cr.arc(
                self.obj().allocated_width() as f64 - radius,
                radius,
                radius,
                PI * 1.5,
                PI * 0.0,
            ); //top right
            cr.line_to(
                self.obj().allocated_width() as f64,
                self.obj().allocated_height() as f64 - radius,
            );
            cr.arc(
                self.obj().allocated_width() as f64 - radius,
                self.obj().allocated_height() as f64 - radius,
                radius,
                PI * 0.0,
                PI * 0.5,
            ); //bottom right
            cr.line_to(radius, self.obj().allocated_height() as f64);
            cr.arc(
                radius,
                self.obj().allocated_height() as f64 - radius,
                radius,
                PI * 0.5,
                PI * 1.0,
            ); //bottom left
            cr.line_to(0.0, radius);
            cr.clip();
            logs.push(format!("bg color + clip setup {:?}", time.elapsed()));
            time = Instant::now();

            //draw bckground widget
            // if let Some(bg_widget) = &*self.background_widget.borrow() {
            //     self.obj().propagate_draw(bg_widget, cr);
            // }
            logs.push(format!("bg widget draw {:?}", time.elapsed()));
            time = Instant::now();

            //draw active mode widget
            let widget_to_render = self.get_mode_widget(self.mode.borrow().clone());

            //animate blur and opacity if during transition
            let self_w = self.obj().allocation().width() as f64;
            let self_h = self.obj().allocation().height() as f64;
            if self.transition.borrow().is_active() {
                let progress = self.transition.borrow().get_progress();
                // println!("{}, start: {:?}, dur: {:?}",progress, self.transition.borrow().start_time.elapsed(), self.transition.borrow().duration);
                let last_widget_to_render = self.get_mode_widget(self.last_mode.borrow().clone());

                let prev_size = if let Some(widget) = &*last_widget_to_render.borrow() {
                    widget.size_request()
                } else {
                    (0, 0)
                };
                let next_size = if let Some(widget) = &*widget_to_render.borrow() {
                    widget.size_request()
                } else {
                    (0, 0)
                };

                let bigger = next_size.0 > prev_size.0 || next_size.1 > prev_size.1;
                // println!("bigger: w({}), h({})", next_size.0 > prev_size.0, next_size.1 > prev_size.1);
                const RAD: f32 = 9.0;
                const FILTER_BACKEND: FilterBackend = FilterBackend::Gpu; //TODO move to config file

                let mut tmp_surface_1 = gtk::cairo::ImageSurface::create(
                    gdk::cairo::Format::ARgb32,
                    self.obj().allocation().width(),
                    self.obj().allocation().height(),
                )
                .with_context(|| "failed to create new imagesurface")?;
                if let Some(widget) = &*last_widget_to_render.borrow() {
                    // println!("widget pos: ({}, {}), parent size: ({}, {})",widget.allocation().x(),widget.allocation().y(), self_w, self_h);
                    let wid_w = widget.allocation().width() as f64;
                    let wid_h = widget.allocation().height() as f64;

                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface_1)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    let (mut sx, mut sy) = (self_w / wid_w, self_h / wid_h);
                    let scale_prog = self.timing_functions(
                        progress,
                        if bigger {
                            TimingFunction::BiggerStretch
                        } else {
                            TimingFunction::SmallerStretch
                        },
                    ) as f64;
                    // println!("scale_prev: {scale_prog}");
                    sx = (1.0 - scale_prog) + sx * scale_prog; // 0->1 | 1-> +sx >1 | 0.5-> 0.5+sx/2=(1+sx)/2 >1
                    sy = (1.0 - scale_prog) + sy * scale_prog;

                    //setup clip
                    let radius = f64::min(border_radius, f64::min(wid_h / 2.0, wid_w / 2.0));

                    tmp_cr.arc(
                        (self_w - wid_w * sx) / 2.0 + radius,
                        (self_h - wid_h * sy) / 2.0 + radius,
                        radius,
                        PI * 1.0,
                        PI * 1.5,
                    ); //top left //WHY are the angles rotated by 90 degrees
                    tmp_cr.line_to(
                        self_w - (self_w - wid_w * sx) / 2.0 - radius,
                        (self_h - wid_h * sy) / 2.0,
                    );
                    tmp_cr.arc(
                        self_w - (self_w - wid_w * sx) / 2.0 - radius,
                        (self_h - wid_h * sy) / 2.0 + radius,
                        radius,
                        PI * 1.5,
                        PI * 0.0,
                    ); //top right
                    tmp_cr.line_to(
                        self_w - (self_w - wid_w * sx) / 2.0,
                        self_h - (self_h - wid_h * sy) / 2.0 - radius,
                    );
                    tmp_cr.arc(
                        self_w - (self_w - wid_w * sx) / 2.0 - radius,
                        self_h - (self_h - wid_h * sy) / 2.0 - radius,
                        radius,
                        PI * 0.0,
                        PI * 0.5,
                    ); //bottom right
                    tmp_cr.line_to(
                        (self_w - wid_w * sx) / 2.0 + radius,
                        self_h - (self_h - wid_h * sy) / 2.0,
                    );
                    tmp_cr.arc(
                        (self_w - wid_w * sx) / 2.0 + radius,
                        self_h - (self_h - wid_h * sy) / 2.0 - radius,
                        radius,
                        PI * 0.5,
                        PI * 1.0,
                    ); //bottom left
                    tmp_cr.line_to(
                        (self_w - wid_w * sx) / 2.0,
                        (self_h - wid_h * sy) / 2.0 + radius,
                    );
                    tmp_cr.clip();

                    tmp_cr.scale(sx, sy);

                    tmp_cr.translate(
                        -(self_w - wid_w) / 2.0 + (self_w - wid_w * sx) / (2.0 * sx),
                        -(self_h - wid_h) / 2.0 + (self_h - wid_h * sy) / (2.0 * sy),
                    );

                    self.obj().propagate_draw(widget, &tmp_cr);

                    tmp_cr.reset_clip();

                    logs.push(format!(
                        "prev_widget draw + clip + scale {:?}",
                        time.elapsed()
                    ));
                    time = Instant::now();
                    drop(tmp_cr);

                    // crate::filters::filter::apply_blur(
                    //     &mut tmp_surface_1,
                    //     ActivityWidgetPriv::timing_functions(progress, TimingFunction::PrevBlur)
                    //         * RAD,
                    //     FILTER_BACKEND,
                    // )
                    // .with_context(|| "failed to apply blur to tmp surface")?;

                    logs.push(format!("prev blur processed {:?}", time.elapsed()));
                    time = Instant::now();

                    // cr.set_source_surface(&tmp_surface_1, 0.0, 0.0)
                    //     .with_context(|| "failed to set source surface")?;

                    // cr.paint_with_alpha(ActivityWidgetPriv::timing_functions(
                    //     progress,
                    //     TimingFunction::PrevOpacity,
                    // ) as f64)
                    //     .with_context(|| "failed to paint surface to context")?;

                    logs.push(format!("prev blur written to surface {:?}", time.elapsed()));
                    time = Instant::now();
                }
                
                let mut tmp_surface_2 = gtk::cairo::ImageSurface::create(
                    gdk::cairo::Format::ARgb32,
                    self.obj().allocation().width(),
                    self.obj().allocation().height(),
                )
                .with_context(|| "failed to create new imagesurface")?;
                if let Some(widget) = &*widget_to_render.borrow() {
                    let wid_w = widget.allocation().width() as f64;
                    let wid_h = widget.allocation().height() as f64;

                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface_2)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    let (mut sx, mut sy) = (self_w / wid_w, self_h / wid_h);
                    let scale_prog = self.timing_functions(
                        1.0-progress,
                        if bigger {
                            TimingFunction::SmallerStretch
                        } else {
                            TimingFunction::BiggerStretch
                        },
                    ) as f64;
                    // println!("scale_next: {scale_prog}");
                    sx = (1.0 - scale_prog) + sx * scale_prog; // 0->1 | 1-> +sx >1 | 0.5-> 0.5+sx/2=(1+sx)/2 >1
                    sy = (1.0 - scale_prog) + sy * scale_prog;

                    //setup clip
                    let radius = f64::min(border_radius, f64::min(wid_h / 2.0, wid_w / 2.0));

                    tmp_cr.arc(
                        (self_w - wid_w * sx) / 2.0 + radius,
                        (self_h - wid_h * sy) / 2.0 + radius,
                        radius,
                        PI * 1.0,
                        PI * 1.5,
                    ); //top left //WHY are the angles rotated by 90 degrees
                    tmp_cr.line_to(
                        self_w - (self_w - wid_w * sx) / 2.0 - radius,
                        (self_h - wid_h * sy) / 2.0,
                    );
                    tmp_cr.arc(
                        self_w - (self_w - wid_w * sx) / 2.0 - radius,
                        (self_h - wid_h * sy) / 2.0 + radius,
                        radius,
                        PI * 1.5,
                        PI * 0.0,
                    ); //top right
                    tmp_cr.line_to(
                        self_w - (self_w - wid_w * sx) / 2.0,
                        self_h - (self_h - wid_h * sy) / 2.0 - radius,
                    );
                    tmp_cr.arc(
                        self_w - (self_w - wid_w * sx) / 2.0 - radius,
                        self_h - (self_h - wid_h * sy) / 2.0 - radius,
                        radius,
                        PI * 0.0,
                        PI * 0.5,
                    ); //bottom right
                    tmp_cr.line_to(
                        (self_w - wid_w * sx) / 2.0 + radius,
                        self_h - (self_h - wid_h * sy) / 2.0,
                    );
                    tmp_cr.arc(
                        (self_w - wid_w * sx) / 2.0 + radius,
                        self_h - (self_h - wid_h * sy) / 2.0 - radius,
                        radius,
                        PI * 0.5,
                        PI * 1.0,
                    ); //bottom left
                    tmp_cr.line_to(
                        (self_w - wid_w * sx) / 2.0,
                        (self_h - wid_h * sy) / 2.0 + radius,
                    );
                    tmp_cr.clip();

                    tmp_cr.scale(sx, sy);
                    tmp_cr.translate(
                        -(self_w - wid_w) / 2.0 + (self_w - wid_w * sx) / (2.0 * sx),
                        -(self_h - wid_h) / 2.0 + (self_h - wid_h * sy) / (2.0 * sy),
                    );

                    self.obj().propagate_draw(widget, &tmp_cr);

                    tmp_cr.reset_clip();

                    logs.push(format!(
                        "next_widget draw + clip + scale {:?}",
                        time.elapsed()
                    ));
                    time = Instant::now();

                    drop(tmp_cr);

                    // crate::filters::filter::apply_blur(
                    //     &mut tmp_surface_2,
                    //     ActivityWidgetPriv::timing_functions(progress, TimingFunction::NextBlur)
                    //         * RAD,
                    //     FILTER_BACKEND,
                    // )
                    // .with_context(|| "failed to apply blur to tmp surface")?;

                    logs.push(format!("next blur processed {:?}", time.elapsed()));
                    time = Instant::now();

                    // cr.set_source_surface(&tmp_surface_2, 0.0, 0.0)
                    //     .with_context(|| "failed to set source surface")?;

                    // cr.paint_with_alpha(ActivityWidgetPriv::timing_functions(
                    //     progress,
                    //     TimingFunction::NextOpacity,
                    // ) as f64)
                    //     .with_context(|| "failed to paint surface to context")?;

                    logs.push(format!("next blur written to surface {:?}", time.elapsed()));
                    time = Instant::now();
                }

                // let mut orig_surface = cr.group_target();

                crate::filters::filter::apply_blur_and_merge_opacity_dual(
                    // &mut orig_surface,
                    &mut tmp_surface_1,
                    &mut tmp_surface_2,
                    self.timing_functions(
                        progress,
                        if bigger {
                            TimingFunction::BiggerBlur
                        } else {
                            TimingFunction::SmallerBlur
                        },
                    ) * RAD,
                    self.timing_functions(
                        1.0 - progress,
                        if bigger {
                            TimingFunction::SmallerBlur
                        } else {
                            TimingFunction::BiggerBlur
                        },
                    ) * RAD,
                    self.timing_functions(
                        1.0 - progress,
                        if bigger {
                            TimingFunction::BiggerOpacity
                        } else {
                            TimingFunction::SmallerOpacity
                        },
                    ),
                    self.timing_functions(
                        progress,
                        if bigger {
                            TimingFunction::SmallerOpacity
                        } else {
                            TimingFunction::BiggerOpacity
                        },
                    ),
                    FILTER_BACKEND,
                )
                .with_context(|| "failed to apply double blur + merge to tmp surface")?;

                logs.push(format!("double blur processed {:?}", time.elapsed()));
                time = Instant::now();

                cr.set_source_surface(&tmp_surface_1, 0.0, 0.0)
                    .with_context(|| "failed to set source surface")?;

                cr.paint()
                    .with_context(|| "failed to paint surface to context")?;

                logs.push(format!(
                    "double blur written to surface {:?}",
                    time.elapsed()
                ));
                // time = Instant::now();
            } else if let Some(widget) = &*widget_to_render.borrow() {
                self.obj().propagate_draw(widget, cr);
                logs.push(format!("static widget drawn {:?}", time.elapsed()));
                // time = Instant::now();
            }

            //reset
            cr.reset_clip();

            let border_color_top: gdk::RGBA = self
                .obj()
                .style_context()
                .style_property_for_state("border-top-color", self.obj().state_flags())
                .get()?;
            let border_color_bottom: gdk::RGBA = self
                .obj()
                .style_context()
                .style_property_for_state("border-bottom-color", self.obj().state_flags())
                .get()?;
            let border_color_left: gdk::RGBA = self
                .obj()
                .style_context()
                .style_property_for_state("border-left-color", self.obj().state_flags())
                .get()?;
            let border_color_right: gdk::RGBA = self
                .obj()
                .style_context()
                .style_property_for_state("border-right-color", self.obj().state_flags())
                .get()?;

            let border_width_top: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-top-width", self.obj().state_flags())
                .get()?;
            let border_width_bottom: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-bottom-width", self.obj().state_flags())
                .get()?;
            let border_width_left: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-left-width", self.obj().state_flags())
                .get()?;
            let border_width_right: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-right-width", self.obj().state_flags())
                .get()?;

            let border_style: gtk::BorderStyle = self
                .obj()
                .style_context()
                .style_property_for_state("border-style", self.obj().state_flags())
                .get()?;

            let draw_border: bool;
            let (mut offset_top, mut offset_bottom, mut offset_left, mut offset_right) =
                (0.0, 0.0, 0.0, 0.0);
            match border_style {
                gtk::BorderStyle::Solid => {
                    //FIXME doesn't work well, it's clipped on the edges
                    draw_border = true;
                }
                gtk::BorderStyle::Inset => {
                    draw_border = true;
                    offset_top = border_width_top as f64 / 2.0;
                    offset_bottom = border_width_bottom as f64 / 2.0;
                    offset_left = border_width_left as f64 / 2.0;
                    offset_right = border_width_right as f64 / 2.0;
                }
                _ => {
                    draw_border = false;
                    //border type not supported
                }
            }
            if draw_border {
                cr.move_to(
                    radius - f64::cos(PI * 1.75) * (radius - offset_top),
                    radius + f64::sin(PI * 1.75) * (radius - offset_top),
                );
                cr.arc(radius, radius, radius - offset_top, PI * 1.25, PI * 1.5); //top
                cr.line_to(self_w - radius, offset_top);
                cr.arc(
                    self_w - radius,
                    radius,
                    radius - offset_top,
                    PI * 1.5,
                    PI * 1.75,
                );

                cr.set_source_rgba(
                    border_color_top.red(),
                    border_color_top.green(),
                    border_color_top.blue(),
                    border_color_top.alpha(),
                );
                cr.set_line_width(border_width_top as f64);
                cr.stroke()?;

                cr.arc(
                    self_w - radius,
                    radius,
                    radius - offset_right,
                    PI * 1.75,
                    PI * 0.0,
                ); //right
                cr.line_to(self_w - offset_right, self_h - radius);
                cr.arc(
                    self_w - radius,
                    self_h - radius,
                    radius - offset_right,
                    PI * 0.0,
                    PI * 0.25,
                );

                cr.set_source_rgba(
                    border_color_right.red(),
                    border_color_right.green(),
                    border_color_right.blue(),
                    border_color_right.alpha(),
                );
                cr.set_line_width(border_width_right as f64);
                cr.stroke()?;

                cr.arc(
                    self_w - radius,
                    self_h - radius,
                    radius - offset_bottom,
                    PI * 0.25,
                    PI * 0.5,
                ); //bottom
                cr.line_to(radius, self_h - offset_bottom);
                cr.arc(
                    radius,
                    self_h - radius,
                    radius - offset_bottom,
                    PI * 0.5,
                    PI * 0.75,
                );

                cr.set_source_rgba(
                    border_color_bottom.red(),
                    border_color_bottom.green(),
                    border_color_bottom.blue(),
                    border_color_bottom.alpha(),
                );
                cr.set_line_width(border_width_bottom as f64);
                cr.stroke()?;

                cr.arc(
                    radius,
                    self_h - radius,
                    radius - offset_left,
                    PI * 0.75,
                    PI * 1.0,
                ); //left
                cr.line_to(offset_left, radius);
                cr.arc(radius, radius, radius - offset_left, PI * 1.0, PI * 1.25);
                // cr.move_to(offset_left+30.0, radius-30.0);

                cr.set_source_rgba(
                    border_color_left.red(),
                    border_color_left.green(),
                    border_color_left.blue(),
                    border_color_left.alpha(),
                );
                cr.set_line_width(border_width_left as f64);
                cr.stroke()?;
            }

            self.transition.borrow_mut().update_active();
            if self.transition.borrow().is_active() {
                self.obj().queue_draw();
            }

            cr.restore()?;
        };

        if let Err(err) = res {
            eprintln!("{err}");
        }

        logs.push(format!("total: {:?}\n", start.elapsed()));

        if start.elapsed()>Duration::from_millis(16){
            for log in logs {
                // println!("{log}"); //TODO maybe create a utility library
            }
        }
        glib::Propagation::Proceed
    }
}

fn get_max_preferred_size(m1: (i32, i32), m2: (i32, i32)) -> (i32, i32) {
    (std::cmp::max(m1.0, m2.0), std::cmp::max(m1.1, m2.1))
}
