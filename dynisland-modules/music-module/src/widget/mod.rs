pub mod compact;
pub mod expanded;
pub mod minimal;
pub mod visualizer;

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

use crate::module::MusicConfig;

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
    pub can_playpause: bool,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub can_loop: bool,
    pub can_shuffle: bool,
    pub shuffle: bool,
    pub loop_status: mpris::LoopStatus,
}
impl Default for UIPlaybackStatus {
    fn default() -> Self {
        UIPlaybackStatus {
            playback_status: mpris::PlaybackStatus::Stopped,
            can_playpause: false,
            can_go_next: false,
            can_go_previous: false,
            can_loop: false,
            can_shuffle: false,
            shuffle: false,
            loop_status: mpris::LoopStatus::None,
        }
    }
}

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
    config: &MusicConfig,
    action_tx: UnboundedSender<UIAction>,
) -> DynamicActivity {
    let mut activity = DynamicActivity::new(prop_send, module, name);

    //create activity widget
    let activity_widget = activity.get_activity_widget();
    // set_act_widget(&mut activity_widget);
    //get widgets
    let minimal = minimal::get_minimal(config);
    let compact = compact::get_compact(config);
    let expanded = expanded::get_expanded(config, action_tx);
    // let overlay = Self::get_overlay();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(&minimal);
    activity_widget.set_compact_mode_widget(&compact);
    activity_widget.set_expanded_mode_widget(&expanded);
    // activity_widget.set_overlay_mode_widget(&overlay);

    setup_music_metadata_prop(&mut activity, &activity_widget);

    setup_album_art_prop(&mut activity, &activity_widget);

    setup_visualizer_data_prop(&mut activity, &activity_widget);

    setup_visualizer_gradient_prop(&mut activity);

    setup_music_time_prop(&mut activity, &activity_widget);

    setup_playback_status_prop(&mut activity, &activity_widget);

    setup_scrolling_label_speed_prop(&mut activity, &activity_widget);

    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(gdk::BUTTON_PRIMARY);

    let aw = activity_widget.clone();
    press_gesture.connect_released(move |_gest, _, _, _| {
        match aw.mode() {
            ActivityMode::Minimal => {
                // m1.lock().await.set(ActivityMode::Compact).unwrap();
            }
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Expanded);
            }
            ActivityMode::Expanded => {
                // m1.lock().await.set(ActivityMode::Overlay).unwrap();
            }
            ActivityMode::Overlay => {
                // m1.lock().await.set(ActivityMode::Minimal).unwrap();
            }
        }
    });

    activity_widget.add_controller(press_gesture);

    let release_gesture = GestureClick::new();
    release_gesture.set_button(gdk::BUTTON_SECONDARY);
    let aw = activity_widget.clone();
    release_gesture.connect_released(move |_gest, _, _, _| {
        match aw.mode() {
            ActivityMode::Minimal => {
                // m1.lock().await.set(ActivityMode::Compact).unwrap();
            }
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Minimal);
            }
            ActivityMode::Expanded => {
                aw.set_mode(ActivityMode::Compact);
            }
            ActivityMode::Overlay => {
                // m1.lock().await.set(ActivityMode::Minimal).unwrap();
            }
        }
    });
    activity_widget.add_controller(release_gesture);

    // TODO add property and config for scrolling label speed

    activity
}

fn setup_playback_status_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
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
                match playback_status.can_shuffle {
                    true => {
                        match playback_status.shuffle {
                            true => {
                                shuffle.set_icon_name("media-playlist-shuffle-symbolic");
                            }
                            false => {
                                shuffle.set_icon_name("media-playlist-consecutive-symbolic");
                            }
                        }
                        shuffle.set_sensitive(true);
                    }
                    false => {
                        shuffle.set_icon_name("media-playlist-shuffle-symbolic");
                        shuffle.set_sensitive(false);
                    }
                }

                match playback_status.can_go_previous {
                    true => {
                        // previous.set_icon_name("media-seek-backward-symbolic");
                        previous.set_sensitive(true);
                    }
                    false => {
                        // previous.set_icon_name("list-remove-symbolic");
                        previous.set_sensitive(false);
                    }
                }
                match playback_status.can_playpause {
                    true => {
                        match playback_status.playback_status {
                            mpris::PlaybackStatus::Playing => {
                                //TODO find another icon because i don't like it
                                play_pause.set_icon_name("media-playback-pause-symbolic");
                            }
                            mpris::PlaybackStatus::Paused => {
                                play_pause.set_icon_name("media-playback-start-symbolic");
                            }
                            mpris::PlaybackStatus::Stopped => {
                                play_pause.set_icon_name("media-playback-stop-symbolic");
                            }
                        }
                        play_pause.set_sensitive(true);
                    }
                    false => {
                        play_pause.set_icon_name("media-playback-stop-symbolic");
                        play_pause.set_sensitive(false);
                    }
                }

                match playback_status.can_go_next {
                    true => {
                        // next.set_icon_name("media-seek-forward-symbolic");
                        next.set_sensitive(true);
                    }
                    false => {
                        next.set_sensitive(false);
                        // next.set_icon_name("list-remove-symbolic");
                    }
                }
                match playback_status.can_loop {
                    true => {
                        match playback_status.loop_status {
                            mpris::LoopStatus::None => {
                                repeat.set_icon_name("mail-forward");
                            }
                            mpris::LoopStatus::Track => {
                                repeat.set_icon_name("media-playlist-repeat-song-symbolic");
                            }
                            mpris::LoopStatus::Playlist => {
                                repeat.set_icon_name("media-playlist-repeat-symbolic");
                            }
                        }
                        repeat.set_sensitive(true);
                    }
                    false => {
                        repeat.set_icon_name("media-playlist-repeat-symbolic");
                        repeat.set_sensitive(false);
                    }
                }
            })
            .unwrap();
    }
}

