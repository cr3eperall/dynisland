use std::{
    cell::RefCell,
    f64::consts::PI,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use glib::{object_subclass, prelude::*, wrapper};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*, CssProvider};

use super::transition::Transition;

//Add function to set background_widget css

const MINIMAL_HEIGHT: i32 = 40;

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedActivityMode")]
pub enum ActivityMode {
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
    Overlay = 3,
}

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedLocalCssContext")]
pub struct LocalCssContext {
    css_provider: CssProvider,

    size: (i32, i32),
    border_radius: i32,
}

impl LocalCssContext {
    pub fn new() -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            size: (MINIMAL_HEIGHT, MINIMAL_HEIGHT),
            border_radius: 100,
        }
    }

    pub fn get_css_provider(&self) -> CssProvider {
        self.css_provider.clone()
    }
    pub fn get_size(&self) -> (i32, i32) {
        self.size
    }
    pub fn get_border_radius(&self) -> i32 {
        self.border_radius
    }

    pub fn set_size(&mut self, size: (i32, i32)) -> Result<()> {
        if self.size == size {
            return Ok(());
        };
        self.size = size;
        self.update_provider()
    }
    pub fn set_border_radius(&mut self, border_radius: i32) -> Result<()> {
        if self.border_radius == border_radius {
            return Ok(());
        };
        self.border_radius = border_radius;
        self.update_provider()
    }

    fn update_provider(&self) -> Result<()> {
        let (w, h) = self.size;
        let border_radius = self.border_radius;
        self.css_provider
            .load_from_data(
                format!(
                    r".activity-background{{ 
                        min-width: {w}px; 
                        min-height: {h}px; 
                    }}
                    .mode-compact{{
                        border-radius: {border_radius}px;
                    }}
                    .mode-minimal{{
                        border-radius: {border_radius}px;
                    }}
                    .mode-expanded{{
                        border-radius: {border_radius}px;
                    }}
                    .mode-overlay{{
                        border-radius: {border_radius}px;
                    }}"
                )
                .as_bytes(),
            )
            .with_context(|| "failed to update css provider data")
    }
}

impl Default for LocalCssContext {
    fn default() -> Self {
        Self::new()
    }
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

    #[property(
        get,
        set,
        nick = "Change Transition Duration",
        blurb = "The Duration of the Transition"
    )]
    transition_duration: RefCell<u64>,

    #[property(get, nick = "Local CSS Provider")]
    local_css_context: RefCell<LocalCssContext>,

    last_mode: RefCell<ActivityMode>,

    transition: RefCell<Transition>,

    background_widget: RefCell<Option<gtk::Widget>>,

    minimal_mode_widget: RefCell<Option<gtk::Widget>>,

    compact_mode_widget: RefCell<Option<gtk::Widget>>,

    expanded_mode_widget: RefCell<Option<gtk::Widget>>,

    overlay_mode_widget: RefCell<Option<gtk::Widget>>,
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
                x = parent_allocation.x() + (parent_allocation.width() - width) / 2;
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
}

