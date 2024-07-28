use std::time::Duration;

use dynisland_core::graphics::widgets::scrolling_label::ScrollingLabel;
use glib::Cast;
use gtk::{prelude::*, GestureClick, Widget};
use tokio::sync::mpsc::UnboundedSender;

use super::{visualizer::get_visualizer, UIAction};

pub fn get_expanded(action_tx: UnboundedSender<UIAction>) -> gtk::Widget {
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
                .file(crate::module::DEFAULT_ALBUM_ART_PATH)
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
                    song_name.set_scroll_speed(20.0, true);
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
        let visualizer = get_visualizer(visualizer_size, visualizer_size);
        // visualizer.add_css_class("visualizer");
        // {
        //     let image = gtk::Image::builder()
        //         .file("/home/david/Pictures/visualizer_tmp.jpeg")
        //         .width_request((visualizer_size * 0.8) as i32)
        //         .height_request((visualizer_size * 0.8) as i32)
        //         .hexpand(true)
        //         .halign(gtk::Align::Center)
        //         .build();
        //     visualizer.append(&image);
        // }

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
            .margin_start((width * 0.05) as i32)
            .build();
        elapsed.add_css_class("elapsed-time");

        //TODO add time when dragging scale
        let progress_bar = gtk::Scale::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request((width * 0.6) as i32)
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
                    .send(UIAction::SetPosition(Duration::from_millis(
                        prog_1.value() as u64
                    )))
                    .expect("failed to send seek message");
            });
            progress_bar.add_controller(release_gesture);
        }

        let remaining = gtk::Label::builder()
            .label("--:--")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request((width * 0.15) as i32)
            .margin_end((width * 0.05) as i32)
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
            // .label("Sh")
            .icon_name("media-playlist-shuffle-symbolic")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.15)) as i32)
            .width_request((width * 0.15) as i32)
            .sensitive(false)
            .build();
        shuffle.add_css_class("shuffle");
        {
            let a_tx = action_tx.clone();
            shuffle.connect_clicked(move |_| {
                a_tx.send(UIAction::Shuffle).unwrap();
            });
        }

        let previous = gtk::Button::builder()
            // .label("Pr")
            .icon_name("media-seek-backward")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.2)) as i32)
            .width_request((width * 0.2) as i32)
            .sensitive(false)
            .build();
        previous.add_css_class("previous");
        {
            let a_tx = action_tx.clone();
            previous.connect_clicked(move |_| {
                a_tx.send(UIAction::Previous).unwrap();
            });
        }

        let play_pause = gtk::Button::builder()
            // .label("Pl")
            .icon_name("media-playback-start-symbolic")
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
            // .label("Nx")
            .icon_name("media-seek-forward")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.2)) as i32)
            .width_request((width * 0.2) as i32)
            .sensitive(false)
            .build();
        next.add_css_class("next");
        {
            let a_tx = action_tx.clone();
            next.connect_clicked(move |_| {
                a_tx.send(UIAction::Next).unwrap();
            });
        }

        let repeat = gtk::Button::builder()
            // .label("Rp")
            .icon_name("media-playlist-repeat-symbolic")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .height_request((height.min(width * 0.15)) as i32)
            .width_request((width * 0.15) as i32)
            .sensitive(false)
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
