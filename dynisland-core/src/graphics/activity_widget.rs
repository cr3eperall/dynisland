use css_anim::{soy::EaseFunction, transition::{TransitionManager, TransitionDef}};
use log::{debug, error};
use rand::{distributions::Alphanumeric, Rng};
use std::{
    cell::RefCell,
    f64::consts::PI,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use glib::{object_subclass, prelude::*, wrapper};
use glib_macros::Properties;
use gtk::{prelude::*, subclass::prelude::*};

use crate::filters::filter::FilterBackend;

use super::{
    activity_widget_local_css_context::ActivityWidgetLocalCssContext,
    // animations::transition::Transition,
};

const BLUR_RADIUS: f32 = 6.0;

#[derive(Clone, glib::Boxed, Debug, Copy)]
#[boxed_type(name = "BoxedActivityMode")]
pub enum ActivityMode {
    Minimal = 0,
    Compact = 1,
    Expanded = 2,
    Overlay = 3,
}

impl ToString for ActivityMode {
    fn to_string(&self) -> String {
        match self {
            ActivityMode::Minimal => "minimal".to_string(),
            ActivityMode::Compact => "compact".to_string(),
            ActivityMode::Expanded => "expanded".to_string(),
            ActivityMode::Overlay => "overlay".to_string(),
        }
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

    #[property(get, nick = "Local CSS Provider")]
    local_css_context: RefCell<ActivityWidgetLocalCssContext>,

    #[property(get, set, nick = "Widget name")]
    name: RefCell<String>,

    last_mode: RefCell<ActivityMode>,

    transition_manager: RefCell<TransitionManager>,

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
                self.last_mode.replace(*self.mode.borrow());
                self.mode.replace(value.get().unwrap());
                
                let mode=self.mode.borrow();
                let last_mode=self.last_mode.borrow();

                let mut prev_size=(self.obj().allocated_width() as f64, self.obj().allocated_height() as f64);
                if let Some(widget) = &*self.get_mode_widget(*last_mode).borrow() {
                    let tmp=get_final_widget_size(
                        widget,
                        *self.last_mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    );
                    prev_size=(tmp.0 as f64, tmp.1 as f64);
                }
                let mut next_size=(self.obj().allocated_width() as f64, self.obj().allocated_height() as f64);
                if let Some(widget) = &*self.get_mode_widget(*mode).borrow() {
                    let tmp=get_final_widget_size(
                        widget,
                        *self.mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    );
                    next_size=(tmp.0 as f64, tmp.1 as f64);
                }
                let mut css_context=self.local_css_context.borrow_mut();
                let bigger = next_size.0 > prev_size.0 || next_size.1 > prev_size.1;

                self.transition_opacity(false, bigger, &css_context, 1.0, 0.0);
                self.transition_opacity(true, bigger, &css_context, 0.0, 1.0);

                self.transition_blur(false, bigger, &css_context, 0.0, BLUR_RADIUS as f64);
                self.transition_blur(true, bigger, &css_context, BLUR_RADIUS as f64, 0.0);

                self.transition_stretch(false, bigger, &css_context, (1.0,1.0), (next_size.0/prev_size.0,next_size.1/prev_size.1));
                self.transition_stretch(true, bigger, &css_context, (prev_size.0/next_size.0, prev_size.1/next_size.1), (1.0,1.0));

                self.raise_windows();
                if self.get_mode_widget(*self.mode.borrow()).borrow().is_some() {
                    css_context
                        .set_size((next_size.0 as i32, next_size.1 as i32))
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
        let mut transition_manager=TransitionManager::new(false);
        init_transition_properties(&mut transition_manager);


        Self {
            mode: RefCell::new(ActivityMode::Minimal),
            // transition_duration: RefCell::new(0),
            local_css_context: RefCell::new(ActivityWidgetLocalCssContext::new(&name)),
            last_mode: RefCell::new(ActivityMode::Minimal),
            name: RefCell::new(name),
            transition_manager: RefCell::new(transition_manager),
            minimal_mode_widget: RefCell::new(None),
            compact_mode_widget: RefCell::new(None),
            expanded_mode_widget: RefCell::new(None),
            overlay_mode_widget: RefCell::new(None),
            background_widget: RefCell::new(None),
        }
    }
}

fn init_transition_properties(transition_manager: &mut TransitionManager) {
    transition_manager.add_property("minimal-opacity", 1.0);
    transition_manager.add_property("minimal-blur", 0.0);
    transition_manager.add_property("minimal-stretch-x", 1.0);
    transition_manager.add_property("minimal-stretch-y", 1.0);

    transition_manager.add_property("compact-opacity", 0.0);
    transition_manager.add_property("compact-blur", BLUR_RADIUS as f64);
    transition_manager.add_property("compact-stretch-x", 1.0);
    transition_manager.add_property("compact-stretch-y", 1.0);

    transition_manager.add_property("expanded-opacity", 0.0);
    transition_manager.add_property("expanded-blur", BLUR_RADIUS as f64);
    transition_manager.add_property("expanded-stretch-x", 1.0);
    transition_manager.add_property("expanded-stretch-y", 1.0);

    transition_manager.add_property("overlay-opacity", 0.0);
    transition_manager.add_property("overlay-blur", BLUR_RADIUS as f64);
    transition_manager.add_property("overlay-stretch-x", 1.0);
    transition_manager.add_property("overlay-stretch-y", 1.0);
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
    
    fn raise_windows(&self) {
        if let Some(widget) = &*self.get_mode_widget(*self.mode.borrow()).borrow() {
            if let Some(widget) = &*self
                .get_mode_widget(*self.last_mode.borrow())
                .borrow()
            {
                match widget.window() {
                    //lower previous window associated to widget if it has one, this "disables" events on the last mode widget
                    Some(window) => window.lower(),
                    None => {
                        // debug!("no window");
                    }
                }
            }
            if let Some(widget) = &*self.background_widget.borrow() {
                match widget.window() {
                    //raise background widget's window if it has one, this creates a default if the new widget doesn't have a window
                    Some(window) => window.raise(),
                    None => {
                        // debug!("no window");
                    }
                }
            }
            match widget.window() {
                //raise window associated to widget if it has one, this "enables" events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
                }
            }
        }
    }

    fn transition_opacity(&self, next_mode:bool, bigger:bool, css_context:&ActivityWidgetLocalCssContext, from:f64,to:f64){
        let duration=Duration::from_millis(css_context.get_transition_duration());
        let opacity=if next_mode{self.mode.borrow()}else{self.last_mode.borrow()}.to_string()+"-opacity";
        self.transition_manager.borrow_mut().set_value_with_transition_from(&opacity, from, to,&TransitionDef::new(duration, 
        if bigger ^ next_mode {
            css_context.get_transition_bigger_opacity()
        } else {
            css_context.get_transition_smaller_opacity()
        }, Duration::ZERO, false));
    }
    fn transition_blur(&self, next_mode:bool, bigger:bool, css_context:&ActivityWidgetLocalCssContext, from:f64,to:f64){
        let duration=Duration::from_millis(css_context.get_transition_duration());
        let blur=if next_mode{self.mode.borrow()}else{self.last_mode.borrow()}.to_string()+"-blur";
        self.transition_manager.borrow_mut().set_value_with_transition_from(&blur, from, to,&TransitionDef::new(duration, 
        if bigger ^ next_mode {
            css_context.get_transition_bigger_blur()
        } else {
            css_context.get_transition_smaller_blur()
        }, Duration::ZERO, false));
    }
    fn transition_stretch(&self, next_mode:bool, bigger:bool, css_context:&ActivityWidgetLocalCssContext, from:(f64,f64),to:(f64,f64)){
        let duration=Duration::from_millis(css_context.get_transition_duration());
        let stretch_x=if next_mode{self.mode.borrow()}else{self.last_mode.borrow()}.to_string()+"-stretch-x";
        let stretch_y=if next_mode{self.mode.borrow()}else{self.last_mode.borrow()}.to_string()+"-stretch-y";
        self.transition_manager.borrow_mut().set_value_with_transition_from(&stretch_x, from.0, to.0,&TransitionDef::new(duration, 
        if bigger ^ next_mode {
            css_context.get_transition_bigger_stretch()
        } else {
            css_context.get_transition_smaller_stretch()
        }, Duration::ZERO, false));
        self.transition_manager.borrow_mut().set_value_with_transition_from(&stretch_y, from.1, to.1,&TransitionDef::new(duration, 
        if bigger ^ next_mode {
            css_context.get_transition_bigger_stretch()
        } else {
            css_context.get_transition_smaller_stretch()
        }, Duration::ZERO, false));
    }
}

//init widget info
#[object_subclass]
impl ObjectSubclass for ActivityWidgetPriv {
    type ParentType = gtk::Container;
    type Type = ActivityWidget;

    const NAME: &'static str = "ActivityWidget";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("activity-widget");
        // unsafe {
        //     let widget_class = klass as *mut _ as *mut gtk::ffi::GtkWidgetClass;
        //     let mut n: c_uint=0;
        //     let out = gtk::ffi::gtk_widget_class_list_style_properties(widget_class, &mut n);
        //     let sl = std::slice::from_raw_parts_mut(out, n.try_into().unwrap());
        //     for prop in sl {
        //         let pspec=glib::ParamSpec::from_glib_ptr_borrow(&prop.cast_const());
        //      }

        //     gtk::ffi::gtk_widget_class_install_style_property(klass, pspec)
        // }
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
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
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
            let (width, height) = get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger=width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(true, bigger, &self.local_css_context(), (self.allocated_width() as f64/width as f64, self.allocated_height() as f64/height as f64), (1.0,1.0));
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
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
            let (width, height) = get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger=width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(true, bigger, &self.local_css_context(), (self.allocated_width() as f64/width as f64, self.allocated_height() as f64/height as f64), (1.0,1.0));
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
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
            let (width, height) = get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger=width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(true, bigger, &self.local_css_context(), (self.allocated_width() as f64/width as f64, self.allocated_height() as f64/height as f64), (1.0,1.0));
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
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
            let (width, height) = get_final_widget_size(
                widget,
                self.mode(),
                self.local_css_context().get_minimal_height(),
            );
            self.imp()
                .local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
            let bigger=width > self.allocated_width() || height > self.allocated_height();
            self.imp().transition_stretch(true, bigger, &self.local_css_context(), (self.allocated_width() as f64/width as f64, self.allocated_height() as f64/height as f64), (1.0,1.0));
            match widget.window() {
                //raise window associated to widget if it has one, this enables events on the active mode widget
                Some(window) => window.raise(),
                None => {
                    // debug!("no window");
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
    pub fn set_minimal_height(&self, height: i32, module: bool) -> Result<()> {
        self.imp()
            .local_css_context
            .borrow_mut()
            .set_minimal_height(height, module)
    }

    crate::implement_set_transition!(pub, transition_size,[]);
    crate::implement_set_transition!(pub, transition_bigger_blur,["blur"]);
    crate::implement_set_transition!(pub, transition_bigger_stretch,["stretch-x","stretch-y"]);
    crate::implement_set_transition!(pub, transition_bigger_opacity,["opacity"]);
    crate::implement_set_transition!(pub, transition_smaller_blur,[]);
    crate::implement_set_transition!(pub, transition_smaller_stretch,[]);
    crate::implement_set_transition!(pub, transition_smaller_opacity,[]);
}

#[macro_export]
macro_rules! implement_set_transition{
    ($vis:vis, $val:tt, $props:expr) => {
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&self, transition: Box<dyn EaseFunction>, module: bool) -> Result<()> {
                self.imp()
                    .local_css_context
                    .borrow_mut()
                    .name(dyn_clone::clone_box(transition.as_ref()), module)?;
                let dur=Duration::from_millis(self.imp().local_css_context.borrow().get_transition_duration());
                for prop in $props{
                    self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("minimal-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    self.imp().transition_manager.borrow_mut().set_duration(&(String::from("minimal-")+prop), dur);
                    
                    self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("compact-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    self.imp().transition_manager.borrow_mut().set_duration(&(String::from("compact-")+prop), dur);

                    self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("expanded-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    self.imp().transition_manager.borrow_mut().set_duration(&(String::from("expanded-")+prop), dur);

                    self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("overlay-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    self.imp().transition_manager.borrow_mut().set_duration(&(String::from("overlay-")+prop), dur);
                }
                Ok(())
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
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_width_for_height(height), (height, height))
            }
            _ => (min_height, min_height),
        }
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_height_for_width(width), (0, width))
            }
            _ => (min_height, min_height),
        }
    }

    fn preferred_height(&self) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_height(),
            _ => (min_height, min_height),
        }
    }

    fn preferred_width(&self) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_width(),
            _ => (min_height, min_height),
        }
    }

    fn size_allocate(&self, allocation: &gdk::Rectangle) {
        // trace!("activity allocate: ({}, {})", allocation.width(), allocation.height());

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

        if let Some(widget) = &*self.get_mode_widget(*self.mode.borrow()).borrow() {
            let (width, height) = get_final_widget_size(
                widget,
                *self.mode.borrow(),
                self.local_css_context.borrow().get_minimal_height(),
            );
            self.local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
        }
        // trace!("css_size: {:?}",self.local_css_context.borrow().get_size());
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        // FIXME probably need to fix margins like in scrolling_label
        let mut logs: Vec<String> = vec![];
        let start = Instant::now();
        let mut time = Instant::now();

        // let binding = self.obj();
        // let klass = binding.class();
        // unsafe {
        //     let widget_class = klass.as_ref() as *const _ as *mut gtk::ffi::GtkWidgetClass;
        //     let mut n: c_uint=0;
        //     let out = gtk::ffi::gtk_widget_class_list_style_properties(widget_class, &mut n);
        //     let sl = std::slice::from_raw_parts_mut(out, n.try_into().unwrap());
        //     for prop in sl {
        //         let pspec=glib::ParamSpec::from_glib_ptr_borrow(&prop.cast_const());
        //     }

        // }
        let res: Result<()> = try {
            cr.save()?;
            cr.move_to(
                self.obj().allocation().x() as f64,
                self.obj().allocation().y() as f64,
            );
            let self_w = self.obj().allocation().width() as f64;
            let self_h = self.obj().allocation().height() as f64;
            let border_radius: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-radius", self.obj().state_flags())
                .get()?;
            let border_radius = border_radius as f64;
            let radius = f64::min(border_radius, f64::min(self_w / 2.0, self_h / 2.0));

            self.local_css_context
                .borrow_mut()
                .set_border_radius(radius as i32)
                .expect("failed to set activity border-radius");

            //draw background
            gtk::render_background(&self.obj().style_context(), cr, 0.0, 0.0, self_w, self_h);

            // //draw background widget
            // if let Some(bg_widget) = &*self.background_widget.borrow() {
            //     self.obj().propagate_draw(bg_widget, cr);
            // }

            //setup clip
            begin_draw_scaled_clip(cr, (self_w, self_h), (self_w, self_h), (1.0, 1.0), radius);

            logs.push(format!("bg + clip setup {:?}", time.elapsed()));
            time = Instant::now();

            //draw active mode widget
            let widget_to_render = self.get_mode_widget(*self.mode.borrow());

            //animate blur, opacity and stretch if during transition
            if self.transition_manager.borrow_mut().has_running() {
                // trace!("{}, start: {:?}, dur: {:?}",progress, self.transition.borrow().start_time.elapsed(), self.transition.borrow().duration);
                let last_widget_to_render = self.get_mode_widget(*self.last_mode.borrow());

                let prev_size = if let Some(widget) = &*last_widget_to_render.borrow() {
                    get_final_widget_size(
                        widget,
                        *self.last_mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    )
                } else {
                    (0, 0)
                };
                let next_size = if let Some(widget) = &*widget_to_render.borrow() {
                    get_final_widget_size(
                        widget,
                        *self.mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    )
                } else {
                    (0, 0)
                };
                const FILTER_BACKEND: FilterBackend = FilterBackend::Gpu; //TODO move to config file, if i implement everything on the cpu

                let mut tmp_surface_1 = gtk::cairo::ImageSurface::create(
                    gdk::cairo::Format::ARgb32,
                    self_w as i32,
                    self_h as i32,
                )
                .with_context(|| "failed to create new imagesurface")?;
                //PREV
                if let Some(widget) = &*last_widget_to_render.borrow() {
                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface_1)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    let sx=self.transition_manager.borrow_mut().get_value(&(self.last_mode.borrow().to_string()+"-stretch-x")).unwrap();
                    let sy=self.transition_manager.borrow_mut().get_value(&(self.last_mode.borrow().to_string()+"-stretch-y")).unwrap();
                    // debug!("PREV: sx: {}, sy: {}", sx, sy);
                    // let (mut sx, mut sy) = (
                    //     self_w / prev_size.0 as f64,
                    //     self_h / prev_size.1 as f64,
                    // );
                    // let scale_prog = self.timing_functions(
                    //     progress,
                    //     if bigger {
                    //         TimingFunction::BiggerStretch
                    //     } else {
                    //         TimingFunction::SmallerStretch
                    //     },
                    // );

                    // sx = (1.0 - scale_prog) + sx * scale_prog; // 0->1 | 1-> +sx >1 | 0.5-> 0.5+sx/2=(1+sx)/2 >1
                    // sy = (1.0 - scale_prog) + sy * scale_prog;
                    
                    //setup clip
                    let radius = f64::min(
                        border_radius,
                        f64::min((prev_size.0 as f64 * sx) / 2.0, (prev_size.1 as f64 * sy) / 2.0),
                    );

                    begin_draw_scaled_clip(
                        &tmp_cr,
                        (self_w, self_h),
                        (prev_size.0 as f64, prev_size.1 as f64),
                        (sx, sy),
                        radius,
                    );

                    //scale and center
                    tmp_cr.scale(sx, sy);

                    tmp_cr.translate(
                        //V
                        -(self_w - prev_size.0 as f64) / 2.0
                            + (self_w - prev_size.0 as f64 * sx) / (2.0 * sx),
                        -(self_h - prev_size.1 as f64) / 2.0
                            + (self_h - prev_size.1 as f64 * sy) / (2.0 * sy),
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

                    // logs.push(format!("prev blur processed {:?}", time.elapsed()));
                    // time = Instant::now();

                    // cr.set_source_surface(&tmp_surface_1, 0.0, 0.0)
                    //     .with_context(|| "failed to set source surface")?;

                    // cr.paint_with_alpha(ActivityWidgetPriv::timing_functions(
                    //     progress,
                    //     TimingFunction::PrevOpacity,
                    // ) as f64)
                    //     .with_context(|| "failed to paint surface to context")?;

                    // logs.push(format!("prev blur written to surface {:?}", time.elapsed()));
                    // time = Instant::now();
                }

                let mut tmp_surface_2 = gtk::cairo::ImageSurface::create(
                    gdk::cairo::Format::ARgb32,
                    self_w as i32,
                    self_h as i32,
                )
                .with_context(|| "failed to create new imagesurface")?;
                //NEXT
                if let Some(widget) = &*widget_to_render.borrow() {

                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface_2)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    let sx=self.transition_manager.borrow_mut().get_value(&(self.mode.borrow().to_string()+"-stretch-x")).unwrap();
                    let sy=self.transition_manager.borrow_mut().get_value(&(self.mode.borrow().to_string()+"-stretch-y")).unwrap();
                    // debug!("NEXT: sx: {}, sy: {}", sx, sy);
                    // let (mut sx, mut sy) = (
                    //     self_w / next_size.0 as f64,
                    //     self_h / next_size.1 as f64,
                    // );

                    // let scale_prog =self.timing_functions(
                    //     1.0 - progress,
                    //     if bigger {
                    //         TimingFunction::SmallerStretch
                    //     } else {
                    //         TimingFunction::BiggerStretch
                    //     },
                    // ) as f64;

                    // sx = (1.0 - scale_prog) + sx * scale_prog; // 0->1 | 1-> +sx >1 | 0.5-> 0.5+sx/2=(1+sx)/2 >1
                    // sy = (1.0 - scale_prog) + sy * scale_prog;
                    
                    //setup clip
                    let radius = f64::min(
                        border_radius,
                        f64::min((next_size.0 as f64 * sx) / 2.0, (next_size.1 as f64 * sx) / 2.0)
                    );

                    begin_draw_scaled_clip(
                        &tmp_cr,
                        (self_w, self_h),
                        (next_size.0 as f64, next_size.1 as f64),
                        (sx, sy),
                        radius,
                    );

                    //scale and center
                    
                    //for some reason, without this the widget is not centered with scales near 1.0 (probably due to rounding errors)
                    const CORRECTIVE_FACTOR:f64=2.0;
                    
                    tmp_cr.scale(sx, sy);

                    tmp_cr.translate(
                        //V
                        -CORRECTIVE_FACTOR-(self_w - next_size.0 as f64) / 2.0
                            + (self_w - next_size.0 as f64 * sx) / (2.0 * sx),
                        -CORRECTIVE_FACTOR-(self_h - next_size.1 as f64) / 2.0
                            + (self_h - next_size.1 as f64 * sy) / (2.0 * sy),
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

                    // logs.push(format!("next blur processed {:?}", time.elapsed()));
                    // time = Instant::now();

                    // cr.set_source_surface(&tmp_surface_2, 0.0, 0.0)
                    //     .with_context(|| "failed to set source surface")?;

                    // cr.paint_with_alpha(ActivityWidgetPriv::timing_functions(
                    //     progress,
                    //     TimingFunction::NextOpacity,
                    // ) as f64)
                    //     .with_context(|| "failed to paint surface to context")?;

                    // logs.push(format!("next blur written to surface {:?}", time.elapsed()));
                    // time = Instant::now();
                }

                // let mut orig_surface = cr.group_target();
                let last_blur=self.transition_manager.borrow_mut().get_value(&(self.last_mode.borrow().to_string()+"-blur")).unwrap();
                let next_blur = self.transition_manager.borrow_mut().get_value(&(self.mode.borrow().to_string()+"-blur")).unwrap();

                let last_opacity = self.transition_manager.borrow_mut().get_value(&(self.last_mode.borrow().to_string()+"-opacity")).unwrap();
                let next_opacity = self.transition_manager.borrow_mut().get_value(&(self.mode.borrow().to_string()+"-opacity")).unwrap();

                // debug!("last_blur: {}, next_blur: {}, last_opacity: {}, next_opacity: {}\n", last_blur, next_blur, last_opacity, next_opacity);
                crate::filters::filter::apply_blur_and_merge_opacity_dual(
                    // &mut orig_surface,
                    &mut tmp_surface_1,
                    &mut tmp_surface_2,
                    last_blur as f32,
                    next_blur as f32,
                    last_opacity as f32,
                    next_opacity as f32,
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
                self.obj().queue_draw();
            } else if let Some(widget) = &*widget_to_render.borrow() {
                self.obj().propagate_draw(widget, cr);
                logs.push(format!("static widget drawn {:?}", time.elapsed()));
            }

            //reset
            cr.reset_clip();
            gtk::render_frame(&self.obj().style_context(), cr, 0.0, 0.0, self_w, self_h);

            // self.transition_manager.borrow_mut().update_active();

            cr.restore()?;
        };

        if let Err(err) = res {
            error!("{err}");
        }

        logs.push(format!("total: {:?}", start.elapsed()));

        if start.elapsed() > Duration::from_millis(16) {
            let mut out = String::from("\n");
            for log in logs {
                out.push_str(&log);
                out.push('\n');
            }
            debug!("{out}"); //TODO maybe create a utility library
        }
        glib::Propagation::Proceed
    }
}