fn setup_music_time_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
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
        let aw = activity_widget.clone();
        activity
            .subscribe_to_property("music-time", move |new_value| {
                let (mut current_time, mut total_duration) =
                    cast_dyn_any!(new_value, (Duration, Duration)).unwrap();
                if let ActivityMode::Expanded = aw.mode() {
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
                }
            })
            .unwrap();
    }
}

fn setup_visualizer_gradient_prop(activity: &mut DynamicActivity) {
    let gradient_css_provider = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &gradient_css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    activity
        .add_dynamic_property("visualizer-gradient", [[[0_u8; 3]; 6]; 3])
        .unwrap();
    {
        activity
            .subscribe_to_property("visualizer-gradient", move |new_value| {
                let data = cast_dyn_any!(new_value, [[[u8; 3]; 6]; 3]).unwrap();
                gradient_css_provider.load_from_string(&visualizer::get_gradient_css(data))
            })
            .unwrap();
    }
}

fn setup_visualizer_data_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
    let bar_height_css_provider = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &bar_height_css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    activity
        .add_dynamic_property("visualizer-data", [0_u8; 6])
        .unwrap();
    {
        let aw = activity_widget.clone();
        activity
            .subscribe_to_property("visualizer-data", move |new_value| {
                let data = cast_dyn_any!(new_value, [u8; 6]).unwrap();
                bar_height_css_provider.load_from_string(&visualizer::get_bar_css(
                    data,
                    32,
                    30,
                    60,
                    aw.mode(),
                ));
            })
            .unwrap();
    }
}

fn setup_album_art_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
    let empty: Vec<u8> = Vec::new();
    activity.add_dynamic_property("album-art", empty).unwrap();
    {
        let album_art = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Image>()
            .unwrap();
        let compact_album_art = activity_widget
            .compact_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Image>()
            .unwrap();
        let minimal_album_art = activity_widget
            .minimal_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Image>()
            .unwrap();

        activity
            .subscribe_to_property("album-art", move |new_value| {
                let buf = cast_dyn_any!(new_value, Vec<u8>).unwrap();
                let data = buf.as_slice();
                let data = Bytes::from(data);
                let mut pixbuf = Pixbuf::from_stream(
                    &MemoryInputStream::from_bytes(&data),
                    None::<&gtk::gio::Cancellable>,
                )
                .ok();
                if pixbuf.is_none() {
                    pixbuf = Pixbuf::new(gdk::gdk_pixbuf::Colorspace::Rgb, true, 8, 10, 10);
                }
                // let mut pixbuf=pixbuf.unwrap().scale_simple(6, 3, gdk::gdk_pixbuf::InterpType::Bilinear);
                let texture = gdk::Texture::for_pixbuf(&pixbuf.unwrap());
                album_art.set_paintable(Some(&texture));
                compact_album_art.set_paintable(Some(&texture));
                minimal_album_art.set_paintable(Some(&texture));
            })
            .unwrap();
    }
}

fn setup_music_metadata_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
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
        let compact_song_name_widget = activity_widget
            .compact_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .downcast::<ScrollingLabel>()
            .unwrap();
        activity
            .subscribe_to_property("music-metadata", move |new_value| {
                let (song_name, artist_name) = cast_dyn_any!(new_value, (String, String)).unwrap();
                song_name_widget.label().set_label(song_name);
                artist_name_widget.set_label(artist_name);
                compact_song_name_widget.label().set_label(song_name);
            })
            .unwrap();
    }
}

fn setup_scrolling_label_speed_prop(
    activity: &mut DynamicActivity,
    activity_widget: &ActivityWidget,
) {
    activity
        .add_dynamic_property("scrolling-label-speed", 30.0_f32)
        .unwrap();
    {
        let expanded_label = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<ScrollingLabel>()
            .unwrap();
        let compact_label = activity_widget
            .compact_mode_widget()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .downcast::<ScrollingLabel>()
            .unwrap();
        activity
            .subscribe_to_property("scrolling-label-speed", move |new_value| {
                let data = cast_dyn_any!(new_value, f32).unwrap();
                // log::info!("setting speed: {}", data);
                expanded_label.set_scroll_speed(*data, true);
                compact_label.set_scroll_speed(*data, true);
            })
            .unwrap();
    }
}
