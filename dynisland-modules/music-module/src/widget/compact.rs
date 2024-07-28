use dynisland_core::graphics::widgets::scrolling_label::ScrollingLabel;
use glib::Cast;
use gtk::prelude::*;

use super::visualizer::get_visualizer;

pub fn get_compact() -> gtk::Widget {
    let height: f32=40.0;
    let width: f32=280.0;
    let compact = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(height as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(false)
        .build();
    {
        let album_art_width=width * 0.2;
        let album_art_size = height.min(width);
        let album_art = gtk::Box::builder()
            .width_request(album_art_width as i32)
            .homogeneous(false)
            .hexpand(false)
            .build();

        album_art.add_css_class("album-art");
        {
            let image = gtk::Image::builder()
                .file(crate::module::DEFAULT_ALBUM_ART_PATH)
                .hexpand(true)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .width_request((album_art_size * 0.75) as i32)
                .height_request((album_art_size * 0.75) as i32)
                .overflow(gtk::Overflow::Hidden)
                .build();

            // log::debug!("{}", (album_art_size * 0.7) as i32);
            album_art.append(&image);
        }
        
        let song_name = ScrollingLabel::new(None);
        {
            song_name.label().set_text("Song name");
            song_name.set_width_request((width * 0.6) as i32);
            song_name.set_halign(gtk::Align::Start);
            song_name.set_valign(gtk::Align::Center);
            song_name.set_hexpand(false);
            song_name.add_css_class("song-name");
            song_name.set_scroll_speed(20.0, true);
        }

        let visualizer_width = width * 0.2;
        let visualizer_container=gtk::Box::builder()
        .width_request(visualizer_width as i32)
        .homogeneous(false)
        .halign(gtk::Align::Center)
        .hexpand(false)
        .build();
        visualizer_container.add_css_class("visualizer-container");
        {
            let visualizer_size = height.min(visualizer_width); //TODO replace with actual visualizer
            let visualizer = get_visualizer(visualizer_size, visualizer_size);
            visualizer_container.append(&visualizer);
        }

        compact.append(&album_art);
        compact.append(&song_name);
        compact.append(&visualizer_container);
    }
    compact.upcast()
}