use dynisland_core::{
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::{
        activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
        widgets::{rolling_char::RollingChar, scrolling_label::ScrollingLabel},
    },
};

use gtk::{prelude::*, GestureClick, Label};

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
) -> DynamicActivity {
    let mut activity = DynamicActivity::new(prop_send, module, name);

    //create activity widget
    let mut activity_widget = activity.get_activity_widget();
    set_act_widget(&mut activity_widget);
    //get widgets
    // let background = Self::get_bg();
    let minimal = get_minimal();
    let compact = get_compact();
    let expanded = get_expanded();
    let overlay = get_overlay();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(&minimal);
    activity_widget.set_compact_mode_widget(&compact);
    activity_widget.set_expanded_mode_widget(&expanded);
    activity_widget.set_overlay_mode_widget(&overlay);

    // activity_widget.connect_mode_notify(|f| {
    // let l = f.mode();
    // debug!("Changed mode: {:?}", l);
    // });

    // activity.set_activity_widget(activity_widget.clone());

    activity
        .add_dynamic_property("mode", ActivityMode::Minimal)
        .unwrap();
    activity
        .add_dynamic_property("comp-label", "compact".to_string())
        .unwrap();
    activity
        .add_dynamic_property("scrolling-label-text", "Hello, World".to_string())
        .unwrap();
    activity.add_dynamic_property("rolling-char", '0').unwrap();

    let minimal_cl = minimal.clone();
    activity
        .subscribe_to_property("scrolling-label-text", move |new_value| {
            let real_value = cast_dyn_any!(new_value, String).unwrap();
            log::debug!("text changed:{real_value}");
            minimal_cl
                .downcast_ref::<gtk::Box>()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<ScrollingLabel>()
                .unwrap()
                .label()
                .set_text(real_value.as_str());
        })
        .unwrap();

    let mode = activity.get_property_any("mode").unwrap();

    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(gdk::BUTTON_PRIMARY);

    let m1 = mode.clone();
    press_gesture.connect_released(move |_gest, _, _, _| {
        // debug!("primary");
        // gest.set_state(gtk::EventSequenceState::Claimed);
        let m1 = m1.clone();
        glib::MainContext::default().spawn_local(async move {
            let mode_g = m1.lock().await;
            let mode = *cast_dyn_any!(mode_g.get(), ActivityMode).unwrap();
            drop(mode_g);

            match mode {
                ActivityMode::Minimal => {
                    m1.lock().await.set(ActivityMode::Compact).unwrap();
                }
                ActivityMode::Compact => {
                    m1.lock().await.set(ActivityMode::Expanded).unwrap();
                }
                ActivityMode::Expanded => {
                    m1.lock().await.set(ActivityMode::Overlay).unwrap();
                }
                ActivityMode::Overlay => {
                    m1.lock().await.set(ActivityMode::Minimal).unwrap();
                }
            }
        });
    });

    activity_widget.add_controller(press_gesture);

    let m1 = mode.clone();
    let release_gesture = GestureClick::new();
    release_gesture.set_button(gdk::BUTTON_SECONDARY);
    release_gesture.connect_released(move |_gest, _, _, _| {
        // debug!("secondary");
        // gest.set_state(gtk::EventSequenceState::Claimed);
        let m1 = m1.clone();
        glib::MainContext::default().spawn_local(async move {
            let mode_g = m1.lock().await;
            let mode = *cast_dyn_any!(mode_g.get(), ActivityMode).unwrap();
            drop(mode_g);

            match mode {
                ActivityMode::Minimal => {
                    log::warn!("Don't. It will crash and idk why");
                    m1.lock().await.set(ActivityMode::Overlay).unwrap();
                }
                ActivityMode::Compact => {
                    m1.lock().await.set(ActivityMode::Minimal).unwrap();
                }
                ActivityMode::Expanded => {
                    m1.lock().await.set(ActivityMode::Compact).unwrap();
                }
                ActivityMode::Overlay => {
                    m1.lock().await.set(ActivityMode::Expanded).unwrap();
                }
            }
        });
    });

    activity_widget.add_controller(release_gesture);

    //set mode when updated
    activity
        .subscribe_to_property("mode", move |new_value| {
            let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
            activity_widget.set_mode(real_value);
        })
        .unwrap();

    let c1 = compact.clone();
    activity
        .subscribe_to_property("rolling-char", move |new_value| {
            let real_value = cast_dyn_any!(new_value, char).unwrap();
            let first_child = c1 //TODO i need to change this, maybe with a macro
                .downcast_ref::<gtk::Box>()
                .unwrap()
                .first_child()
                .unwrap();

            let rolling_char_1 = first_child
                .next_sibling()
                .unwrap()
                .downcast::<RollingChar>()
                .unwrap();
            rolling_char_1.set_current_char(real_value);

            let rolling_char_2 = rolling_char_1
                .next_sibling()
                .unwrap()
                .downcast::<RollingChar>()
                .unwrap();
            rolling_char_2.set_current_char(real_value);
        })
        .unwrap();

    //set label when updated
    activity
        .subscribe_to_property("comp-label", move |new_value| {
            let real_value = cast_dyn_any!(new_value, String).unwrap();
            compact //TODO i need to change this, maybe with a macro
                .downcast_ref::<gtk::Box>()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<gtk::Label>()
                .unwrap()
                .set_label(real_value);
        })
        .unwrap();

    activity
}

