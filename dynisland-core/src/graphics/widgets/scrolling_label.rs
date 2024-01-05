use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use css_anim::{
    ease_functions::LinearEaseFunction,
    soy::EaseFunction,
    transition::{TransitionDef, TransitionManager},
};
use glib::{object_subclass, prelude::*, wrapper};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*};
use log::error;

use crate::{
    graphics::{
        activity_widget::allocate_and_draw::begin_draw_clip,
        config_variable::ConfigVariable,
        // animations::transition::{StateStruct, StateTransition},
    },
    implement_get_set,
};

// #[derive(Copy, Clone, Debug)]
// pub enum ScrollingLabelTransitionStateEnum {
//     Stopped,
//     Timeout,
//     Running,
// }

// #[derive(Clone, Debug)]
// pub struct ScrollingLabelTransitionState {
//     running_duration: u64,
//     timeout_duration: u64,
//     state: ScrollingLabelTransitionStateEnum,
// }
// impl Default for ScrollingLabelTransitionState {
//     fn default() -> Self {
//         Self {
//             running_duration: 0,
//             timeout_duration: 0,
//             state: ScrollingLabelTransitionStateEnum::Stopped,
//         }
//     }
// }
// impl StateTransition<ScrollingLabelTransitionState> {
//     pub fn set_running_duration(&mut self, running_duration: u64) {
//         if running_duration == self.get_state_struct().running_duration {
//             return;
//         }

//         self.get_state_struct().running_duration = running_duration;
//         if let ScrollingLabelTransitionStateEnum::Running = self.get_state() {
//             self.start_timer_duration(Duration::from_millis(running_duration));
//         }
//     }
//     pub fn set_timeout_duration(&mut self, timeout_duration: u64) {
//         if timeout_duration == self.get_state_struct().timeout_duration {
//             return;
//         }

//         self.get_state_struct().timeout_duration = timeout_duration;
//         if let ScrollingLabelTransitionStateEnum::Timeout = self.get_state() {
//             self.start_timer_duration(Duration::from_millis(timeout_duration));
//         }
//     }
// }

// impl StateStruct for ScrollingLabelTransitionState {
//     type StateEnum = ScrollingLabelTransitionStateEnum;

//     //assume transition is enabled and not running
//     fn timer_ended_callback(state_transition: &mut StateTransition<Self>) {
//         match state_transition.get_state() {
//             ScrollingLabelTransitionStateEnum::Stopped => {
//                 let state = state_transition.get_state_struct();
//                 state.set_state(ScrollingLabelTransitionStateEnum::Timeout);
//                 let timeout_duration = state.timeout_duration;
//                 state_transition.start_timer_duration(Duration::from_millis(timeout_duration));
//             }
//             ScrollingLabelTransitionStateEnum::Timeout => {
//                 let state = state_transition.get_state_struct();
//                 state.set_state(ScrollingLabelTransitionStateEnum::Running);
//                 let running_duration = state.running_duration;
//                 state_transition.start_timer_duration(Duration::from_millis(running_duration));
//             }
//             ScrollingLabelTransitionStateEnum::Running => {
//                 let state = state_transition.get_state_struct();
//                 state.set_state(ScrollingLabelTransitionStateEnum::Timeout);
//                 let timeout_duration = state.timeout_duration;
//                 state_transition.start_timer_duration(Duration::from_millis(timeout_duration));
//             }
//         }
//     }

//     fn get_idle_state() -> Self::StateEnum {
//         ScrollingLabelTransitionStateEnum::Stopped
//     }

//     fn get_state(&self) -> Self::StateEnum {
//         self.state
//     }

//     fn set_state(&mut self, state: Self::StateEnum) {
//         self.state = state;
//     }
// }

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedScrollingLabelLocalCssContext")]
pub struct ScrollingLabelLocalTransitionContext {
    transition_speed: ConfigVariable<u64>, //pixels per second //TODO set_by_module useless for now, because i can't set the speed or timeout from the general config file, currently this is only customizable if the modules include a setting for it
    transition_timeout: ConfigVariable<u64>, //millis
    transition: ConfigVariable<Box<dyn EaseFunction>>,
}

