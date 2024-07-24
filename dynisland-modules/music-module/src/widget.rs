use std::time::Duration;

use dynisland_core::{
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::{
        activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
        widgets::scrolling_label::ScrollingLabel,
    },
};
use gdk::{gdk_pixbuf::Pixbuf, gio::MemoryInputStream};
use glib::Bytes;
use gtk::{prelude::*, GestureClick, Image, Widget};
use tokio::sync::mpsc::UnboundedSender;

pub enum UIAction {
    Shuffle,
    Previous,
    PlayPause,
    Next,
    Loop,
    SetPosition(Duration),
}
#[derive(Debug, Clone)]
pub struct UIPlaybackStatus {
    pub playback_status: mpris::PlaybackStatus,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub shuffle: bool,
    pub loop_status: mpris::LoopStatus,
}
impl Default for UIPlaybackStatus {
    fn default() -> Self {
        UIPlaybackStatus {
            playback_status: mpris::PlaybackStatus::Stopped,
            can_go_next: false,
            can_go_previous: false,
            shuffle: false,
            loop_status: mpris::LoopStatus::None,
        }
    }
}

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
    action_tx: UnboundedSender<UIAction>,
) -> DynamicActivity {
    let mut activity = DynamicActivity::new(prop_send, module, name);

    //create activity widget
    let mut activity_widget = activity.get_activity_widget();
    set_act_widget(&mut activity_widget);
    //get widgets
    let minimal = get_minimal();
    let compact = get_compact();
    let expanded = get_expanded(action_tx);
    // let overlay = Self::get_overlay();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(&minimal);
    activity_widget.set_compact_mode_widget(&compact);
    activity_widget.set_expanded_mode_widget(&expanded);
    // activity_widget.set_overlay_mode_widget(&overlay);

    activity
        .add_dynamic_property("mode", ActivityMode::Minimal)
        .unwrap();
    {
        let aw = activity_widget.clone();
        activity
            .subscribe_to_property("mode", move |new_value| {
                let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
                aw.set_mode(real_value);
            })
            .unwrap();
    }
    activity
        .add_dynamic_property("music-metadata", (String::new(), String::new()))
        .unwrap();
    {
        let music_info_container2: Widget = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .first_child()
            .unwrap();
        let song_name_widget = music_info_container2
            .first_child()
            .unwrap()
            .downcast::<ScrollingLabel>()
            .unwrap();
        let artist_name_widget = song_name_widget
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap();
        activity
            .subscribe_to_property("music-metadata", move |new_value| {
                let (song_name, artist_name) = cast_dyn_any!(new_value, (String, String)).unwrap();
                song_name_widget.label().set_label(song_name);
                artist_name_widget.set_label(artist_name);
            })
            .unwrap();
    }

    let empty:Vec<u8>=Vec::new();
    activity
        .add_dynamic_property("album-art", empty)
        .unwrap();
    {
        let album_art = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap().downcast::<Image>().unwrap();

        activity
            .subscribe_to_property("album-art", move |new_value| {
                let buf = cast_dyn_any!(new_value, Vec<u8>).unwrap();
                let data=buf.as_slice();
                let data =Bytes::from(data);
                let mut pixbuf = Pixbuf::from_stream(&MemoryInputStream::from_bytes(&data), None::<&gtk::gio::Cancellable>).ok();
                if pixbuf.is_none(){
                    pixbuf=Pixbuf::new(gdk::gdk_pixbuf::Colorspace::Rgb, true, 8, 10, 10);
                }
                let texture=gdk::Texture::for_pixbuf(&pixbuf.unwrap());
                album_art.set_paintable(Some(&texture));
            })
            .unwrap();
    }

    activity
        .add_dynamic_property("music-time", (Duration::ZERO, Duration::ZERO))
        .unwrap();
    {
        let progress_info = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap();
        let elapsed = progress_info
            .first_child()
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap();
        let progress_bar = elapsed
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Scale>()
            .unwrap();
        let remaining = progress_bar
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap();
        activity
            .subscribe_to_property("music-time", move |new_value| {
                let (mut current_time, mut total_duration) =
                    cast_dyn_any!(new_value, (Duration, Duration)).unwrap();
                progress_bar.set_range(0.0, total_duration.as_millis() as f64);

                if !progress_bar.has_css_class("dragging") {
                    progress_bar.set_value(current_time.as_millis() as f64);
                    // log::warn!("{}",progress_bar.value());
                }
                current_time = Duration::from_secs(current_time.as_secs());
                total_duration = Duration::from_secs(total_duration.as_secs());
                elapsed.set_label(&format!(
                    "{:02}:{:02}",
                    current_time.as_secs() / 60,
                    current_time.as_secs() % 60
                ));
                let remaining_time = total_duration.saturating_sub(current_time);
                remaining.set_label(&format!(
                    "-{:02}:{:02}",
                    remaining_time.as_secs() / 60,
                    remaining_time.as_secs() % 60
                ));
            })
            .unwrap();
    }

    activity
        .add_dynamic_property("playback-status", UIPlaybackStatus::default())
        .unwrap();
    {
        let control_container = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .next_sibling()
            .unwrap();
        let shuffle = control_container
            .first_child()
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap();
        let previous = shuffle
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap();
        let play_pause = previous
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap();
        let next = play_pause
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap();
        let repeat = next
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap();

        activity
            .subscribe_to_property("playback-status", move |new_value| {
                let playback_status = cast_dyn_any!(new_value, UIPlaybackStatus).unwrap();

                match playback_status.shuffle {
                    true => {
                        shuffle.set_label("ON");
                    }
                    false => {
                        shuffle.set_label("OFF");
                    }
                }
                match playback_status.can_go_previous {
                    true => {
                        previous.set_label("<-");
                        previous.set_sensitive(true);
                    }
                    false => {
                        previous.set_label("--");
                        previous.set_sensitive(false);
                    }
                }
                match playback_status.playback_status {
                    mpris::PlaybackStatus::Playing => {
                        play_pause.set_label("Ps");
                    }
                    mpris::PlaybackStatus::Paused => {
                        play_pause.set_label("Pl");
                    }
                    mpris::PlaybackStatus::Stopped => {
                        play_pause.set_label("St");
                    }
                }
                match playback_status.can_go_next {
                    true => {
                        next.set_label("->");
                        next.set_sensitive(true);
                    }
                    false => {
                        next.set_sensitive(false);
                        next.set_label("--");
                    }
                }
                match playback_status.loop_status {
                    mpris::LoopStatus::None => {
                        repeat.set_label("OFF");
                    }
                    mpris::LoopStatus::Track => {
                        repeat.set_label("TRK");
                    }
                    mpris::LoopStatus::Playlist => {
                        repeat.set_label("PL");
                    }
                }
            })
            .unwrap();
    }

    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(gdk::BUTTON_PRIMARY);

    let mode = activity.get_property_any("mode").unwrap();

    let m1 = mode.clone();
    //FIXME doesn't follow guidelines: shouldn't register minimal->compact mode change
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
                    log::warn!("Don't. It will crash if there is an overlay widget and idk why");
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
    let lab1 = gtk::Label::builder() //TODO replace with scrollable label
        .label("")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .wrap(false)
        .hexpand(true)
        .build();
    minimal.append(&lab1);

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
    let lab1 = gtk::Label::builder() //TODO replace with scrollable label
        .label("")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .wrap(false)
        .hexpand(true)
        .build();
    compact.append(&lab1);
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

fn get_expanded(action_tx: UnboundedSender<UIAction>) -> gtk::Widget {
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
    {
        let info_container = info_container(width as f32, height as f32 * 0.45);
        let progress_container =
            progress_container(width as f32, height as f32 * 0.15, action_tx.clone());
        let controls_container = controls_container(width as f32, height as f32 * 0.40, action_tx);

        v_container.append(&info_container);
        v_container.append(&progress_container);
        v_container.append(&controls_container);
    }
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
    {
        let album_art_size = height.min(width * 0.3);
        let album_art = gtk::Box::builder()
            .width_request(album_art_size as i32)
            .build();

        album_art.add_css_class("album-art");
        {
            let image = gtk::Image::builder()
                .file("/home/david/Pictures/Music_not_playing.svg")
                .hexpand(true)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .width_request((album_art_size * 0.7) as i32)
                .height_request((album_art_size * 0.7) as i32)
                .overflow(gtk::Overflow::Hidden)
                .build();

            // log::debug!("{}", (album_art_size * 0.7) as i32);
            album_art.append(&image);
        }
        let music_info_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .height_request(height as i32)
            .width_request((width * 0.5) as i32)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Start)
            .vexpand(false)
            .hexpand(false)
            .homogeneous(true)
            .build();
        music_info_container.add_css_class("info-names1");
        {
            let music_info_container2 = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .valign(gtk::Align::Center)
                .halign(gtk::Align::Start)
                .width_request((width * 0.5) as i32)
                .vexpand(false)
                .hexpand(false)
                .build();
            music_info_container2.add_css_class("info-names2");
            {
                let song_name = ScrollingLabel::new(None);
                {
                    song_name.label().set_text("Song name");
                    // song_name.set_width_request((width * 0.45) as i32);
                    song_name.set_halign(gtk::Align::Start);
                    song_name.set_valign(gtk::Align::Center);
                    song_name.set_hexpand(false);
                    song_name.add_css_class("song-name");
                }

                let artist_name = gtk::Label::builder() //TODO maybe replace with scrollable label, for now ellipses are enough
                    .label("Artist name")
                    .halign(gtk::Align::Start)
                    .valign(gtk::Align::Center)
                    .wrap(false)
                    .max_width_chars(20)
                    .ellipsize(gdk::pango::EllipsizeMode::End)
                    .hexpand(true)
                    .build();
                artist_name.add_css_class("artist-name");

                music_info_container2.append(&song_name);
                music_info_container2.append(&artist_name);
            }
            music_info_container.append(&music_info_container2);
        }

        let visualizer_size = height.min(width * 0.2); //TODO replace with actual visualizer
        let visualizer = gtk::Box::builder()
            .width_request(visualizer_size as i32)
            .build();
        visualizer.add_css_class("visualizer");
        {
            let image = gtk::Image::builder()
                .file("/home/david/Pictures/visualizer_tmp.jpeg")
                .width_request((visualizer_size * 0.8) as i32)
                .height_request((visualizer_size * 0.8) as i32)
                .hexpand(true)
                .halign(gtk::Align::Center)
                .build();
            visualizer.append(&image);
        }

        container.append(&album_art);
        container.append(&music_info_container);
        container.append(&visualizer);
    }
    container.into()
}
fn progress_container(width: f32, height: f32, action_tx: UnboundedSender<UIAction>) -> Widget {
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
    {
        let elapsed = gtk::Label::builder()
            .label("--:--")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request((width * 0.15) as i32)
            .build();
        elapsed.add_css_class("elapsed-time");

        let progress_bar = gtk::Scale::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request((width * 0.7) as i32)
            .draw_value(false)
            .build();
        progress_bar.add_css_class("progress-bar");
        {
            progress_bar.set_range(0.0, 1.0);
            progress_bar.set_increments((1.0 / (width * 0.7)).into(), 0.1);

            let prog_1 = progress_bar.clone();
            let release_gesture = GestureClick::new();
            release_gesture.set_button(gdk::BUTTON_PRIMARY);
            release_gesture.connect_unpaired_release(move |_gest, _, _, _, _| {
                action_tx
                    .send(UIAction::SetPosition(Duration::from_millis(prog_1.value() as u64)))
                    .expect("failed to send seek message");
            });
            progress_bar.add_controller(release_gesture);
        }

        let remaining = gtk::Label::builder()
            .label("--:--")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request((width * 0.15) as i32)
            .build();
        remaining.add_css_class("remaining-time");

        container.append(&elapsed);
        container.append(&progress_bar);
        container.append(&remaining);
    }
    container.into()
}

