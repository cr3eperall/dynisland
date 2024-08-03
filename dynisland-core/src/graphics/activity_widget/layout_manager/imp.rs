use gtk::{prelude::*, subclass::prelude::*};

use crate::{
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, util, ActivityWidget},
    randomize_name,
};

#[derive(Default)]
pub struct ActivityLayoutManagerPriv {}

#[glib::object_subclass]
impl ObjectSubclass for ActivityLayoutManagerPriv {
    const NAME: &'static str = randomize_name!("ActivityLayoutManager");
    type Type = super::ActivityLayoutManager;
    type ParentType = gtk::LayoutManager;
}

impl ObjectImpl for ActivityLayoutManagerPriv {}

// impl ActivityLayoutManagerPriv {

// }

impl LayoutManagerImpl for ActivityLayoutManagerPriv {
    fn measure(
        &self,
        widget: &gtk::Widget,
        orientation: gtk::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let activity_widget = widget.downcast_ref::<ActivityWidget>();
        if activity_widget.is_none() {
            log::error!("Error downcasting ActivityWidget");
            return (0, 0, -1, -1);
        }
        let activity_widget = activity_widget.unwrap();

        let min_height = activity_widget
            .local_css_context()
            .get_config_minimal_height();
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
        let activity_widget = widget.downcast_ref::<ActivityWidget>();
        if activity_widget.is_none() {
            log::error!("Error downcasting ActivityWidget");
            return;
        }
        let binding = activity_widget.unwrap();
        let min_height = binding.local_css_context().get_config_minimal_height();
        let activity = binding.imp();

        if let Some(content) = &*activity.background_widget.borrow() {
            content.allocate(width, height, -1, None);
        };

        if let Some(content) = &*activity.minimal_mode_widget.borrow() {
            let (width, height, transform) = util::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Minimal,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        }
        if let Some(content) = &*activity.compact_mode_widget.borrow() {
            let (width, height, transform) = util::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Compact,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        }
        if let Some(content) = &*activity.expanded_mode_widget.borrow() {
            let (width, height, transform) = util::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Expanded,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        }
        if let Some(content) = &*activity.overlay_mode_widget.borrow() {
            let (width, height, transform) = util::get_child_aligned_allocation(
                (width, height, baseline),
                content,
                ActivityMode::Overlay,
                min_height,
            );

            content.allocate(width, height, -1, transform);
        };
    }
}
