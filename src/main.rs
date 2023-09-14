#![feature(async_closure)]
use std::time::Duration;

use anyhow::{Context, Ok, Result};
use dynisland::widgets::activity_widget::{ActivityWidget, ActivityMode};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

fn main() -> Result<()> {
    let css_content = grass::from_path(
        "/home/david/dev/rust/dynisland/file.scss",
        &grass::Options::default(),
    );

    gtk::init().with_context(|| "failed to init gtk")?;

    //setup css styles
    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_data(css_content.unwrap().as_bytes())
        .unwrap();
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().unwrap(),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = gtk::Window::builder()
        .title("test")
        .type_(gtk::WindowType::Toplevel)
        .height_request(500)
        .width_request(800)
        .build();

    let main_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .vexpand(false)
        .hexpand(false)
        .build();
    main_box.style_context().add_class("box1");
    main_box.set_homogeneous(false);
    // box1.set_clip(&gtk::Rectangle::new(0, 0, 500, 400));

    let fixed_h_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(400)
        .vexpand(false)
        .hexpand(false)
        .build();
    fixed_h_box.style_context().add_class("box2");
    fixed_h_box.set_homogeneous(false);

    let overlay = ActivityWidget::new();
    overlay.set_vexpand(false);
    overlay.set_hexpand(false);
    overlay.set_valign(gtk::Align::Start);
    overlay.set_halign(gtk::Align::Start);
    //overlay.set_width_request(400);
    overlay.set_transition_duration(2000);
    overlay.style_context().add_class("overlay1");

    let background = gtk::Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .build();
    background.style_context().add_class("text1");

    let minimal = gtk::Box::builder()
    .height_request(100)
        .width_request(200)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    minimal.style_context().add_class("box3");

    minimal.add(&gtk::Label::builder()
    .label("label")
    .halign(gtk::Align::Center)
    .valign(gtk::Align::Center)
    .hexpand(true)
    .build());


    let compact = gtk::Box::builder()
    .height_request(400)
        .width_request(800)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    compact.style_context().add_class("box4");

    compact.add(&gtk::Label::builder()
    .label("sadhfjasd")
    .halign(gtk::Align::Center)
    .valign(gtk::Align::Center)
    .hexpand(true)
    .build());

    overlay.add(&background);
    overlay.add_minimal_mode(&minimal.upcast());
    overlay.add_compact_mode(&compact.upcast());

    fixed_h_box.add(&overlay);

    main_box.add(&fixed_h_box);

    window.add(&main_box);

    window.connect_destroy(|_| std::process::exit(0));
    window.show_all();

    let (ui_send, mut ui_recv) = tokio::sync::mpsc::unbounded_channel::<(i32,i32)>();
    gtk::Window::set_interactive_debugging(true);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("idk tokio rt failed");
        rt.block_on(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            ui_send.send((800,400)).expect("recv closed");
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            ui_send.send((200,100)).expect("recv closed");
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            ui_send.send((800,400)).expect("recv closed");
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            ui_send.send((200,100)).expect("recv closed");
        });
    });
    let local_css_provider = gtk::CssProvider::new();
    local_css_provider
    .load_from_data(format!(".text1{{ min-width: 200px; min-height:100px; }}").as_bytes())
                .unwrap();
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().unwrap(),
        &local_css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_SETTINGS,
    );

    glib::MainContext::default().spawn_local(async move {
        loop {

            let (w,h) = match ui_recv.recv().await{
                Some(n) => n,
                None => return,
            };

            local_css_provider
            .load_from_data(format!(".text1{{ min-width: {w}px; min-height: {h}px; }}").as_bytes())
                .unwrap();

            match overlay.mode() {
                ActivityMode::Minimal => overlay.set_mode(ActivityMode::Compact),
                ActivityMode::Compact => overlay.set_mode(ActivityMode::Minimal),
                _ => {},
            }

        }
    });

    gtk::main();
    Ok(())
}
