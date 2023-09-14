use std::{cell::RefCell, f64::consts::PI, time::{Duration, Instant}};

use glib::{object_subclass, wrapper, prelude::*};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*, CssProvider};
use anyhow::{Result, Context};

wrapper! {
    pub struct ActivityWidget(ObjectSubclass<ActivityWidgetPriv>)
    @extends gtk::Container, gtk::Widget;
}
#[derive(Clone, glib::Boxed)]
#[boxed_type(name = "BoxedActivityMode")]
pub enum ActivityMode {
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
    Overlay = 3,
}


pub struct Transition{
    pub active: bool,
    pub duration: Duration,
    pub start_time: Instant,
}

impl Transition {
    pub fn is_active(&self) -> bool {
        self.active && Instant::now() < self.start_time + self.duration
    }

    pub fn get_progress(&self) -> f32 {
        self.start_time.elapsed()
        .div_duration_f32(self.duration).clamp(0.0, 1.0)
    }

    pub fn new(start_time: Instant, duration: Duration) -> Self {
        Transition { active: start_time<=Instant::now(), duration, start_time }
    }

    pub fn update_active(&mut self) {
        if let Some(dur) = Instant::now().checked_duration_since(self.start_time){
            if dur < self.duration {
                self.active=true;
                return;
            }
        }
        self.active = false;
    }
}

#[derive(Properties)]
#[properties(wrapper_type = ActivityWidget)]
pub struct ActivityWidgetPriv {
    #[property(get, set, nick = "Change mode", blurb = "The Activity Mode")]
    mode: RefCell<ActivityMode>,