impl ScrollingLabelLocalTransitionContext {
    pub fn new() -> Self {
        Self {
            transition_timeout: ConfigVariable::new(0),
            // transition_timeout_set_by_module: false,
            transition_speed: ConfigVariable::new(1),
            // transition_speed_set_by_module: false,
            transition: ConfigVariable::new(Box::<LinearEaseFunction>::default()),
        }
    }

    implement_get_set!(pub, transition_timeout, u64);
    implement_get_set!(pub, transition_speed, u64);
    implement_get_set!(pub, transition, Box<dyn EaseFunction>);
}

impl Default for ScrollingLabelLocalTransitionContext {
    fn default() -> Self {
        Self::new()
    }
}

wrapper! {
    pub struct ScrollingLabel(ObjectSubclass<ScrollingLabelPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget;
}

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedOrientation")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Properties)]
#[properties(wrapper_type = ScrollingLabel)]
pub struct ScrollingLabelPriv {
    // #[property(get, set, nick = "Change mode", blurb = "The Activity Mode")]
    // mode: RefCell<ActivityMode>,
    local_transition_context: RefCell<ScrollingLabelLocalTransitionContext>,

    #[property(get, set, nick = "Scrolling Orientation")]
    orientation: RefCell<Orientation>,

    #[property(get, set, nick = "Max Width")]
    max_width: RefCell<i32>,
    #[property(get, set, nick = "Max Height before enabling scrolling")]
    max_height: RefCell<i32>,

    transition_manager: RefCell<TransitionManager>, //TODO borrow_mut is called in a lot of places, need to verify if borrow rules are always followed / use try_borrow_mut() / switch to mutex

    #[property(get, set, nick = "If the animation is enabled")]
    transition_enabled: RefCell<bool>,

    #[property(get, set, nick = "If the text rolls before ending the animation")]
    transition_roll: RefCell<bool>,

    /// if you use this, you shouldn't change alignment, text or wrap
    #[property(get, nick = "Internal Label")]
    inner_label: RefCell<gtk::Label>,
}

//set properties
#[glib::derived_properties]
impl ObjectImpl for ScrollingLabelPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn notify(&self, pspec: &glib::ParamSpec) {
        self.parent_notify(pspec)
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "orientation" => {
                let or = value.get().unwrap();
                let inner_label = self.inner_label.borrow();
                match or {
                    Orientation::Horizontal => {
                        inner_label.set_halign(gtk::Align::Start);
                        inner_label.set_justify(gtk::Justification::Left);
                        inner_label.set_valign(gtk::Align::Center);
                    }
                    Orientation::Vertical => {
                        inner_label.set_halign(gtk::Align::Center);
                        inner_label.set_justify(gtk::Justification::Left);
                        inner_label.set_valign(gtk::Align::Start);
                    }
                }
                self.orientation.replace(or);

                self.obj().queue_allocate();

                if *self.transition_enabled.borrow() {
                    let mut tm = self.transition_manager.borrow_mut();
                    tm.set_value_no_anim("translate", 0.0);
                    tm.set_duration("translate", self.get_transition_duration());
                    tm.set_value("translate", self.get_transition_size())
                }
            }
            "max-width" => {
                let value = value.get::<i32>().unwrap();
                if value > 0 || value == -1 {
                    self.max_width.replace(value);
                }
                self.obj().queue_allocate();

                if *self.transition_enabled.borrow() {
                    let mut tm = self.transition_manager.borrow_mut();
                    tm.set_value_no_anim("translate", 0.0);
                    tm.set_duration("translate", self.get_transition_duration());
                    tm.set_value("translate", self.get_transition_size())
                }
            }
            "max-height" => {
                let value = value.get::<i32>().unwrap();
                if value > 0 || value == -1 {
                    self.max_height.replace(value);
                }
                self.obj().queue_allocate();

                if *self.transition_enabled.borrow() {
                    let mut tm = self.transition_manager.borrow_mut();
                    tm.set_value_no_anim("translate", 0.0);
                    tm.set_duration("translate", self.get_transition_duration());
                    tm.set_value("translate", self.get_transition_size())
                }
            }
            "transition-enabled" => {
                let value = value.get::<bool>().unwrap();
                if value && !*self.transition_enabled.borrow() {
                    let mut tm = self.transition_manager.borrow_mut();
                    tm.set_value_no_anim("translate", 0.0);
                    tm.set_duration("translate", self.get_transition_duration());
                    tm.set_value("translate", self.get_transition_size())
                }
                self.transition_enabled.replace(value);
                self.obj().queue_draw();
            }
            "transition-roll" => {
                let value = value.get::<bool>().unwrap();
                self.transition_roll.replace(value);
                if *self.transition_enabled.borrow() {
                    let mut tm = self.transition_manager.borrow_mut();
                    tm.set_value_no_anim("translate", 0.0);
                    tm.set_duration("translate", self.get_transition_duration());
                    tm.set_value("translate", self.get_transition_size())
                }
            }
            x => {
                panic!("Tried to set inexistant property of ScrollingLabel: {}", x)
            }
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}

//default data
impl Default for ScrollingLabelPriv {
    fn default() -> Self {
        let mut transition_manager = TransitionManager::new(false);
        transition_manager.add_property("translate", 0.0);
        transition_manager.set_transition(
            "translate",
            &TransitionDef::new(
                Duration::ZERO,
                Box::<LinearEaseFunction>::default(),
                Duration::ZERO,
                false,
            ),
        );
        let inner_label = gtk::Label::new(None);
        inner_label.set_halign(gtk::Align::Start);
        inner_label.set_valign(gtk::Align::Center);
        // inner_label.set_margin_start(10);
        // inner_label.set_margin_end(10);
        inner_label.set_wrap(true);

        // let mut transition = StateTransition::default();
        // transition.enable();
        Self {
            local_transition_context: RefCell::new(ScrollingLabelLocalTransitionContext::new()),
            orientation: RefCell::new(Orientation::Horizontal),
            transition_manager: RefCell::new(transition_manager),
            transition_enabled: RefCell::new(true),
            transition_roll: RefCell::new(true),
            //TODO ???(What does this mean? What other things?) should also set max length and other things
            inner_label: RefCell::new(inner_label),
            max_width: RefCell::new(-1),
            max_height: RefCell::new(-1),
        }
    }
}

//init widget info
#[object_subclass]
impl ObjectSubclass for ScrollingLabelPriv {
    type ParentType = gtk::Bin;
    type Type = ScrollingLabel;

    const NAME: &'static str = "ScrollingLabel";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("scrolling-label");
    }
}