pub fn begin_draw_scaled_clip(
    cr: &gdk::cairo::Context,
    (self_w, self_h): (f64, f64),
    (inner_w, inner_h): (f64, f64),
    (scale_x, scale_y): (f64, f64),
    radius: f64,
) {
    cr.arc(
        (self_w - inner_w * scale_x) / 2.0 + radius,
        (self_h - inner_h * scale_y) / 2.0 + radius,
        radius,
        PI * 1.0,
        PI * 1.5,
    );
    //top left //WHY are the angles rotated by 90 degrees
    cr.line_to(
        self_w - (self_w - inner_w * scale_x) / 2.0 - radius,
        (self_h - inner_h * scale_y) / 2.0,
    );
    cr.arc(
        self_w - (self_w - inner_w * scale_x) / 2.0 - radius,
        (self_h - inner_h * scale_y) / 2.0 + radius,
        radius,
        PI * 1.5,
        PI * 0.0,
    );
    //top right
    cr.line_to(
        self_w - (self_w - inner_w * scale_x) / 2.0,
        self_h - (self_h - inner_h * scale_y) / 2.0 - radius,
    );
    cr.arc(
        self_w - (self_w - inner_w * scale_x) / 2.0 - radius,
        self_h - (self_h - inner_h * scale_y) / 2.0 - radius,
        radius,
        PI * 0.0,
        PI * 0.5,
    );
    //bottom right
    cr.line_to(
        (self_w - inner_w * scale_x) / 2.0 + radius,
        self_h - (self_h - inner_h * scale_y) / 2.0,
    );
    cr.arc(
        (self_w - inner_w * scale_x) / 2.0 + radius,
        self_h - (self_h - inner_h * scale_y) / 2.0 - radius,
        radius,
        PI * 0.5,
        PI * 1.0,
    );
    //bottom left
    cr.line_to(
        (self_w - inner_w * scale_x) / 2.0,
        (self_h - inner_h * scale_y) / 2.0 + radius,
    );
    cr.clip();
}

fn get_max_preferred_size(m1: (i32, i32), m2: (i32, i32)) -> (i32, i32) {
    (std::cmp::max(m1.0, m2.0), std::cmp::max(m1.1, m2.1))
}

fn get_final_widget_size(
    widget: &gtk::Widget,
    mode: ActivityMode,
    minimal_height: i32,
) -> (i32, i32) {
    let height = match mode {
        ActivityMode::Minimal | ActivityMode::Compact => minimal_height,
        ActivityMode::Expanded | ActivityMode::Overlay => {
            if widget.height_request() != -1 {
                widget.height_request()
            } else {
                widget.allocation().height()
            }
        }
    };
    let width = if widget.width_request() != -1 {
        widget.width_request()
    } else {
        widget.allocation().width()
    };
    (width, height)
}