//default data
impl Default for ActivityWidgetPriv {
    fn default() -> Self {
        Self {
            mode: RefCell::new(ActivityMode::Minimal),
            transition_duration: RefCell::new(0),
            local_css_context: RefCell::new(LocalCssContext::new()),
            last_mode: RefCell::new(ActivityMode::Minimal),
            transition: RefCell::new(Transition::new(Instant::now(), Duration::ZERO)),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

//set properties
#[glib::derived_properties]
impl ObjectImpl for ActivityWidgetPriv {
    // fn signals() -> &'static [glib::subclass::Signal] { //TODO check if it's really necessary
    //     static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
    //         vec![Signal::builder("scheduled-clock")
    //         .param_types([i32::static_type()])
    //         .run_first()
    //         .build()]
    //     });
    //     SIGNALS.as_ref()
    // }

    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "mode" => {
                self.last_mode.replace(self.mode.borrow().clone());
                self.mode.replace(value.get().unwrap());
                let start: Instant;
                let duration: Duration;
                if self.transition.borrow().is_active() {
                    duration = Duration::from_millis(*self.transition_duration.borrow())
                        + self.transition.borrow().duration_to_end();
                    start = Instant::now()
                        .checked_sub(self.transition.borrow().duration_to_end())
                        .expect("time error");
                } else {
                    start = Instant::now();
                    duration = Duration::from_millis(*self.transition_duration.borrow());
                }

                self.transition.replace(Transition::new(start, duration));

                if let Some(widget) = &*self.get_mode_widget(self.mode.borrow().clone()).borrow() {
                    let height = match *self.mode.borrow() {
                        ActivityMode::Minimal | ActivityMode::Compact => MINIMAL_HEIGHT,
                        ActivityMode::Expanded | ActivityMode::Overlay => {
                            widget.allocation().height()
                        }
                    };
                    self.local_css_context
                        .borrow_mut()
                        .set_size((widget.allocation().width(), height))
                        .expect("failed to set activity size");
                }
                self.obj().queue_draw(); // Queue a draw call with the updated value
            }
            "transition-duration" => {
                self.transition_duration.replace(value.get().unwrap());
            }
            x => panic!("Tried to set inexistant property of ActivityWidget: {}", x),
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}

//init widget info
#[object_subclass]
impl ObjectSubclass for ActivityWidgetPriv {
    type ParentType = gtk::Container;
    type Type = ActivityWidget;

    const NAME: &'static str = "ActivityWidget";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("activity-widget"); //TODO change css class to unique identifier
    }
}

impl Default for ActivityWidget {
    fn default() -> Self {
        Self::new()
    }
}

//set mode widgets and get new instance
impl ActivityWidget {
    pub fn new() -> Self {
        let wid = glib::Object::new::<Self>();
        wid.set_has_window(false);

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().unwrap(),
            &wid.imp().local_css_context.borrow().get_css_provider(),
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
            self.local_css_context()
                .set_size((widget.width_request(), MINIMAL_HEIGHT))
                .expect("failed to set activity size");
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
            self.local_css_context()
                .set_size((widget.width_request(), MINIMAL_HEIGHT))
                .expect("failed to set activity size");
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
            self.local_css_context()
                .set_size((widget.width_request(), widget.height_request()))
                .expect("failed to set activity size");
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
            self.local_css_context()
                .set_size((widget.width_request(), widget.height_request()))
                .expect("failed to set activity size");
        }
        self.queue_draw(); // Queue a draw call with the updated widget
    }
}

//add/remove bg_widget and expose info to GTK debugger
impl ContainerImpl for ActivityWidgetPriv {
    fn add(&self, widget: &gtk::Widget) {
        if let Some(bg_widget) = &*self.background_widget.borrow() {
            bg_widget
                .style_context()
                .remove_class("activity-background"); //TODO change css class to unique identifier
            bg_widget.unparent();
        }
        widget.set_parent(self.obj().as_ref());
        widget.style_context().add_class("activity-background"); //TODO change css class to unique identifier
        self.background_widget.replace(Some(widget.clone()));
    }