fn controls_container(width: f32, height: f32, action_tx: UnboundedSender<UIAction>) -> Widget {
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
    {
        let shuffle = gtk::Button::builder()
            .label("Sh")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.15)) as i32)
            .width_request((width * 0.15) as i32)
            .build();
        shuffle.add_css_class("shuffle");
        {
            let a_tx = action_tx.clone();
            shuffle.connect_clicked(move |_| {
                a_tx.send(UIAction::Shuffle).unwrap();
            });
        }

        let previous = gtk::Button::builder()
            .label("Pr")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.2)) as i32)
            .width_request((width * 0.2) as i32)
            .build();
        previous.add_css_class("previous");
        {
            let a_tx = action_tx.clone();
            previous.connect_clicked(move |_| {
                a_tx.send(UIAction::Previous).unwrap();
            });
        }

        let play_pause = gtk::Button::builder()
            .label("Pl")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.2)) as i32)
            .width_request((width * 0.2) as i32)
            .build();
        play_pause.add_css_class("play-pause");
        {
            let a_tx = action_tx.clone();
            play_pause.connect_clicked(move |_| {
                a_tx.send(UIAction::PlayPause).unwrap();
            });
        }

        let next = gtk::Button::builder()
            .label("Nx")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.2)) as i32)
            .width_request((width * 0.2) as i32)
            .build();
        next.add_css_class("next");
        {
            let a_tx = action_tx.clone();
            next.connect_clicked(move |_| {
                a_tx.send(UIAction::Next).unwrap();
            });
        }

        let repeat = gtk::Button::builder()
            .label("Rp")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.15)) as i32)
            .width_request((width * 0.15) as i32)
            .build();
        repeat.add_css_class("loop");
        {
            let a_tx = action_tx.clone();
            repeat.connect_clicked(move |_| {
                a_tx.send(UIAction::Loop).unwrap();
            });
        }

        container.append(&shuffle);
        container.append(&previous);
        container.append(&play_pause);
        container.append(&next);
        container.append(&repeat);
    }
    container.into()
}
