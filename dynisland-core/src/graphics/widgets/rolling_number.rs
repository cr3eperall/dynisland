use css_anim::{
    ease_functions::LinearEaseFunction,
    soy::{Bezier, EaseFunction},
    transition::{TransitionDef, TransitionManager},
};

use crate::{
    filters::filter, graphics::config_variable::ConfigVariable, implement_get_set,
    implement_set_transition,
};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use glib::{object_subclass, prelude::*, wrapper};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*};
use log::error;

const BLUR_RADIUS: f32 = 4.0;
const MIN_STRETCH: f64 = 0.5;
const SIZE_MULT_FACTOR: f64 = 1.5;
const TRANSLATE_CORRECTIVE_FACTOR: f64 = -1.5;

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedRollingNumberLocalTransitionContext")]
pub struct RollingNumberLocalTransitionContext {
    transition_delay: ConfigVariable<i64>, //TODO set_by_module useless for now, because i can't set the speed or timeout from the general config file, currently this is only customizable if the modules include a setting for it
    transition_duration: ConfigVariable<u64>, //millis
    translate_prev_transition: ConfigVariable<Box<dyn EaseFunction>>,
    scale_prev_transition: ConfigVariable<Box<dyn EaseFunction>>,
    opacity_prev_transition: ConfigVariable<Box<dyn EaseFunction>>,
    blur_prev_transition: ConfigVariable<Box<dyn EaseFunction>>,

    translate_next_transition: ConfigVariable<Box<dyn EaseFunction>>,
    scale_next_transition: ConfigVariable<Box<dyn EaseFunction>>,
    opacity_next_transition: ConfigVariable<Box<dyn EaseFunction>>,
    blur_next_transition: ConfigVariable<Box<dyn EaseFunction>>,
}
impl RollingNumberLocalTransitionContext {
    pub fn new() -> Self {
        Self {
            transition_delay: ConfigVariable::new(0),

            transition_duration: ConfigVariable::new(0),

            translate_prev_transition: ConfigVariable::new(Box::new(Bezier::new(
                0.81, 0.51, 0.39, 0.9,
            ))),
            scale_prev_transition: ConfigVariable::new(Box::new(Bezier::new(0.2, 0.2, 0.5, 1.3))),
            opacity_prev_transition: ConfigVariable::new(Box::new(Bezier::new(0.6, 0.3, 0.2, 1.2))),
            blur_prev_transition: ConfigVariable::new(Box::new(Bezier::new(0.7, 0.0, 1.0, 0.4))),

            translate_next_transition: ConfigVariable::new(Box::new(Bezier::new(
                0.6, 0.6, 0.4, 1.3,
            ))),
            scale_next_transition: ConfigVariable::new(Box::new(Bezier::new(0.5, -0.45, 0.5, 1.0))),
            opacity_next_transition: ConfigVariable::new(Box::new(Bezier::new(0.3, 1.0, 0.3, 1.0))),
            blur_next_transition: ConfigVariable::new(Box::new(Bezier::new(0.12, 0.76, 0.2, 1.0))),
        }
    }

    implement_get_set!(pub, transition_delay, i64);
    implement_get_set!(pub, transition_duration, u64);
    implement_get_set!(pub, translate_prev_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, scale_prev_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, opacity_prev_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, blur_prev_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, translate_next_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, scale_next_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, opacity_next_transition, Box<dyn EaseFunction>);
    implement_get_set!(pub, blur_next_transition, Box<dyn EaseFunction>);
}

impl Default for RollingNumberLocalTransitionContext {
    fn default() -> Self {
        Self::new()
    }
}

wrapper! {
    pub struct RollingNumber(ObjectSubclass<RollingNumberPriv>)
    @extends gtk::Container, gtk::Widget;
}

#[derive(Properties)]
#[properties(wrapper_type = RollingNumber)]
pub struct RollingNumberPriv {
    local_transition_context: RefCell<RollingNumberLocalTransitionContext>,

    transition_manager: RefCell<TransitionManager>, //TODO borrow_mut is called in a lot of places, need to verify if borrow rules are always followed / use try_borrow_mut() / switch to mutex

