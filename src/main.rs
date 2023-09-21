#![feature(async_closure)]
#![feature(trait_upcasting)]

use anyhow::{Context, Ok, Result};
use dynisland::{widgets::{
    dynamic_property::ValidDynType,
    activity_widget::{ActivityMode, ActivityWidget},
    dynamic_activity::DynamicActivity,
}, cast_dyn_prop};
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
    let window = get_window();

    gtk::Window::set_interactive_debugging(true);
    
    {
        //create ui channel
        let (ui_send, mut ui_recv) = tokio::sync::mpsc::unbounded_channel::<(String, Box<dyn ValidDynType>)>();
        //create one activity
        let mut activity = get_activity(ui_send);

        //data producer
        let mode = activity.get_property("mode").unwrap();
        let label=activity.get_property("comp-label").unwrap();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("idk tokio rt failed");
            rt.block_on(async {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    mode.lock().await.set(ActivityMode::Minimal).unwrap();

                    tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
                    mode.lock().await.set(ActivityMode::Compact).unwrap();
                    let old_label_val;
                    {
                        let label_val = label.lock().await;
                        let str_val:&String=(label_val.get() as &dyn std::any::Any).downcast_ref().unwrap();
                        old_label_val=str_val.clone();
                        
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    label.lock().await.set("sdkjvksdv1".to_string()).unwrap();
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    label.lock().await.set("fghn".to_string()).unwrap();
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                    label.lock().await.set(old_label_val).unwrap();


                    tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
                    mode.lock().await.set(ActivityMode::Expanded).unwrap();

                    tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
                    mode.lock().await.set(ActivityMode::Compact).unwrap();
                }
            });
        });
        
        //add activity widget to window
        window.add(&activity.widget);
        
        //ui-thread executor of property subscribers fot this activity
        glib::MainContext::default().spawn_local(async move {
            while let Some(res) = ui_recv.recv().await {
                match activity.get_subscribers(&res.0) {
                    core::result::Result::Ok(subs) => {
                        for sub in subs {
                            sub(&*res.1);
                        }
                    }
                    Err(err) => eprintln!("{}", err),
                }
            }
        });
    }

    //show window
    window.connect_destroy(|_| std::process::exit(0));
    window.show_all();
    //start application
    gtk::main();
    Ok(())
}

fn get_activity(ui_send:tokio::sync::mpsc::UnboundedSender<(String, Box<dyn ValidDynType>)>)-> DynamicActivity{
    let mut activity = DynamicActivity::new(ui_send.clone());
    
    //create activity widget
    let activity_widget = get_act_widget();
    //get widgets
    let background = get_bg();
    let minimal = get_minimal();
    let compact = get_compact();
    let expanded = get_expanded();
    
    //load widgets in the activity widget
    activity_widget.add(&background);
    activity_widget.set_minimal_mode(&minimal);
    activity_widget.set_compact_mode(&compact);
    activity_widget.set_expanded_mode(&expanded);
    
    activity_widget.connect_mode_notify(|f| {
        let l = f.mode();
        println!("ch: {:?}", l);
    });
    activity.widget=activity_widget.clone();

    activity
        .add_dynamic_property("mode", ActivityMode::Minimal)
        .unwrap();
    //set mode when updated
    activity
        .subscribe_to_property("mode", move |new_value| {
            let real_value = cast_dyn_prop!(new_value, ActivityMode).unwrap();
            activity_widget.set_mode(real_value);
        })
        .unwrap();

    activity
        .add_dynamic_property("comp-label", "compact".to_string())
        .unwrap();
    //set label when updated
    activity
        .subscribe_to_property("comp-label", move |new_value| {
            let real_value = cast_dyn_prop!(new_value, String).unwrap();
            compact.clone()
            .downcast::<gtk::Box>().unwrap()
            .children().get(0).unwrap().clone()
            .downcast::<gtk::Label>().unwrap()
            .set_label(real_value);
        })
        .unwrap();

    activity
}

fn get_window() -> gtk::Window {
    gtk::Window::builder()
        .title("test")
        .type_(gtk::WindowType::Toplevel)
        .height_request(500)
        .width_request(800)
        .build()
}

fn get_act_widget() -> ActivityWidget {
    let activity_widget = ActivityWidget::new();
    activity_widget.set_vexpand(false);
    activity_widget.set_hexpand(false);
    activity_widget.set_valign(gtk::Align::Start);
    activity_widget.set_halign(gtk::Align::Center);
    activity_widget.set_transition_duration(2000);
    activity_widget.style_context().add_class("overlay");
    activity_widget
}

fn get_bg() -> gtk::Widget {
    let background = gtk::Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .build();
    background.upcast()
}

fn get_minimal() -> gtk::Widget {
    let minimal = gtk::Box::builder()
        .height_request(40)
        .width_request(50)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    minimal.add(
        &gtk::Label::builder()
            .label("m")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    minimal.upcast()
}

fn get_compact() -> gtk::Widget {
    let compact = gtk::Box::builder()
        .height_request(40)
        .width_request(180)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    compact.add(
        &gtk::Label::builder()
            .label("compact")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    compact.upcast()
}

fn get_expanded() -> gtk::Widget {
    let expanded = gtk::Box::builder()
        .height_request(100)
        .width_request(350)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    expanded.add(
        &gtk::Label::builder()
            .label("Expanded label,\n Hello Hello")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    expanded.upcast()
}
