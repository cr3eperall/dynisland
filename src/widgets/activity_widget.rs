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

#[derive(Clone, glib::Boxed)]
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

    #[property(
        get,
        set,
        nick = "Change Transition Duration",
        blurb = "The Duration of the Transition"
    )]
    transition_duration: RefCell<u64>,

    #[property(get, nick = "Local CSS Provider")]
    local_css_provider: RefCell<CssProvider>,

    last_mode: RefCell<ActivityMode>,

    transition: RefCell<Transition>,

    size_initialized: RefCell<bool>,

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
            local_css_provider: RefCell::new(gtk::CssProvider::new()),
            last_mode: RefCell::new(ActivityMode::Minimal),
            transition: RefCell::new(Transition::new(Instant::now(), Duration::ZERO)),
            size_initialized: RefCell::new(false),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

// pub fn default_background_widget() -> RefCell<Option<gtk::Widget>> {
//     let widget = gtk::Box::builder().vexpand(false).hexpand(false).build();
//     RefCell::new(Some(widget.upcast()))
// }

//set properties
impl ObjectImpl for ActivityWidgetPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "mode" => {
                self.last_mode.replace(self.mode.borrow().clone());
                self.mode.replace(value.get().unwrap());
                self.transition.replace(Transition::new(
                    Instant::now(),
                    Duration::from_millis(*self.transition_duration.borrow()),
                ));

                if let Some(widget) = &*self.get_mode_widget(self.mode.borrow().clone()).borrow() {
                    let (w, h) = (widget.allocation().width(), widget.allocation().height());
                    self.local_css_provider
                        .borrow()
                        .load_from_data(
                            format!(
                                ".activity-background{{ min-width: {w}px; min-height: {h}px; }}"
                            )
                            .as_bytes(),
                        )
                        .unwrap(); //TODO change css class to unique identifier
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
            &*wid.imp().local_css_provider.borrow(),
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
            let (w, h) = (widget.width_request(), widget.height_request());
            self.local_css_provider()
                .load_from_data(
                    format!(".activity-background{{ min-width: {w}px; min-height: {h}px; }}")
                        .as_bytes(),
                )
                .unwrap(); //TODO change css class to unique identifier
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
            let (w, h) = (widget.width_request(), widget.height_request());
            self.local_css_provider()
                .load_from_data(
                    format!(".activity-background{{ min-width: {w}px; min-height: {h}px; }}")
                        .as_bytes(),
                )
                .unwrap(); //TODO change css class to unique identifier
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
            let (w, h) = (widget.width_request(), widget.height_request());
            self.local_css_provider()
                .load_from_data(
                    format!(".activity-background{{ min-width: {w}px; min-height: {h}px; }}")
                        .as_bytes(),
                )
                .unwrap(); //TODO change css class to unique identifier
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
            let (w, h) = (widget.width_request(), widget.height_request());
            self.local_css_provider()
                .load_from_data(
                    format!(".activity-background{{ min-width: {w}px; min-height: {h}px; }}")
                        .as_bytes(),
                )
                .unwrap(); //TODO change css class to unique identifier
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
        self.size_initialized.replace(false);
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
            _ => (0, 0),
        }
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_height_for_width(width), (0, width))
            }
            _ => (0, 0),
        }
    }

    fn preferred_height(&self) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_height(),
            _ => (0, 0),
        }
    }

    fn preferred_width(&self) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_width(),
            _ => (0, 0),
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
                // println!("{progress}");
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

                    cr.paint_with_alpha(1.0 - progress as f64)
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

                    cr.paint_with_alpha(progress as f64)
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