    #[property(get, set, nick = "If the animation is enabled")]
    transition_enabled: RefCell<bool>,

    #[property(get, set, builder('0'))]
    number: RefCell<char>,

    last_number: RefCell<char>,
    /// controls if the labels are switched or not
    first_is_prev: RefCell<bool>,

    /// if you use this, you shouldn't change alignment, text or wrap
    #[property(get, nick = "First Internal Label")]
    inner_label_1: RefCell<gtk::Label>, //TODO maybe change both using a callback

    #[property(get, nick = "Second Internal Label")]
    inner_label_2: RefCell<gtk::Label>,
}

#[glib::derived_properties]
impl ObjectImpl for RollingNumberPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn notify(&self, pspec: &glib::ParamSpec) {
        self.parent_notify(pspec)
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "transition-enabled" => {
                let value = value.get::<bool>().unwrap();

                self.transition_enabled.replace(value);
                self.obj().queue_draw();
            }
            "number" => {
                let value: char = value.get::<char>().unwrap();
                self.last_number.replace(*self.number.borrow());
                self.number.replace(value);
                if *self.transition_enabled.borrow() {
                    let tm = &mut *self.transition_manager.borrow_mut();
                    self.begin_transition(tm);
                    // debug!("begin transition");
                } else {
                    self.first_is_prev.replace(!*self.first_is_prev.borrow());
                    if *self.first_is_prev.borrow() {
                        self.inner_label_2
                            .borrow()
                            .set_text(&self.number.borrow().to_string());
                    } else {
                        self.inner_label_1
                            .borrow()
                            .set_text(&self.number.borrow().to_string());
                    }
                }
                self.obj().queue_draw();
            }
            x => {
                panic!("Tried to set inexistant property of RollingNumber: {}", x)
            }
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}

//default data
impl Default for RollingNumberPriv {
    fn default() -> Self {
        let mut transition_manager = TransitionManager::new(false);
        init_transition_properties(&mut transition_manager);

        Self {
            local_transition_context: RefCell::new(RollingNumberLocalTransitionContext::new()),
            transition_manager: RefCell::new(transition_manager),
            transition_enabled: RefCell::new(true),
            last_number: RefCell::new('0'),
            number: RefCell::new('0'),
            first_is_prev: RefCell::new(false),
            inner_label_1: RefCell::new(gtk::Label::new(None)),
            inner_label_2: RefCell::new(gtk::Label::new(None)),
        }
    }
}

fn init_transition_properties(tm: &mut TransitionManager) {
    tm.set_default_transition(&TransitionDef::new(
        Duration::from_millis(3000),
        Box::<LinearEaseFunction>::default(),
        Duration::ZERO,
        false,
    ));
    tm.add_property("translate-prev", 0.0);
    tm.add_property("translate-next", 0.0);
    tm.add_property("opacity-prev", 0.0);
    tm.add_property("opacity-next", 1.0);
    tm.add_property("blur-prev", BLUR_RADIUS as f64);
    tm.add_property("blur-next", 0.0);
    tm.add_property("scale-prev", MIN_STRETCH);
    tm.add_property("scale-next", 1.0);
}

//init widget info
#[object_subclass]
impl ObjectSubclass for RollingNumberPriv {
    type ParentType = gtk::Container;
    type Type = RollingNumber;

    const NAME: &'static str = "RollingNumber";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("rolling-number");
    }
}

impl Default for RollingNumber {
    fn default() -> Self {
        Self::new()
    }
}

impl RollingNumber {
    pub fn new() -> Self {
        let wid = glib::Object::new::<Self>();
        wid.set_has_window(false);
        wid.inner_label_1().set_parent(&wid);
        wid.inner_label_2().set_parent(&wid);
        wid
    }

