#![feature(async_closure)]

use anyhow::{Context, Ok, Result};
use dynisland::widgets::activity_widget::{ActivityMode, ActivityWidget};
use gtk::prelude::*;

fn main() -> Result<()> {
    //parse static scss file
    let css_content = grass::from_path(
        "/home/david/dev/rust/dynisland/file.scss",
        &grass::Options::default(),
    );
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;

    //setup static css style
    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_data(css_content.unwrap().as_bytes())
        .unwrap();
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().unwrap(),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    //create window
    let window = gtk::Window::builder()
        .title("test")
        .type_(gtk::WindowType::Toplevel)
        .height_request(500)
        .width_request(800)
        .build();

    //create overlay
    let overlay = ActivityWidget::new();
    overlay.set_vexpand(false);
    overlay.set_hexpand(false);
    overlay.set_valign(gtk::Align::Start);
    overlay.set_halign(gtk::Align::Center);
    overlay.set_transition_duration(2000);
    overlay.style_context().add_class("overlay");

    //get widgets
    let background = get_bg();
    let minimal = get_minimal();
    let compact = get_compact();

    //load widgets in the overlay
    overlay.add(&*background);
    overlay.set_minimal_mode(&minimal.upcast());
    overlay.set_compact_mode(&compact.upcast());

    //add overlay to window
    window.add(&overlay);

    //show window
    window.connect_destroy(|_| std::process::exit(0));
    window.show_all();

    //event loop
    let (ui_send, mut ui_recv) = tokio::sync::mpsc::unbounded_channel::<ActivityMode>();
    gtk::Window::set_interactive_debugging(true);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("idk tokio rt failed");
        rt.block_on(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
            ui_send.send(ActivityMode::Compact).expect("recv closed");
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            ui_send.send(ActivityMode::Minimal).expect("recv closed");
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            ui_send.send(ActivityMode::Compact).expect("recv closed");
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            ui_send.send(ActivityMode::Minimal).expect("recv closed");
        });
    });

    //ui event executor loop
    glib::MainContext::default().spawn_local(async move {
        loop {
            let mode = match ui_recv.recv().await {
                Some(n) => n,
                None => return,
            };
            overlay.set_mode(mode)
        }
    });
    //start application
    gtk::main();
    Ok(())
}

fn get_bg() -> Box<impl IsA<gtk::Widget>> {
    let background = gtk::Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .build();
    Box::new(background)
}

fn get_compact() -> Box<impl IsA<gtk::Widget>> {
    let compact = gtk::Box::builder()
        .height_request(400)
        .width_request(800)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    compact.style_context().add_class("box4");

    compact.add(
        &gtk::Label::builder()
            .label("sadhfjasd")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    Box::new(compact)
}

fn get_minimal() -> Box<impl IsA<gtk::Widget>> {
    let minimal = gtk::Box::builder()
        .height_request(100)
        .width_request(200)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    minimal.style_context().add_class("box3");

    minimal.add(
        &gtk::Label::builder()
            .label("label")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    Box::new(minimal)
}
