use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use gtk::prelude::*;
use tokio::{
    runtime::Runtime,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
};

use crate::{
    modules::example::{self, ExampleModule},
    widgets::dynamic_activity::DynamicActivity,
};

pub type Producer = fn(
    activities: Arc<Mutex<HashMap<String, Arc<Mutex<DynamicActivity>>>>>,
    rt: &Runtime,
    app_send: UnboundedSender<ServerCommand>,
);

pub enum ServerCommand {
    AddActivity(String, Arc<Mutex<DynamicActivity>>),
    AddProducer(String, Producer),
    //TODO add remove activity and producer
}

pub struct App {
    pub window: gtk::Window,
    pub module_map: Arc<Mutex<HashMap<String, ExampleModule>>>,
    pub producers_runtime: Arc<Runtime>,
    pub app_send: Option<UnboundedSender<ServerCommand>>,
}

impl App {
    pub fn initialize_server(&mut self) -> Result<()> {
        //parse static scss file
        let css_content = grass::from_path(
            "/home/david/dev/rust/dynisland/file.scss",
            &grass::Options::default(),
        );

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
        let act_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Start)
            .build();

        gtk::Window::set_interactive_debugging(true);

        self.window.add(&act_container);

        //show window
        self.window.connect_destroy(|_| std::process::exit(0));
        self.window.show_all();

        let (app_send, mut app_recv) = unbounded_channel::<ServerCommand>();
        self.app_send = Some(app_send.clone());

        self.load_modules();

        let rt = self.producers_runtime.clone();
        let map = self.module_map.clone();
        glib::MainContext::default().spawn_local(async move {
            while let Some(command) = app_recv.recv().await {
                match command {
                    ServerCommand::AddProducer(module, producer) => {
                        producer(
                            map.lock()
                                .await
                                .get(&module)
                                .unwrap_or_else(|| panic!("module {} not found", module))
                                .get_registered_activities(),
                            &rt,
                            app_send.clone(),
                        );
                    }
                    ServerCommand::AddActivity(module, activity) => {
                        act_container.add(&activity.lock().await.get_activity_widget());
                        act_container.show_all();
                        let map = map.lock().await;
                        let module = map
                            .get(&module)
                            .unwrap_or_else(|| panic!("module {} not found", module));
                        module.register_activity(activity).await;
                        println!("registered activity");
                    }
                }
            }
        });

        //start application
        gtk::main();
        Ok(())
    }

    pub fn load_modules(&mut self) {
        let example_mod = example::ExampleModule::new(self.app_send.as_ref().unwrap().clone());
        self.module_map
            .blocking_lock()
            .insert(example_mod.get_name().to_string(), example_mod);
        for module in self.module_map.blocking_lock().values() {
            module.init();
        }
    }
}

pub fn get_window() -> gtk::Window {
    gtk::Window::builder()
        .title("test")
        // .type_(gtk::WindowType::Popup)
        .has_focus(true)
        .height_request(500)
        .width_request(800)
        .build()
    // gtk_layer_shell::init_for_window(&window);
    // gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    // gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);
    // gtk_layer_shell::set_margin(&window, gtk_layer_shell::Edge::Top, 5);

    // window
}

pub fn get_new_tokio_rt() -> Arc<Runtime> {
    let (rt_send, rt_recv) = tokio::sync::oneshot::channel::<Arc<Runtime>>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("idk tokio rt failed");
        let rt = Arc::new(rt);
        rt_send.send(rt.clone()).expect("failed to send rt");
        rt.block_on(std::future::pending::<()>()); //keep thread alive
    });

    rt_recv.blocking_recv().expect("failed to receive rt")
}