    pub fn set_transition_delay(&self, delay: i64, module: bool) -> Result<()> {
        self.imp()
            .local_transition_context
            .borrow_mut()
            .set_transition_delay(delay, module)?;
        self.imp()
            .transition_manager
            .borrow_mut()
            .set_default_delay(Duration::from_millis(delay.unsigned_abs()), delay < 0);
        Ok(())
    }
    pub fn set_transition_duration(&self, duration: u64, module: bool) -> Result<()> {
        self.imp()
            .local_transition_context
            .borrow_mut()
            .set_transition_duration(duration, module)?;
        self.imp()
            .transition_manager
            .borrow_mut()
            .set_default_duration(Duration::from_millis(duration));
        Ok(())
    }
    implement_set_transition!(
        pub,
        local_transition_context,
        translate_prev_transition,
        ["translate-prev"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        scale_prev_transition,
        ["scale-prev"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        opacity_prev_transition,
        ["opacity-prev"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        blur_prev_transition,
        ["blur-prev"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        translate_next_transition,
        ["translate-next"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        scale_next_transition,
        ["scale-next"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        opacity_next_transition,
        ["opacity-next"]
    );
    implement_set_transition!(
        pub,
        local_transition_context,
        blur_next_transition,
        ["blur-next"]
    );
}

impl ContainerImpl for RollingNumberPriv {
    fn add(&self, _widget: &gtk::Widget) {
        glib::g_warning!(
            "warning",
            "you cannot add or remove widgets from RollingNumber"
        );
    }

    fn remove(&self, _widget: &gtk::Widget) {
        glib::g_warning!(
            "warning",
            "you cannot add or remove widgets from RollingNumber"
        );
    }

    fn forall(&self, _: bool, callback: &gtk::subclass::container::Callback) {
        callback.call(self.inner_label_1.borrow().upcast_ref());
        callback.call(self.inner_label_2.borrow().upcast_ref());
    }

    fn child_type(&self) -> glib::Type {
        gtk::Widget::static_type()
    }
}

impl RollingNumberPriv {
    fn get_child_aligned_allocation(&self, child: &gtk::Label) -> gdk::Rectangle {
        let parent_allocation = self.obj().allocation();
        // trace!("parent alloc: ({}, {})", parent_allocation.width(), parent_allocation.height());
        let x = parent_allocation.x() + (parent_allocation.width() - child.preferred_width().0) / 2;
        let y =
            parent_allocation.y() + (parent_allocation.height() - child.preferred_height().0) / 2;

        gtk::Allocation::new(x, y, child.preferred_width().0, child.preferred_height().0)
    }

    fn begin_transition(&self, tm: &mut TransitionManager) {
        let inverted = !*self.first_is_prev.borrow();
        self.first_is_prev.replace(inverted);
        if *self.first_is_prev.borrow() {
            self.inner_label_2
                .borrow()
                .set_text(&self.number.borrow().to_string());
        } else {
            self.inner_label_1
                .borrow()
                .set_text(&self.number.borrow().to_string());
        }
        let max_transition = self.obj().allocation().height() as f64 / (SIZE_MULT_FACTOR * 2.0);
        tm.set_value_from("translate-prev", 0.0, -max_transition * 1.5);
        tm.set_value_from("translate-next", max_transition * 1.5, 0.0);
        tm.set_value_from("opacity-prev", 1.0, 0.0);
        tm.set_value_from("opacity-next", 0.0, 1.0);
        tm.set_value_from("blur-prev", 0.0, BLUR_RADIUS as f64);
        tm.set_value_from("blur-next", BLUR_RADIUS as f64, 0.0);
        tm.set_value_from("scale-prev", 1.0, MIN_STRETCH);
        tm.set_value_from("scale-next", MIN_STRETCH, 1.0);
    }
}

impl WidgetImpl for RollingNumberPriv {
    fn preferred_width_for_height(&self, height: i32) -> (i32, i32) {
        let width_1 = self
            .inner_label_1
            .borrow()
            .preferred_width_for_height(height);
        let width_2 = self
            .inner_label_2
            .borrow()
            .preferred_width_for_height(height);
        (width_1.0.max(width_2.0), width_1.0.max(width_2.0))
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        let mut height_1 = self
            .inner_label_1
            .borrow()
            .preferred_height_for_width(width);
        let mut height_2 = self
            .inner_label_2
            .borrow()
            .preferred_height_for_width(width);
        height_1 = (
            (height_1.0 as f64 * SIZE_MULT_FACTOR) as i32,
            (height_1.1 as f64 * SIZE_MULT_FACTOR) as i32,
        );
        height_2 = (
            (height_2.0 as f64 * SIZE_MULT_FACTOR) as i32,
            (height_2.1 as f64 * SIZE_MULT_FACTOR) as i32,
        );
        (height_1.0.max(height_2.0), height_1.0.max(height_2.0))
    }

    fn preferred_height(&self) -> (i32, i32) {
        let mut height_1 = self.inner_label_1.borrow().preferred_height();
        let mut height_2 = self.inner_label_2.borrow().preferred_height();
        height_1 = (
            (height_1.0 as f64 * SIZE_MULT_FACTOR) as i32,
            (height_1.1 as f64 * SIZE_MULT_FACTOR) as i32,
        );
        height_2 = (
            (height_2.0 as f64 * SIZE_MULT_FACTOR) as i32,
            (height_2.1 as f64 * SIZE_MULT_FACTOR) as i32,
        );
        (height_1.0.max(height_2.0), height_1.0.max(height_2.0))
    }

    fn preferred_width(&self) -> (i32, i32) {
        let width_1 = self.inner_label_1.borrow().preferred_width();
        let width_2 = self.inner_label_2.borrow().preferred_width();
        (width_1.0.max(width_2.0), width_1.0.max(width_2.0))
    }

    fn size_allocate(&self, allocation: &gdk::Rectangle) {
        let prev_inner = &*self.inner_label_1.borrow();
        let next_inner = &*self.inner_label_2.borrow();
        let allocation = &gdk::Rectangle::new(
            allocation.x(),
            allocation.y(),
            ((next_inner.preferred_width().0 as f64 * SIZE_MULT_FACTOR) as i32)
                .max((prev_inner.preferred_width().0 as f64 * SIZE_MULT_FACTOR) as i32),
            ((next_inner.preferred_height().0 as f64 * SIZE_MULT_FACTOR) as i32)
                .max((prev_inner.preferred_height().0 as f64 * SIZE_MULT_FACTOR) as i32),
        );
        self.obj().set_allocation(allocation);
        self.obj().set_clip(allocation);

        let allocation = self.get_child_aligned_allocation(prev_inner);
        prev_inner.size_allocate(&allocation);

        let allocation = self.get_child_aligned_allocation(next_inner);
        next_inner.size_allocate(&allocation);
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        let mut logs: Vec<String> = vec![];
        let start = Instant::now();

        let res: Result<()> = try {
            //draw background
            gtk::render_background(
                &self.obj().style_context(),
                cr,
                0.0,
                0.0,
                self.obj().allocation().width() as f64,
                self.obj().allocation().height() as f64,
            );

            let prev_inner = &*if *self.first_is_prev.borrow() {
                self.inner_label_1.borrow()
            } else {
                self.inner_label_2.borrow()
            };

            let next_inner = &*if *self.first_is_prev.borrow() {
                self.inner_label_2.borrow()
            } else {
                self.inner_label_1.borrow()
            };

            let mut tm = self.transition_manager.borrow_mut();
            let animating = tm.is_running("translate-prev");

            let self_w = self.obj().allocation().width();
            let self_h = self.obj().allocation().height();

            // let translate_prev = tm.get_value("translate-prev").unwrap();
            // debug!("translate_prev: {}", translate_prev);

            if animating {
                // debug!("animating, {}", *self.first_is_prev.borrow());
                let translate_prev = tm.get_value("translate-prev").unwrap();
                let translate_next = tm.get_value("translate-next").unwrap();
                let opacity_prev = tm.get_value("opacity-prev").unwrap() as f32;
                let opacity_next = tm.get_value("opacity-next").unwrap() as f32;
                let blur_prev = tm.get_value("blur-prev").unwrap() as f32;
                let blur_next = tm.get_value("blur-next").unwrap() as f32;
                let stretch_prev = tm.get_value("scale-prev").unwrap();
                let stretch_next = tm.get_value("scale-next").unwrap();
                let (translate_prev_x, translate_prev_y) = (0.0, translate_prev);
                let (translate_next_x, translate_next_y) = (0.0, translate_next);

                let mut prev_surface =
                    gtk::cairo::ImageSurface::create(gdk::cairo::Format::ARgb32, self_w, self_h)
                        .with_context(|| "failed to create new imagesurface")?;
                {
                    let tmp_cr = gdk::cairo::Context::new(&prev_surface)
                        .with_context(|| "failed to retrieve context from tmp surface")?;
                    let prev_scaled_size = (
                        prev_inner.allocated_width() as f64 * (1.0 - stretch_prev),
                        prev_inner.allocated_height() as f64 * (1.0 - stretch_prev),
                    );

                    tmp_cr.translate(
                        translate_prev_x + (prev_scaled_size.0 / 2.0),
                        translate_prev_y + (prev_scaled_size.1 / 2.0),
                    );

                    tmp_cr.scale(stretch_prev, stretch_prev);

                    self.obj().propagate_draw(prev_inner, &tmp_cr);
                }

                let mut next_surface =
                    gtk::cairo::ImageSurface::create(gdk::cairo::Format::ARgb32, self_w, self_h)
                        .with_context(|| "failed to create new imagesurface")?;
                {
                    let tmp_cr = gdk::cairo::Context::new(&next_surface)
                        .with_context(|| "failed to retrieve context from tmp surface")?;
                    let next_size = (
                        next_inner.allocated_width() as f64 * (1.0 - stretch_next),
                        next_inner.allocated_height() as f64 * (1.0 - stretch_next),
                    );

                    tmp_cr.translate(
                        translate_next_x + (next_size.0 / 2.0),
                        translate_next_y + (next_size.1 / 2.0),
                    );

                    tmp_cr.scale(stretch_next, stretch_next);

                    self.obj().propagate_draw(next_inner, &tmp_cr);
                }

                let size = self_w * self_h;
                if size > *filter::benchmark::COMPUTE_BENCHMARK_LIMIT.blocking_lock() {
                    filter::apply_blur_and_merge_opacity_dual(
                        &mut prev_surface,
                        &mut next_surface,
                        blur_prev,
                        blur_next,
                        opacity_prev,
                        opacity_next,
                        filter::FilterBackend::Gpu,
                    )
                    .with_context(|| "failed to apply double blur + merge to tmp surface")?;

                    cr.set_source_surface(
                        &prev_surface,
                        TRANSLATE_CORRECTIVE_FACTOR,
                        TRANSLATE_CORRECTIVE_FACTOR,
                    )
                    .with_context(|| "failed to set source surface")?;

                    cr.paint()
                        .with_context(|| "failed to paint surface to context")?;
                } else {
                    filter::apply_blur_auto(&mut prev_surface, blur_prev + 0.5)
                        .with_context(|| "failed to apply blur to tmp surface")?;
                    filter::apply_blur_auto(&mut next_surface, blur_next + 0.5)
                        .with_context(|| "failed to apply blur to tmp surface")?;

                    cr.set_source_surface(prev_surface, 0.0, 0.0)
                        .with_context(|| "failed to set source surface")?;
                    cr.paint_with_alpha(opacity_prev.into())
                        .with_context(|| "failed to paint surface to context")?;

                    cr.set_source_surface(next_surface, 0.0, 0.0)
                        .with_context(|| "failed to set source surface")?;
                    cr.paint_with_alpha(opacity_next.into())
                        .with_context(|| "failed to paint surface to context")?;
                }

                self.obj().queue_draw();
            } else if tm.is_idle("translate-prev") {
                // debug!("idle, {}", *self.first_is_prev.borrow());
                let dur_to_end = tm.time_to_animating("translate-prev");
                if dur_to_end <= Duration::from_millis(70) {
                    self.obj().queue_draw();
                } else {
                    let wid = self.obj().clone();
                    glib::MainContext::default().spawn_local(async move {
                        glib::timeout_future(dur_to_end - Duration::from_millis(50)).await;
                        wid.queue_draw(); // queue draw for future
                    });
                }
                self.obj().propagate_draw(prev_inner, cr);
            } else {
                // debug!("Completed, {}", *self.first_is_prev.borrow());
                self.obj().propagate_draw(next_inner, cr);
            }

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