    #[property(get, set, nick = "Change Transition Duration", blurb = "The Duration of the Transition")]
    transition_duration: RefCell<u64>,

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

impl Default for ActivityWidgetPriv {
    fn default() -> Self {
        Self {
            mode: RefCell::new(ActivityMode::Minimal),
            transition_duration: RefCell::new(0),
            local_css_provider: RefCell::new(gtk::CssProvider::new()),
            last_mode: RefCell::new(ActivityMode::Minimal),
            transition: RefCell::new(Transition::new(Instant::now(), Duration::ZERO)),
            size_initialized:RefCell::new(false),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

pub fn default_background_widget() -> RefCell<Option<gtk::Widget>> {
    let widget = gtk::Box::builder().vexpand(false).hexpand(false).build();
    RefCell::new(Some(widget.upcast()))
}

impl ObjectImpl for ActivityWidgetPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "mode" => {
                self.last_mode.replace(self.mode.borrow().clone());
                self.mode.replace(value.get().unwrap());
                self.transition.replace(Transition::new(Instant::now(), Duration::from_millis(*self.transition_duration.borrow())));
                self.obj().queue_draw(); // Queue a draw call with the updated value
            },
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

#[object_subclass]
impl ObjectSubclass for ActivityWidgetPriv {
    type ParentType = gtk::Container;
    type Type = ActivityWidget;

    const NAME: &'static str = "ActivityWidget";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("activity-widget");
    }
}

impl ActivityWidget {
    pub fn new() -> Self {
        let wid=glib::Object::new::<Self>();
        wid.set_has_window(false);
        wid
    }
    pub fn add_minimal_mode(&self, widget: &gtk::Widget){
        let priv_= self.imp();
        if let Some(content) = &*priv_.minimal_mode_widget.borrow() {
            // print_error(anyhow!("Error, trying to add multiple backgrounds to an activity widget"));
            content.unparent();
            // unsafe { content.destroy() };
        }
        // self.create_child_window(widget);
        widget.set_parent(self);
        priv_.minimal_mode_widget.replace(Some(widget.clone()));
    }
    pub fn add_compact_mode(&self, widget: &gtk::Widget){
        let priv_= self.imp();
        if let Some(content) = &*priv_.compact_mode_widget.borrow() {
            // print_error(anyhow!("Error, trying to add multiple backgrounds to an activity widget"));
            content.unparent();
            // unsafe { content.destroy() };
        }
        // self.create_child_window(widget);
        widget.set_parent(self);
        priv_.compact_mode_widget.replace(Some(widget.clone()));
    }
}


fn get_max_preferred(m1: (i32,i32),m2:(i32,i32)) -> (i32,i32){
    (std::cmp::max(m1.0, m2.0),std::cmp::max(m1.1, m2.1))
}

fn get_child_aligned_allocation(child: &gtk::Widget, parent_allocation: &gdk::Rectangle) -> gdk::Rectangle {
    let x: i32;
            let y:i32;
            let mut width=child.preferred_width().0;
            let mut height=child.preferred_height().0;
            match child.halign(){
                gtk::Align::Start => {
                    x=parent_allocation.x();
                },
                gtk::Align::End => {
                    x=parent_allocation.x()+(parent_allocation.width()-width);
                },
                gtk::Align::Center => {
                    x=parent_allocation.x()+(parent_allocation.width()-width)/2;
                },
                _ => {
                    glib::g_warning!("warning","this will not work for animated resizing");
                    x=parent_allocation.x();
                    width=parent_allocation.width();
                },
            }
            match child.valign(){
                gtk::Align::Start => {
                    y=parent_allocation.y();
                },
                gtk::Align::End => {
                    y=parent_allocation.y()+(parent_allocation.height()-height);
                },
                gtk::Align::Center => {
                    y=parent_allocation.y()+(parent_allocation.height()-height)/2;
                },
                _ => {
                    glib::g_warning!("warning","this will not work for animated resizing");
                    y=parent_allocation.y();
                    height=parent_allocation.height();
                },
            } //TODO change x and y to reflect v and halign
            gtk::Allocation::new(x,y, width, height)
}
// impl BinImpl for ActivityWidgetPriv {}
impl ContainerImpl for ActivityWidgetPriv {
    fn add(&self, widget: &gtk::Widget) {
        if self.background_widget.borrow().is_some() {
            // print_error(anyhow!("Error, trying to add multiple backgrounds to an activity widget"));
            widget.unparent();
        }
        self.size_initialized.replace(false);
        widget.set_parent(self.obj().as_ref());
        self.background_widget.replace(Some(widget.clone()));
    }
    
    fn forall(&self, _: bool, callback: &gtk::subclass::container::Callback) {
        if let Some(content) = &*self.background_widget.borrow(){
            callback.call(content);
        }
        if let Some(content) = &*self.minimal_mode_widget.borrow(){
            callback.call(content);
        }
        if let Some(content) = &*self.compact_mode_widget.borrow(){
            callback.call(content);
        }
        if let Some(content) = &*self.expanded_mode_widget.borrow(){
            callback.call(content);
        }
        if let Some(content) = &*self.overlay_mode_widget.borrow(){
            callback.call(content);
        }
    }
    
    fn remove(&self, widget: &gtk::Widget) {
        if self.background_widget.borrow().as_ref() != Some(widget) {
            glib::g_warning!("warning","{widget} was not inside this container");
            return;
        }
    }
    
    fn child_type(&self) -> glib::Type {
        match &*self.background_widget.borrow() {
            Some(_) => glib::Type::UNIT,
            None => gtk::Widget::static_type(),
        }
    }
}

impl WidgetImpl for ActivityWidgetPriv {
    fn preferred_width_for_height(&self, height: i32) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => get_max_preferred(content.preferred_width_for_height(height),(height,height)),
            _ => (0, 0),
        }
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => get_max_preferred(content.preferred_height_for_width(width),(0,width)),
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
        self.obj().set_allocation(&allocation);

        if let Some(content) = &*self.background_widget.borrow(){
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.minimal_mode_widget.borrow(){
            let allocation = get_child_aligned_allocation(content, allocation);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.compact_mode_widget.borrow(){
            let allocation = get_child_aligned_allocation(content, allocation);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.expanded_mode_widget.borrow(){
            let allocation = get_child_aligned_allocation(content, allocation);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.overlay_mode_widget.borrow(){
            let allocation = get_child_aligned_allocation(content, allocation);
            content.size_allocate(&allocation);
        }
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        let res: Result<()> = try {
            let bg_color: gdk::RGBA = self.obj().style_context().style_property_for_state("background-color", gtk::StateFlags::NORMAL).get()?;
            cr.save()?;

            cr.move_to(self.obj().allocation().x() as f64, self.obj().allocation().y() as f64);

            let radius=self.obj().allocated_height() as f64/2f64;

            //draw background
            cr.rectangle(0.0, 0.0, self.obj().allocated_width()as f64, self.obj().allocated_height()as f64);
            cr.set_source_rgba(bg_color.red(), bg_color.green(), bg_color.blue(), bg_color.alpha()); //TODO should always be transparent
            cr.fill()?;

            //setup clip
            cr.arc(radius, radius, radius, PI*0.5f64, PI*1.5f64);
            cr.line_to(self.obj().allocated_width()as f64-radius, 0.0);
            cr.arc(self.obj().allocated_width()as f64-radius, radius, radius, PI*1.5f64, PI*0.5f64);
            cr.line_to(radius, radius*2f64);
            cr.set_source_rgba(0.0, 0.0, 0.0, bg_color.alpha());
            cr.clip();

            if let Some(bg_widget) = &*self.background_widget.borrow() {
                //draw bckground widget
                self.obj().propagate_draw(bg_widget, &cr);
            }
            
            //draw active mode widget
            let widget_to_render = match *self.mode.borrow() {
                ActivityMode::Minimal => &self.minimal_mode_widget,
                ActivityMode::Compact => &self.compact_mode_widget,
                ActivityMode::Expanded => &self.expanded_mode_widget,
                ActivityMode::Overlay => &self.overlay_mode_widget,
            };

            if self.transition.borrow().is_active() {
                let progress= self.transition.borrow().get_progress();
                println!("{progress}");
                let last_widget_to_render = match *self.last_mode.borrow() {
                    ActivityMode::Minimal => &self.minimal_mode_widget,
                    ActivityMode::Compact => &self.compact_mode_widget,
                    ActivityMode::Expanded => &self.expanded_mode_widget,
                    ActivityMode::Overlay => &self.overlay_mode_widget,
                };
                
                const RAD:f32=4.0;
                const N:usize=5;

                if let Some(widget) = &*last_widget_to_render.borrow() {
                    // println!("rendering widget");
                    let mut tmp_surface = gtk::cairo::ImageSurface::create(gdk::cairo::Format::ARgb32, self.obj().allocation().width(), self.obj().allocation().height())
                    .with_context(||"failed to create new imagesurface")?;
    
                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
                        .with_context(||"failed to retrieve context from tmp surface")?;
    
                    self.obj().propagate_draw(widget, &tmp_cr);
                    drop(tmp_cr);
                    

                    let blurred_surface = crate::filter::apply_blur(&mut tmp_surface,soy::Lerper::calculate(&soy::EASE_OUT, progress)*RAD, N)
                        .with_context(||"failed to apply blur to tmp surface")?;

                    cr.set_source_surface(&blurred_surface, self.obj().allocation().x() as f64, self.obj().allocation().y() as f64)
                    .with_context(||"failed to set source surface")?;
                    
                    cr.paint_with_alpha(1.0-progress as f64).with_context(||"failed to paint surface to context")?;
                }

                if let Some(widget) = &*widget_to_render.borrow() {
                    // println!("rendering widget");
                    let mut tmp_surface = gtk::cairo::ImageSurface::create(gdk::cairo::Format::ARgb32, self.obj().allocation().width(), self.obj().allocation().height())
                    .with_context(||"failed to create new imagesurface")?;
    
                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface)
                        .with_context(||"failed to retrieve context from tmp surface")?;
    
                    self.obj().propagate_draw(widget, &tmp_cr);
                    
                    drop(tmp_cr);
    
                    let blurred_surface = crate::filter::apply_blur(&mut tmp_surface,soy::Lerper::calculate(&soy::EASE_IN, 1.0-progress)*RAD, N)
                        .with_context(||"failed to apply blur to tmp surface")?;

                    cr.set_source_surface(&blurred_surface, self.obj().allocation().x() as f64, self.obj().allocation().y() as f64)
                    .with_context(||"failed to set source surface")?;
                    
                    cr.paint_with_alpha(progress as f64).with_context(||"failed to paint surface to context")?;
                }

                
            }else{
                if let Some(widget) = &*widget_to_render.borrow() {
                    self.obj().propagate_draw(widget, &cr);
                }
            }
            self.transition.borrow_mut().update_active();

            
            // let start = Instant::now();

            // let duration = start.elapsed();
            // println!("{:?}", duration);
        
        //reset
        cr.reset_clip(); //DONE extract clip to after source surface paint
        
        cr.restore()?;
        
        //DONE get data from surface, blur, load in cr.source and paint
        
    };
    
        if let Err(err) = res {
            eprintln!("{err}");
        }

        glib::Propagation::Proceed
    }
}
