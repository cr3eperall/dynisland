use gtk::{graphene::Point, gsk::Transform, prelude::*, subclass::prelude::*};

use crate::graphics::activity_widget::{imp::ActivityMode, ActivityWidget};

#[derive(Default)]
pub struct ActivityLayoutManagerPriv {}

#[glib::object_subclass]
impl ObjectSubclass for ActivityLayoutManagerPriv {
    const NAME: &'static str = "ActivityLayoutManager";
    type Type = super::ActivityLayoutManager;
    type ParentType = gtk::LayoutManager;
}

impl ObjectImpl for ActivityLayoutManagerPriv {}

impl ActivityLayoutManagerPriv {
    pub(super) fn get_child_aligned_allocation(
        //TODO move to util
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
            gtk::Align::Fill => (if child_width>parent_width {(parent_width - child_width) as f32 / 2.0} else {0.0}, parent_width.max(child_width)),
            _ => {
                // center
                ((parent_width - child_width) as f32 / 2.0, child_width)
            }
        };
        let (y, height) = match child.valign() {
            gtk::Align::Baseline | gtk::Align::Start => (0.0, child_height),
            gtk::Align::End => ((parent_height - child_height) as f32, child_height),
            gtk::Align::Fill => (if child_height>parent_height {(parent_height - child_height) as f32 / 2.0} else {0.0}, parent_height.max(child_height)),
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
}

impl LayoutManagerImpl for ActivityLayoutManagerPriv {
    fn measure(
        &self,
        widget: &gtk::Widget,
        orientation: gtk::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let activity_widget = widget.clone().downcast::<ActivityWidget>();
        if let Err(err) = activity_widget {
            log::error!("Error: {:?}", err); //TODO maybe change to glib assert

            return (0, 0, -1, -1);
        }
        let activity_widget = activity_widget.unwrap();

        let min_height = activity_widget.local_css_context().get_minimal_height();
        let first_child = activity_widget.first_child(); //should be the background widget
        match first_child {
            Some(first_child) => {
                let (min_size, nat_size, _, _) = first_child.measure(orientation, for_size);
                (min_height.max(min_size), min_height.max(nat_size), -1, -1)
            }
            None => (min_height, min_height, -1, -1),
        }
    }

    fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, baseline: i32) {
        let activity_widget = widget.clone().downcast::<ActivityWidget>();
        if let Err(err) = activity_widget {
            log::error!("Error: {:?}", err); //TODO maybe change to glib assert
            return;
        }
        let binding = activity_widget.unwrap();
        let min_height = binding.local_css_context().get_minimal_height();
        let activity = binding.imp();

        if let Some(content) = &*activity.background_widget.borrow() {
            content.allocate(width, height, -1, None);
        };

        if let Some(content) = &*activity.minimal_mode_widget.borrow() {
            let (width, height, transform) = Self::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Minimal,
                min_height,
            );

            content.allocate(width, height, -1, transform);

            log::debug!(
                "minimal allocated: ({:?}, {:?})",
                content.allocated_width(),
                content.allocated_height()
            );
        }
        if let Some(content) = &*activity.compact_mode_widget.borrow() {
            let (width, height, transform) = Self::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Compact,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        }
        if let Some(content) = &*activity.expanded_mode_widget.borrow() {
            let (width, height, transform) = Self::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Expanded,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        }
        if let Some(content) = &*activity.overlay_mode_widget.borrow() {
            let (width, height, transform) = Self::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Overlay,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        };
    }
}
