use std::sync::Arc;

use anyhow::{Result, Context};
use gtk::prelude::*;
use tokio::{sync::{Mutex, mpsc::unbounded_channel}, runtime::Runtime};

use crate::{modules::example, widgets::dynamic_activity::DynamicActivity};

pub enum ServerCommand{
    AddActivity(Arc<Mutex<DynamicActivity>>),
    AddProducer(fn(activity: &[Arc<Mutex<DynamicActivity>>], rt: &Runtime)),
    //TODO add remove activity and producer
}


pub fn initialize_server() -> Result<()>{
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
    let act_container=gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Start)
        .build();

    gtk::Window::set_interactive_debugging(true);
    
    window.add(&act_container);

    //show window
    window.connect_destroy(|_| std::process::exit(0));
    window.show_all();
    
    let (app_send, mut app_recv) = unbounded_channel::<ServerCommand>();
    
    let example_mod=example::ExampleModule::new(app_send);
    example_mod.init();
    
    //create tokio runtime used for data producers
    let rt = get_new_tokio_rt();

    glib::MainContext::default().spawn_local(async move {
        let mut v: Vec<Arc<Mutex<DynamicActivity>>>=vec![];
        while let Some(command) = app_recv.recv().await {
            match command{
                ServerCommand::AddProducer(producer) =>{
                    producer(v.as_slice(), &rt);
                },
                ServerCommand::AddActivity(activity) => {
                    act_container.add(&activity.lock().await.get_activity_widget());
                    act_container.show_all();
                    v.push(activity);
                },
            }
        }
    });
    
    //start application
    gtk::main();
    Ok(())
}

fn get_window() -> gtk::Window {
    let window =gtk::Window::builder()
        .title("test")
        .type_(gtk::WindowType::Popup)
        .has_focus(true)
        .height_request(500)
        .width_request(800)
        .build();
    // gtk_layer_shell::init_for_window(&window);
    // gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    // gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 5);

    window

}

fn get_new_tokio_rt()-> Arc<Runtime> {
    let (rt_send, rt_recv)= tokio::sync::oneshot::channel::<Arc<Runtime>>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("idk tokio rt failed");
        let rt=Arc::new(rt);
        rt_send.send(rt.clone()).expect("failed to send rt");
        rt.block_on(std::future::pending::<()>()); //keep thread alive
    });
    
    rt_recv.blocking_recv().expect("failed to receive rt")
}
