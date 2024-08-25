use std::{fmt::Display, str::FromStr};

use gdk::prelude::{DisplayExt, ListModelExtManual, MonitorExt};
use gtk::{graphene::Point, gsk::Transform, prelude::WidgetExt};

use super::activity_widget::boxed_activity_mode::ActivityMode;

#[derive(Clone, Copy, Debug)]
pub enum CssSize {
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

/// Get the size of a widget for a mode
///
/// For `Minimal` mode there is a forced height and width (`minimal_height` and `minimal_width`)
///
/// For `Compact` mode there is only a forced height (`minimal_height`)
///
/// If a size isn't forced the final size is the requested size.
///
/// If the requested size isn't set (or is -1), the natural size is used,
/// this is calculated from `widget.measure(gtk::Orientation, -1)`
pub(super) fn get_final_widget_size(
    widget: &gtk::Widget,
    mode: ActivityMode,
    minimal_height: i32,
    minimal_width: i32,
) -> (i32, i32) {
    let force_height = matches!(mode, ActivityMode::Minimal | ActivityMode::Compact);
    let force_width = matches!(mode, ActivityMode::Minimal);
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
    let width = if force_width {
        minimal_width
    } else if widget.width_request() > 0 {
        widget.width_request()
    } else {
        measured_width.1
    };
    (width.max(minimal_width), height.max(minimal_height))
}

/// Get the allocation for a mode widget, aligned
///
/// if an align is Fill, if the widget is smaller than the parent, the parent size is used,
/// the widget is always centered
///
/// otherwise the child size is used
pub(super) fn get_child_aligned_allocation(
    parent_allocation: (i32, i32, i32),
    child: &gtk::Widget,
    mode: ActivityMode,
    minimal_height: i32,
    use_max_width: bool,
) -> (i32, i32, Option<Transform>) {
    let parent_width = parent_allocation.0;
    let parent_height = parent_allocation.1;
    let _parent_baseline = parent_allocation.2;

    let force_height = matches!(mode, ActivityMode::Minimal | ActivityMode::Compact);
    let (child_width_min, child_width_nat, _, _) = child.measure(
        gtk::Orientation::Horizontal,
        if force_height { minimal_height } else { -1 },
    );
    let (child_height_min, child_height_nat, _, _) = child.measure(gtk::Orientation::Vertical, -1);

    let child_width = if use_max_width {
        child_width_nat
    } else {
        parent_width.clamp(child_width_min, child_width_nat)
    };
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
    let opt_transform = if !(x == 0.0 && y == 0.0) {
        Some(Transform::new().translate(&Point::new(x, y)))
    } else {
        None
    };
    (width, height, opt_transform)
}

/// Get a slice where every value is `other_values` except the one at the index of `mode` that is `mode_value`
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
//TODO listen for added monitors, cache result
pub fn get_max_monitors_size() -> (i32, i32) {
    gdk::Display::default()
        .unwrap()
        .monitors()
        .iter::<gdk::Monitor>()
        .flatten()
        .map(|mon| {
            (
                mon.geometry().x() + mon.geometry().width(),
                mon.geometry().y() + mon.geometry().height(),
            )
        })
        .reduce(|acc, e| (acc.0.max(e.0), acc.1.max(e.1)))
        .unwrap_or((1000, 1000))
}
