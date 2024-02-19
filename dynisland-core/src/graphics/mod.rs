pub mod activity_widget;
pub mod config_variable;
pub mod dynamic_activity;
pub mod dynamic_property;
pub mod widgets;

pub mod util {
    use std::{fmt::Display, str::FromStr};

    use gtk::{graphene::Point, gsk::Transform, prelude::WidgetExt};

    use super::activity_widget::imp::ActivityMode;

    #[derive(Clone, Copy, Debug)]
    pub enum CssSize {
        //TODO replace with proper data structure when i implement the custom css parsing
        Fixed(i32),
        Percent(f32),
    }

    impl Display for CssSize {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CssSize::Fixed(val) => write!(f, "{}px", val),
                CssSize::Percent(val) => write!(f, "{}%", val),
            }
        }
    }
    impl FromStr for CssSize {
        type Err = String;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if let Some(stripped) = s.strip_suffix('%') {
                let val = stripped.parse().map_err(|e| format!("{:?}", e))?;
                Ok(CssSize::Percent(val))
            } else if let Some(stripped) = s.strip_suffix("px") {
                let val = stripped.parse().map_err(|e| format!("{:?}", e))?;
                Ok(CssSize::Fixed(val))
            } else {
                let val = s.parse().map_err(|e| format!("{:?}", e))?;
                Ok(CssSize::Fixed(val))
            }
        }
    }

    impl CssSize {
        pub fn get(&self) -> f32 {
            match self {
                CssSize::Fixed(val) => *val as f32,
                CssSize::Percent(val) => *val,
            }
        }
        pub fn get_for_size(&self, size: f32) -> f32 {
            match self {
                CssSize::Fixed(val) => *val as f32,
                CssSize::Percent(val) => size * val / 100.0,
            }
        }
    }

    pub(super) fn get_final_widget_size(
        //checked
        widget: &gtk::Widget,
        mode: ActivityMode,
        minimal_height: i32,
    ) -> (i32, i32) {
        let force_height = matches!(mode, ActivityMode::Minimal | ActivityMode::Compact);
        let measured_width = widget.measure(
            gtk::Orientation::Horizontal,
            if force_height { minimal_height } else { -1 },
        );
        let measured_height = widget.measure(gtk::Orientation::Vertical, -1);
        let height = if force_height {
            minimal_height
        } else if widget.height_request() > 0 {
            widget.height_request()
        } else {
            measured_height.1
        };
        let width = if widget.width_request() > 0 {
            widget.width_request()
        } else {
            measured_width.1
        };
        (width.max(minimal_height), height.max(minimal_height))
    }

    pub(super) fn get_child_aligned_allocation(
        parent_allocation: (i32, i32, i32),
        child: &gtk::Widget,
        mode: ActivityMode,
        minimal_height: i32,
    ) -> (i32, i32, Option<Transform>) {
        let parent_width = parent_allocation.0;
        let parent_height = parent_allocation.1;
        let _parent_baseline = parent_allocation.2;

        let force_height = matches!(mode, ActivityMode::Minimal | ActivityMode::Compact);
        let (child_width_min, child_width_nat, _, _) = child.measure(
            gtk::Orientation::Horizontal,
            if force_height { minimal_height } else { -1 },
        );
        let (child_height_min, child_height_nat, _, _) =
            child.measure(gtk::Orientation::Vertical, -1);

        let child_width = parent_width.clamp(child_width_min, child_width_nat);
        let child_height = parent_height.clamp(child_height_min, child_height_nat);

        let (x, width) = match child.halign() {
            gtk::Align::Baseline | gtk::Align::Start => (0.0, child_width),
            gtk::Align::End => ((parent_width - child_width) as f32, child_width),
            gtk::Align::Fill => (
                if child_width > parent_width {
                    (parent_width - child_width) as f32 / 2.0
                } else {
                    0.0
                },
                parent_width.max(child_width),
            ),
            _ => {
                // center
                ((parent_width - child_width) as f32 / 2.0, child_width)
            }
        };
        let (y, height) = match child.valign() {
            gtk::Align::Baseline | gtk::Align::Start => (0.0, child_height),
            gtk::Align::End => ((parent_height - child_height) as f32, child_height),
            gtk::Align::Fill => (
                if child_height > parent_height {
                    (parent_height - child_height) as f32 / 2.0
                } else {
                    0.0
                },
                parent_height.max(child_height),
            ),
            _ => {
                // center
                ((parent_height - child_height) as f32 / 2.0, child_height)
            }
        };
        let opt_transform = if x != 0.0 || y != 0.0 {
            Some(Transform::new().translate(&Point::new(x, y)))
        } else {
            None
        };
        (width, height, opt_transform)
    }

    pub(super) fn get_property_slice_for_mode_f64(
        mode: ActivityMode,
        mode_value: f64,
        other_values: f64,
    ) -> [f64; 4] {
        match mode {
            ActivityMode::Minimal => [mode_value, other_values, other_values, other_values],
            ActivityMode::Compact => [other_values, mode_value, other_values, other_values],
            ActivityMode::Expanded => [other_values, other_values, mode_value, other_values],
            ActivityMode::Overlay => [other_values, other_values, other_values, mode_value],
        }
    }
}
