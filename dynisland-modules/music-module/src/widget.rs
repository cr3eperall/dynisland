use dynisland_core::{
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use gtk::{prelude::*, GestureClick, Widget};

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
    let minimal = get_minimal();
    let compact = get_compact();
    let expanded = get_expanded();
    // let overlay = Self::get_overlay();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(&minimal);
    activity_widget.set_compact_mode_widget(&compact);
    activity_widget.set_expanded_mode_widget(&expanded);
    // activity_widget.set_overlay_mode_widget(&overlay);

    activity
        .add_dynamic_property("mode", ActivityMode::Minimal)
        .unwrap();

    let mode = activity.get_property("mode").unwrap();

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

    activity
}

fn set_act_widget(_activity_widget: &mut ActivityWidget) {
    // activity_widget.set_vexpand(false);
    // activity_widget.set_hexpand(false);
    // activity_widget.set_valign(gtk::Align::Start);
    // activity_widget.set_halign(gtk::Align::Center);
}

fn get_minimal() -> gtk::Widget {
    let minimal = gtk::Box::builder()
        // .height_request(40)
        .width_request(140)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(true)
        .build();
    minimal.set_margin_start(20);
    minimal.set_margin_end(20);

    // let minimal = gtk::EventBox::builder()
    //     .height_request(40)
    //     // .width_request(100)
    //     .valign(gtk::Align::Center)
    //     .halign(gtk::Align::Center)
    //     .vexpand(false)
    //     .hexpand(false)
    //     .above_child(false) //Allows events on children (like buttons)
    //     .child(&minimal)
    //     .build();
    // minimal.parent_window().unwrap().set_keep_above(true);
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

    // let compact = gtk::EventBox::builder()
    //     .height_request(40)
    //     .width_request(280)
    //     .valign(gtk::Align::Center)
    //     .halign(gtk::Align::Center)
    //     .vexpand(true)
    //     .hexpand(false)
    //     .child(&compact)
    //     .build();
    compact.upcast()
}

fn get_expanded() -> gtk::Widget {
    let height = 300;
    let width = 450;
    let v_container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .height_request(height)
        .width_request(width)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    let info_container = info_container(width as f32, height as f32 * 0.45);
    let progress_container = progress_container(width as f32, height as f32 * 0.15);
    let controls_container = controls_container(width as f32, height as f32 * 0.40);

    v_container.append(&info_container);
    v_container.append(&progress_container);
    v_container.append(&controls_container);

    v_container.upcast()
}

// fn get_overlay() -> gtk::Widget {
//     let overlay = gtk::Box::builder()
//         .height_request(1080)
//         .width_request(1920)
//         .valign(gtk::Align::Center)
//         .halign(gtk::Align::Center)
//         .vexpand(false)

//         .hexpand(false)
//         .build();
//     overlay.add(
//         &gtk::Label::builder()
//             .label("Overlay label,\n Hello Hello \n Hello Hello")
//             .halign(gtk::Align::Center)
//             .valign(gtk::Align::Center)
//             .hexpand(true)
//             .build(),
//     );
//     overlay.upcast()
// }

fn info_container(width: f32, height: f32) -> Widget {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(height as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    container.add_css_class("info");

    let album_art_size = height.min(width * 0.3);
    let album_art = gtk::Box::builder()
        // .file("/home/david/Pictures/Music_not_playing.svg")
        .width_request(album_art_size as i32)
        .build();
    let image = gtk::Image::builder()
        .file("/home/david/Pictures/Music_not_playing.svg")
        .hexpand(true)
        .halign(gtk::Align::Center)
        .width_request((album_art_size * 0.7) as i32)
        .height_request((album_art_size * 0.7) as i32)
        .build();

    // log::debug!("{}", (album_art_size * 0.7) as i32);
    album_art.append(&image);
    album_art.add_css_class("album-art");

    let music_info_container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .height_request(height as i32)
        .width_request((width * 0.50) as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Start)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(true)
        .build();
    music_info_container.add_css_class("info-names1");
    let music_info_container2 = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Start)
        .vexpand(false)
        .hexpand(false)
        .build();
    music_info_container2.add_css_class("info-names2");
    let song_name = gtk::Label::builder() //TODO replace with scrollable label
        .label("Song name")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .wrap(false)
        .hexpand(true)
        .build();
    song_name.add_css_class("song-name");
    let artist_name = gtk::Label::builder() //TODO replace with scrollable label
        .label("Artist name")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .wrap(false)
        .hexpand(true)
        .build();
    artist_name.add_css_class("artist-name");
    music_info_container2.append(&song_name);
    music_info_container2.append(&artist_name);
    music_info_container.append(&music_info_container2);

    let visualizer_size = height.min(width * 0.2); //TODO replace with actual visualizer
    let visualizer = gtk::Box::builder()
        // .height_request((visualizer_size*0.8) as i32)
        .width_request(visualizer_size as i32)
        .build();
    let image = gtk::Image::builder()
        .file("/home/david/Pictures/visualizer_tmp.jpeg")
        .width_request((visualizer_size * 0.8) as i32)
        .height_request((visualizer_size * 0.8) as i32)
        .hexpand(true)
        .halign(gtk::Align::Center)
        .build();
    // log::debug!("vis: {}", (visualizer_size * 0.8) as i32);
    visualizer.append(&image);
    visualizer.add_css_class("visualizer");

    container.append(&album_art);
    container.append(&music_info_container);
    container.append(&visualizer);

    container.into()
}
fn progress_container(width: f32, height: f32) -> Widget {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request((height) as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    container.add_css_class("progress");

    let elapsed = gtk::Label::builder()
        .label("0:00")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request((width * 0.15) as i32)
        .build();
    elapsed.add_css_class("elapsed-time");
    let progress_bar = gtk::Scale::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request((width * 0.7) as i32)
        .build();
    progress_bar.set_range(0.0, 1.0);
    progress_bar.set_draw_value(false);
    progress_bar.set_increments((1.0 / (width * 0.7)).into(), 0.1);
    progress_bar.add_css_class("progress-bar");
    let remaining = gtk::Label::builder()
        .label("-3:42")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request((width * 0.15) as i32)
        .build();
    remaining.add_css_class("remaining-time");

    container.append(&elapsed);
    container.append(&progress_bar);
    container.append(&remaining);

    container.into()
}
fn controls_container(width: f32, height: f32) -> Widget {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request((height) as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(true)
        .build();

    container.add_css_class("controls");

    let shuffle = gtk::Button::builder()
        .label("Sh")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width * 0.15)) as i32)
        .width_request((width * 0.15) as i32)
        .build();
    shuffle.add_css_class("shuffle");
    let previous = gtk::Button::builder()
        .label("Pr")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width * 0.2)) as i32)
        .width_request((width * 0.2) as i32)
        .build();
    previous.add_css_class("previous");
    let play_pause = gtk::Button::builder()
        .label("Pl")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width * 0.2)) as i32)
        .width_request((width * 0.2) as i32)
        .build();
    play_pause.add_css_class("play-pause");
    let next = gtk::Button::builder()
        .label("Nx")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width * 0.2)) as i32)
        .width_request((width * 0.2) as i32)
        .build();
    next.add_css_class("next");
    let repeat = gtk::Button::builder()
        .label("Rp")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width * 0.15)) as i32)
        .width_request((width * 0.15) as i32)
        .build();

    container.append(&shuffle);
    container.append(&previous);
    container.append(&play_pause);
    container.append(&next);
    container.append(&repeat);

    container.into()
}