fn set_act_widget(_activity_widget: &mut ActivityWidget) {
    // activity_widget.set_vexpand(true);
    // activity_widget.set_hexpand(true);
    // activity_widget.set_valign(gtk::Align::Start);
    // activity_widget.set_halign(gtk::Align::Start);
    // activity_widget.set_transition_duration(2000, true).unwrap();
    // activity_widget.style_context().add_class("overlay");
}

fn get_minimal() -> gtk::Widget {
    let minimal = gtk::Box::builder()
        // .height_request(40)
        .width_request(240)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .overflow(gtk::Overflow::Hidden)
        .homogeneous(false)
        .build();

    let scroll_label = ScrollingLabel::new(Some("VDsdvzdfvzcxvvzxcvzcd"));
    scroll_label.set_hexpand(false);
    scroll_label.set_vexpand(false);
    scroll_label.set_valign(gtk::Align::Center);
    scroll_label.set_halign(gtk::Align::Start);
    // scroll_label.set_width_request(400);
    scroll_label.set_height_request(40);
    scroll_label.set_margin_start(20);
    scroll_label.set_margin_end(20);

    // let test_btn=gtk::Button::new();
    // test_btn.set_label("test");
    // test_btn.connect_clicked(|_btn|{
    //     log::info!("test");
    // });
    // let btn_gest=GestureClick::new();
    // btn_gest.set_button(gdk::BUTTON_PRIMARY);
    // btn_gest.connect_released(|gest,_,_,_|{
    //     gest.set_state(gtk::EventSequenceState::Claimed);
    //     log::info!("test");
    // });
    // test_btn.add_controller(btn_gest);

    // scroll_label.inner_label().set_margin_start(10);
    // scroll_label.inner_label().set_margin_end(30);

    minimal.append(&scroll_label);
    // minimal.append(&test_btn);
    minimal.upcast()
}

fn get_compact() -> gtk::Widget {
    let compact = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(40)
        .width_request(280)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    compact.append(
        &Label::builder()
            .label("Compact")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build(),
    );

    let rn1 = RollingChar::new(None);
    rn1.set_valign(gtk::Align::Center);
    rn1.set_halign(gtk::Align::Center);
    compact.append(&rn1);

    let rn2 = RollingChar::new(None);
    rn2.set_valign(gtk::Align::Center);
    rn2.set_halign(gtk::Align::Center);
    compact.append(&rn2);

    compact.upcast()
}

fn get_expanded() -> gtk::Widget {
    let expanded = gtk::Box::builder()
        .height_request(400)
        .width_request(500)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    expanded.append(
        &gtk::Label::builder()
            .label("Expanded label,\n Hello Hello")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    expanded.upcast()
}

fn get_overlay() -> gtk::Widget {
    let expanded = gtk::Box::builder()
        .height_request(1080)
        .width_request(1920)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    expanded.append(
        &gtk::Label::builder()
            .label("Overlay label,\n Hello Hello \n Hello Hello")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    expanded.upcast()
}