impl Default for ScrollingLabel {
    fn default() -> Self {
        Self::new()
    }
}
impl ScrollingLabel {
    pub fn new() -> Self {
        let wid = glib::Object::new::<Self>();
        wid.set_has_window(false);
        wid.inner_label().set_parent(&wid);

        wid
    }
    pub fn set_transition_speed(&self, pixels_per_second: u64, module: bool) -> Result<()> {
        if pixels_per_second == 0 {
            bail!("cannot set a speed of 0, disable the tranition instead")
        }
        self.imp()
            .local_transition_context
            .borrow_mut()
            .set_transition_speed(pixels_per_second, module)?;

        if self.transition_enabled() {
            let mut tm = self.imp().transition_manager.borrow_mut();
            tm.set_value_no_anim("translate", 0.0);
            tm.set_duration("translate", self.imp().get_transition_duration());
            tm.set_value("translate", self.imp().get_transition_size())
        }
        Ok(())
    }
    pub fn set_timeout_duration(&self, duration_millis: u64, module: bool) -> Result<()> {
        self.imp()
            .local_transition_context
            .borrow_mut()
            .set_transition_timeout(duration_millis, module)?;

        let mut tm = self.imp().transition_manager.borrow_mut();
        tm.set_delay(
            "translate",
            Duration::from_millis(
                self.imp()
                    .local_transition_context
                    .borrow()
                    .get_transition_timeout(),
            ),
            false,
        );
        // if self.transition_enabled() {
        //     tm.set_value_no_anim("translate", 0.0);
        //     tm.set_duration("translate", self.imp().get_transition_duration());
        //     tm.set_value("translate", self.imp().get_transition_size())
        // }
        Ok(())
    }
    pub fn set_transition(&self, transition: Box<dyn EaseFunction>, module: bool) -> Result<()> {
        self.imp()
            .local_transition_context
            .borrow_mut()
            .set_transition(dyn_clone::clone_box(transition.as_ref()), module)?;

        let mut tm = self.imp().transition_manager.borrow_mut();
        tm.set_easing_function("translate", transition);
        // tm.set_delay(
        //     "translate",
        //     Duration::from_millis(
        //         self.imp()
        //             .local_transition_context
        //             .borrow()
        //             .get_transition_timeout(),
        //     ),
        //     false,
        // );
        // if self.transition_enabled() {
        //     tm.set_value_no_anim("translate", 0.0);
        //     tm.set_duration("translate", self.imp().get_transition_duration());
        //     tm.set_value("translate", self.imp().get_transition_size())
        // }
        Ok(())
    }
    pub fn set_text(&self, text: &str) {
        if text == self.imp().inner_label.borrow().text() {
            return;
        }
        self.imp().inner_label.borrow().set_text(text);

        if self.transition_enabled() {
            let mut tm = self.imp().transition_manager.borrow_mut();
            tm.set_value_no_anim("translate", 0.0);
            tm.set_duration("translate", self.imp().get_transition_duration());
            tm.set_value("translate", self.imp().get_transition_size())
        }
    }
}

