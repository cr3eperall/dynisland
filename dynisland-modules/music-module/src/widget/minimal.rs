use glib::Cast;
use gtk::prelude::*;

use crate::module::MusicConfig;

pub fn get_minimal(config: &MusicConfig) -> gtk::Widget {
    let height: f32 = 40.0;
    let width: f32 = 80.0;
    let minimal = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(height as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(true)
        .build();
    {
        let album_art_width = width * 0.45;
        let album_art_size = height.min(width);
        let album_art = gtk::Box::builder()
            .width_request(album_art_width as i32)
            .homogeneous(false)
            .hexpand(false)
            .build();

        album_art.add_css_class("album-art");
        {
            let image = gtk::Image::builder()
                .file(config.default_album_art_path.clone())
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

        minimal.append(&album_art);
    }
    minimal.upcast()
}