    fn remove(&self, widget: &gtk::Widget) {
        if let Some(bg_widget) = &*self.background_widget.borrow() {
            if bg_widget != widget {
                glib::g_warning!("warning", "{widget} was not inside this container");
            } else {
                bg_widget
                    .style_context()
                    .remove_class("activity-background"); //TODO change css class to unique identifier
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
        self.obj().set_allocation(allocation);

        if let Some(content) = &*self.background_widget.borrow() {
            content.size_allocate(allocation);
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
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        let res: Result<()> = try {
            // println!("{}",self.local_css_provider.borrow().to_str());
            let bg_color: gdk::RGBA = self
                .obj()
                .style_context()
                .style_property_for_state("background-color", gtk::StateFlags::NORMAL)
                .get()?;
            cr.save()?;

            cr.move_to(
                self.obj().allocation().x() as f64,
                self.obj().allocation().y() as f64,
            );

            let border_radius: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-radius", gtk::StateFlags::NORMAL)
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

            //draw bckground widget
            if let Some(bg_widget) = &*self.background_widget.borrow() {
                self.obj().propagate_draw(bg_widget, cr);
            }

            //draw active mode widget
            let widget_to_render = self.get_mode_widget(self.mode.borrow().clone());

            //animate blur and opacity if during transition
            if self.transition.borrow().is_active() {
                let progress = self.transition.borrow().get_progress();
                // println!("{}, start: {:?}, dur: {:?}",progress, self.transition.borrow().start_time.elapsed(), self.transition.borrow().duration);
                let last_widget_to_render = self.get_mode_widget(self.last_mode.borrow().clone());

                const RAD: f32 = 4.0;
                const N: usize = 5;

                if let Some(widget) = &*last_widget_to_render.borrow() {
                    let mut tmp_surface = gtk::cairo::ImageSurface::create(
                        gdk::cairo::Format::ARgb32,
                        self.obj().allocation().width(),
                        self.obj().allocation().height(),
                    )
                    .with_context(|| "failed to create new imagesurface")?;

                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    self.obj().propagate_draw(widget, &tmp_cr);

                    drop(tmp_cr);

                    let blurred_surface = crate::filter::apply_blur(
                        &mut tmp_surface,
                        soy::Lerper::calculate(&soy::EASE_OUT, progress) * RAD,
                        N,
                    )
                    .with_context(|| "failed to apply blur to tmp surface")?;

                    cr.set_source_surface(&blurred_surface, 0.0, 0.0)
                        .with_context(|| "failed to set source surface")?;

                    cr.paint_with_alpha(
                        soy::Lerper::calculate(&soy::EASE_OUT, 1.0 - progress) as f64
                    )
                    .with_context(|| "failed to paint surface to context")?;
                }

                if let Some(widget) = &*widget_to_render.borrow() {
                    let mut tmp_surface = gtk::cairo::ImageSurface::create(
                        gdk::cairo::Format::ARgb32,
                        self.obj().allocation().width(),
                        self.obj().allocation().height(),
                    )
                    .with_context(|| "failed to create new imagesurface")?;

                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    self.obj().propagate_draw(widget, &tmp_cr);

                    drop(tmp_cr);

                    let blurred_surface = crate::filter::apply_blur(
                        &mut tmp_surface,
                        soy::Lerper::calculate(&soy::EASE_IN, 1.0 - progress) * RAD,
                        N,
                    )
                    .with_context(|| "failed to apply blur to tmp surface")?;

                    cr.set_source_surface(&blurred_surface, 0.0, 0.0)
                        .with_context(|| "failed to set source surface")?;

                    cr.paint_with_alpha(soy::Lerper::calculate(&soy::EASE_IN, progress) as f64)
                        .with_context(|| "failed to paint surface to context")?;
                }
            } else if let Some(widget) = &*widget_to_render.borrow() {
                self.obj().propagate_draw(widget, cr);
            }

            //TODO implement later
            // let border_color: gdk::RGBA =self.obj().style_context().style_property_for_state("border-color",gtk::StateFlags::NORMAL).get()?;
            // // let border_width: i32 =self.obj().style_context().style_property_for_state("border-width",gtk::StateFlags::NORMAL).get()?;

            // cr.arc(radius, radius, radius, PI*1.0, PI*1.5); //top left //WHY are the angles rotated by 90 degrees
            // cr.line_to(self.obj().allocated_width() as f64-radius,0.0);
            // cr.arc(self.obj().allocated_width()as f64-radius, radius, radius, PI*1.5, PI*0.0); //top right
            // cr.line_to(self.obj().allocated_width()as f64, self.obj().allocated_height() as f64-radius);
            // cr.arc(self.obj().allocated_width()as f64-radius, self.obj().allocated_height() as f64-radius, radius, PI*0.0, PI*0.5); //bottom right
            // cr.line_to(radius,self.obj().allocated_height()as f64);
            // cr.arc(radius, self.obj().allocated_height() as f64-radius, radius, PI*0.5, PI*1.0); //bottom left
            // cr.line_to(0.0, radius);
            // cr.line_cap();
            // cr.set_source_rgba(border_color.red(), border_color.green(), border_color.blue(), border_color.alpha());
            // cr.set_line_width(10 as f64);
            // cr.stroke()?;

            self.transition.borrow_mut().update_active();

            //reset
            cr.reset_clip();

            cr.restore()?;
        };

        if let Err(err) = res {
            eprintln!("{err}");
        }

        glib::Propagation::Proceed
    }
}

fn get_max_preferred_size(m1: (i32, i32), m2: (i32, i32)) -> (i32, i32) {
    (std::cmp::max(m1.0, m2.0), std::cmp::max(m1.1, m2.1))
}