impl ContainerImpl for ScrollingLabelPriv {
    fn add(&self, _widget: &gtk::Widget) {
        glib::g_warning!(
            "warning",
            "you cannot add or remove widgets from ScrollingLabel"
        );
    }

    fn remove(&self, _widget: &gtk::Widget) {
        glib::g_warning!(
            "warning",
            "you cannot add or remove widgets from ScrollingLabel"
        );
    }

    fn forall(&self, _: bool, callback: &gtk::subclass::container::Callback) {
        callback.call(self.inner_label.borrow().upcast_ref());
    }

    fn child_type(&self) -> glib::Type {
        gtk::Widget::static_type()
    }
}

impl BinImpl for ScrollingLabelPriv {}

impl ScrollingLabelPriv {
    fn get_child_aligned_allocation(&self, child: &gtk::Label) -> gdk::Rectangle {
        let parent_allocation = self.obj().allocation();
        // trace!("parent alloc: ({}, {})", parent_allocation.width(), parent_allocation.height());
        let x: i32;
        let y: i32;
        let mut width = match *self.orientation.borrow() {
            Orientation::Horizontal => {
                child
                    .preferred_width_for_height(parent_allocation.height())
                    .1
            }
            Orientation::Vertical => *self.max_width.borrow(),
        };
        // trace!("max_w: {}",*self.max_width.borrow());
        let mut height = match *self.orientation.borrow() {
            Orientation::Horizontal => child.preferred_height().1,
            Orientation::Vertical => child.preferred_height_for_width(*self.max_width.borrow()).1,
        };
        match child.halign() {
            gtk::Align::Start => {
                x = parent_allocation.x();
            }
            gtk::Align::End => {
                x = parent_allocation.x() + (parent_allocation.width() - width);
            }
            gtk::Align::Center => {
                x = parent_allocation.x() + (parent_allocation.width() - width) / 2;
            }
            _ => {
                glib::g_warning!("warning", "align set to FILL/BASELINE, this will break ");
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
                y = parent_allocation.y() + (parent_allocation.height() - height) / 2;
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

    fn get_transition_duration(&self) -> Duration {
        let inner_w = self.inner_label.borrow().allocation().width()
            + self.inner_label.borrow().allocation().x();
        let inner_h = self.inner_label.borrow().allocation().height()
            + self.inner_label.borrow().allocation().y();
        let size = match *self.orientation.borrow() {
            Orientation::Horizontal => {
                if *self.max_width.borrow() == -1 {
                    return Duration::ZERO;
                }

                let size = i32::min(self.obj().allocation().width() - inner_w, 0);
                if *self.transition_roll.borrow() && size != 0 {
                    size - self.obj().allocation().width()
                } else {
                    size
                }
            }
            Orientation::Vertical => {
                if *self.max_height.borrow() == -1 {
                    return Duration::ZERO;
                }
                if *self.max_width.borrow() == -1 {
                    glib::g_warning!(
                        "warning",
                        "Orientation is Vertical but max_width is not set, this will not work well"
                    );
                }
                let size = i32::min(self.obj().allocation().height() - inner_h, 0);
                if *self.transition_roll.borrow() && size != 0 {
                    size - self.obj().allocation().height()
                } else {
                    size
                }
            }
        };
        let duration_ms = 1000.0 * i32::abs(size) as f64
            / u64::max(
                self.local_transition_context
                    .borrow()
                    .get_transition_speed(),
                1,
            ) as f64;
        Duration::from_millis(duration_ms as u64)
    }

    fn get_transition_size(&self) -> f64 {
        match *self.orientation.borrow() {
            Orientation::Horizontal => {
                (self.inner_label.borrow().allocation().width()
                    + self.inner_label.borrow().allocation().x()) as f64
            }
            Orientation::Vertical => {
                (self.inner_label.borrow().allocation().height()
                    + self.inner_label.borrow().allocation().y()) as f64
            }
        }
    }

    // fn timing_functions(progress: f64, timing_for: TimingFunction) -> f64 {
    //     match timing_for {
    //         TimingFunction::Translate => soy::LINEAR.ease(progress),
    //     }
    // }
}

// enum TimingFunction {
//     Translate,
// }
//size allocation and draw
impl WidgetImpl for ScrollingLabelPriv {
    fn preferred_width_for_height(&self, height: i32) -> (i32, i32) {
        let width = *self.max_width.borrow();
        let inner_width = self.inner_label.borrow().preferred_width_for_height(height);
        if width > 0 {
            (
                i32::min(width, inner_width.1),
                i32::min(width, inner_width.1),
            )
        } else {
            (inner_width.1, inner_width.1)
        }
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        let height = *self.max_height.borrow();
        let inner_height = self.inner_label.borrow().preferred_height_for_width(width);
        if height > 0 {
            (
                i32::min(height, inner_height.0),
                i32::min(height, inner_height.1),
            )
        } else {
            (inner_height.1, inner_height.1)
        }
    }

    fn preferred_height(&self) -> (i32, i32) {
        let height = *self.max_height.borrow();
        let inner_height = self.inner_label.borrow().preferred_height();
        if height > 0 {
            (
                i32::min(height, inner_height.0),
                i32::min(height, inner_height.1),
            )
        } else {
            (inner_height.1, inner_height.1)
        }
    }

    fn preferred_width(&self) -> (i32, i32) {
        let width = *self.max_width.borrow();
        let inner_width = self.inner_label.borrow().preferred_width();
        if width > 0 {
            (
                i32::min(width, inner_width.0),
                i32::min(width, inner_width.1),
            )
        } else {
            (inner_width.1, inner_width.1)
        }
    }

    fn size_allocate(&self, allocation: &gdk::Rectangle) {
        self.obj().set_allocation(allocation);
        self.obj().set_clip(allocation);
        // trace!("clip: {:?}", self.obj().clip());

        let inner = &*self.inner_label.borrow();
        let allocation = self.get_child_aligned_allocation(inner);
        inner.size_allocate(&allocation);
        // trace!("orientation: {:?}, child alloc: ({}, {})", self.orientation.borrow(), inner.allocation().width(), inner.allocation().height());
        self.get_transition_duration();
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        //FIXME text motion is choppy
        //TODO add blur/ reduce opacity at the edges maybe with cr.mask()
        let mut logs: Vec<String> = vec![];
        let start = Instant::now();
        // let mut time = Instant::now();

        let res: Result<()> = try {
            //setup clip
            let self_w = if *self.max_width.borrow() != -1 {
                i32::min(self.obj().allocation().width(), *self.max_width.borrow())
            } else {
                self.obj().allocation().width()
            } as f64;
            let self_h = if *self.max_width.borrow() != -1 {
                i32::min(self.obj().allocation().height(), *self.max_height.borrow())
            } else {
                self.obj().allocation().height()
            } as f64;

            //draw background
            gtk::render_background(
                &self.obj().style_context(),
                cr,
                0.0,
                0.0,
                self.obj().allocation().width() as f64,
                self.obj().allocation().height() as f64,
            );

            let border_radius: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-radius", self.obj().state_flags())
                .get()?;
            let border_radius = border_radius as f64;
            let radius = f64::min(border_radius, f64::min(self_w / 2.0, self_h / 2.0));

            begin_draw_clip(cr, (self_w, self_h), (self_w, self_h), radius);

            let inner = &*self.inner_label.borrow();
            // logs.push(format!("transition:{:?}", self.transition.borrow()));
            let mut tm = self.transition_manager.borrow_mut();
            let animating = tm.is_running("translate");

            if animating {
                let translation = tm.get_value("translate").unwrap();
                let inner_w = self.inner_label.borrow().allocation().width()
                    + self.inner_label.borrow().allocation().x();
                let inner_h = self.inner_label.borrow().allocation().height()
                    + self.inner_label.borrow().allocation().y();
                match *self.orientation.borrow() {
                    Orientation::Horizontal => {
                        // logs.push(format!("inner_w:({:?}), inner_x:({:?}), ", self.inner_label.borrow().allocation().width(),self.inner_label.borrow().allocation().x()));
                        if *self.transition_roll.borrow() {
                            let tmp_surface = gtk::cairo::ImageSurface::create(
                                gdk::cairo::Format::ARgb32,
                                inner_w,
                                inner_h,
                            )
                            .with_context(|| "failed to create new imagesurface")?;

                            let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
                                .with_context(|| "failed to retrieve context from tmp surface")?;

                            self.obj().propagate_draw(inner, &tmp_cr);
                            drop(tmp_cr);

                            cr.set_source_surface(&tmp_surface, -translation, 0.0)
                                .with_context(|| "failed to set source surface")?;
                            cr.paint()
                                .with_context(|| "failed to paint surface to context")?;

                            if -translation < (self.obj().allocation().width() - inner_w) as f64 {
                                cr.set_source_surface(
                                    &tmp_surface,
                                    -translation + inner_w as f64,
                                    0.0,
                                )
                                .with_context(|| "failed to set source surface")?;
                                cr.paint()
                                    .with_context(|| "failed to paint surface to context")?;
                            }
                        } else {
                            cr.translate(-translation, 0.0);
                            self.obj().propagate_draw(inner, cr);
                        }
                    }
                    Orientation::Vertical => {
                        // logs.push(format!("inner_h:({:?}), inner_y:({:?}), ", self.inner_label.borrow().allocation().height(),self.inner_label.borrow().allocation().y()));
                        if *self.transition_roll.borrow() {
                            let tmp_surface = gtk::cairo::ImageSurface::create(
                                gdk::cairo::Format::ARgb32,
                                inner_w,
                                inner_h,
                            )
                            .with_context(|| "failed to create new imagesurface")?;

                            let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
                                .with_context(|| "failed to retrieve context from tmp surface")?;

                            self.obj().propagate_draw(inner, &tmp_cr);
                            drop(tmp_cr);

                            cr.set_source_surface(&tmp_surface, 0.0, -translation)
                                .with_context(|| "failed to set source surface")?;
                            cr.paint()
                                .with_context(|| "failed to paint surface to context")?;

                            if -translation < (self.obj().allocation().height() - inner_h) as f64 {
                                cr.set_source_surface(
                                    &tmp_surface,
                                    0.0,
                                    -translation + inner_h as f64,
                                )
                                .with_context(|| "failed to set source surface")?;
                                cr.paint()
                                    .with_context(|| "failed to paint surface to context")?;
                            }
                        } else {
                            cr.translate(0.0, -translation);
                            self.obj().propagate_draw(inner, cr);
                        }
                    }
                }

                self.obj().queue_draw();
            } else {
                if !tm.is_idle("translate") && *self.transition_enabled.borrow() {
                    tm.set_value_no_anim("translate", 0.0);
                    tm.set_duration("translate", self.get_transition_duration());
                    tm.set_value("translate", self.get_transition_size())
                }
                let dur_to_end = tm.time_to_animating("transition");
                if dur_to_end <= Duration::from_millis(70) {
                    self.obj().queue_draw();
                } else {
                    let wid = self.obj().clone();
                    glib::MainContext::default().spawn_local(async move {
                        glib::timeout_future(dur_to_end - Duration::from_millis(50)).await;
                        wid.queue_draw(); // queue draw for future
                    });
                }
                self.obj().propagate_draw(inner, cr);
            }

            // match state {
            //     ScrollingLabelTransitionStateEnum::Stopped => {
            //         self.obj().propagate_draw(inner, cr);
            //     }
            //     ScrollingLabelTransitionStateEnum::Timeout => {
            //         let dur_to_end = self.transition.borrow().duration_to_end();
            //         if dur_to_end <= Duration::from_millis(70) {
            //             self.obj().queue_draw();
            //         } else {
            //             let wid = self.obj().clone();
            //             glib::MainContext::default().spawn_local(async move {
            //                 glib::timeout_future(dur_to_end).await;
            //                 wid.queue_draw(); // queue draw for future
            //             });
            //         }
            //         self.obj().propagate_draw(inner, cr);
            //     }
            //     ScrollingLabelTransitionStateEnum::Running => {
            //         if !self.transition.borrow().is_zero() {
            //             let progress = self.transition.borrow().get_progress();
            //             let inner_w = self.inner_label.borrow().allocation().width()
            //                 + self.inner_label.borrow().allocation().x();
            //             let inner_h = self.inner_label.borrow().allocation().height()
            //                 + self.inner_label.borrow().allocation().y();
            //             match *self.orientation.borrow() {
            //                 Orientation::Horizontal => {
            //                     // logs.push(format!("inner_w:({:?}), inner_x:({:?}), ", self.inner_label.borrow().allocation().width(),self.inner_label.borrow().allocation().x()));
            //                     if *self.transition_roll.borrow() {
            //                         let tmp_surface = gtk::cairo::ImageSurface::create(
            //                             gdk::cairo::Format::ARgb32,
            //                             inner_w,
            //                             inner_h,
            //                         )
            //                         .with_context(|| "failed to create new imagesurface")?;

            //                         let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
            //                             .with_context(|| {
            //                                 "failed to retrieve context from tmp surface"
            //                             })?;

            //                         self.obj().propagate_draw(inner, &tmp_cr);
            //                         drop(tmp_cr);

            //                         let max_tx = -inner_w as f64;

            //                         let tx = ScrollingLabelPriv::timing_functions(
            //                             progress,
            //                             TimingFunction::Translate,
            //                         ) * max_tx;

            //                         cr.set_source_surface(&tmp_surface, tx, 0.0)
            //                             .with_context(|| "failed to set source surface")?;
            //                         // cr.translate(tx, 0.0);
            //                         cr.paint()
            //                             .with_context(|| "failed to paint surface to context")?;

            //                         if tx < (self.obj().allocation().width() - inner_w) as f64 {
            //                             cr.set_source_surface(
            //                                 &tmp_surface,
            //                                 tx + inner_w as f64,
            //                                 0.0,
            //                             )
            //                             .with_context(|| "failed to set source surface")?;
            //                             // cr.translate(tx+inner_w as f64, 0.0);
            //                             cr.paint().with_context(|| {
            //                                 "failed to paint surface to context"
            //                             })?;
            //                         }
            //                     } else {
            //                         let max_tx = (self.obj().allocation().width() - inner_w) as f64;
            //                         cr.translate(
            //                             ScrollingLabelPriv::timing_functions(
            //                                 progress,
            //                                 TimingFunction::Translate,
            //                             ) * max_tx,
            //                             0.0,
            //                         );
            //                         self.obj().propagate_draw(inner, cr);
            //                     }
            //                 }
            //                 Orientation::Vertical => {
            //                     // logs.push(format!("inner_h:({:?}), inner_y:({:?}), ", self.inner_label.borrow().allocation().height(),self.inner_label.borrow().allocation().y()));
            //                     if *self.transition_roll.borrow() {
            //                         let tmp_surface = gtk::cairo::ImageSurface::create(
            //                             gdk::cairo::Format::ARgb32,
            //                             inner_w,
            //                             inner_h,
            //                         )
            //                         .with_context(|| "failed to create new imagesurface")?;

            //                         let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
            //                             .with_context(|| {
            //                                 "failed to retrieve context from tmp surface"
            //                             })?;

            //                         self.obj().propagate_draw(inner, &tmp_cr);
            //                         drop(tmp_cr);

            //                         let max_ty = -inner_h as f64;

            //                         let ty = ScrollingLabelPriv::timing_functions(
            //                             progress,
            //                             TimingFunction::Translate,
            //                         ) * max_ty;

            //                         cr.set_source_surface(&tmp_surface, 0.0, ty)
            //                             .with_context(|| "failed to set source surface")?;
            //                         // cr.translate(tx, 0.0);
            //                         cr.paint()
            //                             .with_context(|| "failed to paint surface to context")?;

            //                         if ty < (self.obj().allocation().height() - inner_h) as f64 {
            //                             cr.set_source_surface(
            //                                 &tmp_surface,
            //                                 0.0,
            //                                 ty + inner_h as f64,
            //                             )
            //                             .with_context(|| "failed to set source surface")?;
            //                             // cr.translate(tx+inner_w as f64, 0.0);
            //                             cr.paint().with_context(|| {
            //                                 "failed to paint surface to context"
            //                             })?;
            //                         }
            //                     } else {
            //                         let max_ty =
            //                             (self.obj().allocation().height() - inner_h) as f64;
            //                         cr.translate(
            //                             0.0,
            //                             ScrollingLabelPriv::timing_functions(
            //                                 progress,
            //                                 TimingFunction::Translate,
            //                             ) * max_ty,
            //                         );
            //                         self.obj().propagate_draw(inner, cr);
            //                     }
            //                 }
            //             }
            //         } else {
            //             self.obj().propagate_draw(inner, cr);
            //         }
            //         self.obj().queue_draw();
            //     }
            // }

            cr.reset_clip();

            gtk::render_frame(
                &self.obj().style_context(),
                cr,
                0.0,
                0.0,
                self.obj().allocation().width() as f64,
                self.obj().allocation().height() as f64,
            );
        };

        if let Err(err) = res {
            error!("{err}");
        }

        logs.push(format!("total: {:?}", start.elapsed()));
        // for log in &logs {
        //     debug!("{log}"); //TODO maybe create a utility library
        // }
        // if !logs.is_empty() {
        //     debug!();
        // }
        // todo!();
        glib::Propagation::Proceed
    }
}
