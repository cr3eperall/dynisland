pub mod allocate_and_draw;
pub mod local_css_context;
pub mod widget;

const BLUR_RADIUS: f32 = 6.0;

const FILTER_BACKEND: crate::filters::filter::FilterBackend =
    crate::filters::filter::FilterBackend::Gpu; //TODO move to config file, if i implement everything on the cpu

//without this the widget is not centered with scales near 1.0 (probably due to rounding errors)
const TRANSLATE_CORRECTIVE_FACTOR: f64 = -1.0;
const CLIP_CORRECTIVE_FACTOR: f64 = 1.0;

pub mod util {
    use css_anim::transition::TransitionManager;
    use gtk::prelude::WidgetExt;

    use super::{widget::ActivityMode, BLUR_RADIUS};

    pub(super) fn init_transition_properties(tm: &mut TransitionManager) {
        tm.add_property("minimal-opacity", 1.0);
        tm.add_property("minimal-blur", 0.0);
        tm.add_property("minimal-stretch-x", 1.0);
        tm.add_property("minimal-stretch-y", 1.0);

        tm.add_property("compact-opacity", 0.0);
        tm.add_property("compact-blur", BLUR_RADIUS as f64);
        tm.add_property("compact-stretch-x", 1.0);
        tm.add_property("compact-stretch-y", 1.0);

        tm.add_property("expanded-opacity", 0.0);
        tm.add_property("expanded-blur", BLUR_RADIUS as f64);
        tm.add_property("expanded-stretch-x", 1.0);
        tm.add_property("expanded-stretch-y", 1.0);

        tm.add_property("overlay-opacity", 0.0);
        tm.add_property("overlay-blur", BLUR_RADIUS as f64);
        tm.add_property("overlay-stretch-x", 1.0);
        tm.add_property("overlay-stretch-y", 1.0);
    }

    pub(super) fn get_max_preferred_size(m1: (i32, i32), m2: (i32, i32)) -> (i32, i32) {
        (std::cmp::max(m1.0, m2.0), std::cmp::max(m1.1, m2.1))
    }

    pub(super) fn get_final_widget_size(
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
}

#[macro_export]
macro_rules! implement_set_transition{
    ($vis:vis, $ctx:tt, $val:tt, $props:expr) => {
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&self, transition: Box<dyn EaseFunction>, module: bool) -> Result<()> {
                self.imp()
                    .$ctx
                    .borrow_mut()
                    .name(dyn_clone::clone_box(transition.as_ref()), module)?;
                let dur=Duration::from_millis(self.imp().$ctx.borrow().get_transition_duration());
                for prop in $props{
                    self.imp().transition_manager.borrow_mut().set_easing_function(prop, dyn_clone::clone_box(transition.as_ref()));
                    self.imp().transition_manager.borrow_mut().set_duration(prop, dur);

                    // self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("minimal-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    // self.imp().transition_manager.borrow_mut().set_duration(&(String::from("minimal-")+prop), dur);

                    // self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("compact-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    // self.imp().transition_manager.borrow_mut().set_duration(&(String::from("compact-")+prop), dur);

                    // self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("expanded-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    // self.imp().transition_manager.borrow_mut().set_duration(&(String::from("expanded-")+prop), dur);

                    // self.imp().transition_manager.borrow_mut().set_easing_function(&(String::from("overlay-")+prop), dyn_clone::clone_box(transition.as_ref()));
                    // self.imp().transition_manager.borrow_mut().set_duration(&(String::from("overlay-")+prop), dur);
                }
                Ok(())
            }
        });
    };
    ($vis:vis, $ctx:tt, $val:tt) => {
        concat_idents::concat_idents!(name = set_, $val {
            $vis fn name(&self, transition: Box<dyn EaseFunction>, module: bool) -> Result<()> {
                self.imp()
                    .$ctx
                    .borrow_mut()
                    .name(dyn_clone::clone_box(transition.as_ref()), module)
            }
        });
    };
}
