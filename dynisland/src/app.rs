use std::{collections::HashMap, rc::Rc};

use anyhow::Result;
use colored::Colorize;
use gtk::{prelude::*, CssProvider};
use log::{debug, error, info};
use notify::Watcher;
use tokio::{
    runtime::Handle,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
};

use crate::config::{self, Config, GeneralConfig};

use dynisland_core::base_module::{Module, UIServerCommand, MODULES};

pub enum BackendServerCommand {
    ReloadConfig(),
}

pub struct App {
    pub window: gtk::Window,
    pub module_map: Rc<Mutex<HashMap<String, Box<dyn Module>>>>,
    pub producers_handle: Handle,
    pub producers_shutdown: tokio::sync::mpsc::Sender<()>,
    pub app_send: Option<UnboundedSender<UIServerCommand>>,
    pub config: Config,
    pub css_provider: CssProvider,
}

impl App {
    pub fn initialize_server(mut self) -> Result<()> {
        //parse static scss file
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().unwrap(),
            &self.css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        self.load_css();
        let act_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Start)
            .margin_top(10)
            .build();

        // gtk::Window::set_interactive_debugging(true);

        self.window.add(&act_container);

        //show window
        self.window.connect_destroy(|_| std::process::exit(0));
        self.window.show_all();

        let (app_send, mut app_recv) = unbounded_channel::<UIServerCommand>();

        self.app_send = Some(app_send.clone());

        self.load_modules();
        self.load_configs();
        self.init_loaded_modules();

        let rt = self.producers_handle.clone();
        let map = self.module_map.clone();

        // let app_send1=self.app_send.clone().unwrap();
        // glib::MainContext::default().spawn_local(async move {
        //     glib::timeout_future_seconds(10).await;
        //     debug!("reloading config");
        //     app_send1.send(UIServerCommand::ReloadConfig()).unwrap();
        // });

        //UI command consumer
        glib::MainContext::default().spawn_local(async move {
            // TODO check if there are too many tasks on the UI thread and it begins to lag
            while let Some(command) = app_recv.recv().await {
                match command {
                    UIServerCommand::AddProducer(module_identifier, producer) => {
                        let map = map.lock().await;
                        let module = map
                            .get(&module_identifier)
                            .unwrap_or_else(|| panic!("module {} not found", module_identifier));

                        module.register_producer(producer).await; //add inside module
                        producer(
                            // execute //TODO make sure this doesn't get executed twice at the same time when the configuration is being reloaded
                            module.get_registered_activities(),
                            &rt,
                            app_send.clone(),
                            module.get_prop_send(),
                            module.get_config(),
                        );
                        info!("registered producer {}", module.get_name());
                    }
                    UIServerCommand::AddActivity(module_identifier, activity) => {
                        act_container.add(&activity.lock().await.get_activity_widget()); //add to window
                        act_container.show_all();
                        Self::update_general_configs_on_activity(
                            &self.config.general_config,
                            &activity,
                        )
                        .await;
                        let map = map.lock().await;
                        let module = map
                            .get(&module_identifier)
                            .unwrap_or_else(|| panic!("module {} not found", module_identifier));
                        module.register_activity(activity).await; //add inside its module
                        info!("registered activity on {}", module.get_name());
                    }
                    UIServerCommand::RemoveActivity(module_identifier, name) => {
                        let map = map.lock().await;
                        let module = map
                            .get(&module_identifier)
                            .unwrap_or_else(|| panic!("module {} not found", module_identifier));
                        let activity_map = module.get_registered_activities();
                        let activity_map = activity_map.lock().await;
                        let act = activity_map.get(&name).unwrap_or_else(|| {
                            panic!(
                                "activity {} not found on module {}",
                                name, module_identifier
                            )
                        });
                        act_container.remove(&act.lock().await.get_activity_widget());
                        module.unregister_activity(&name).await;
                    }
                }
            }
        });

        let (server_send, mut server_recv) = unbounded_channel::<BackendServerCommand>();

        //server command consumer
        glib::MainContext::default().spawn_local(async move {
            while let Some(command) = server_recv.recv().await {
                match command {
                    BackendServerCommand::ReloadConfig() => {
                        // without this sleep, reading the config file sometimes gives an empty file.
                        glib::timeout_future(std::time::Duration::from_millis(50)).await;
                        self.load_configs();
                        self.update_general_configs().await;
                        self.restart_producer_rt();

                        self.load_css();
                    }
                }
            }
        });
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(evt) => {
                    match evt.kind {
                        notify::EventKind::Create(_) | notify::EventKind::Modify(_) => server_send
                            .send(BackendServerCommand::ReloadConfig())
                            .expect("failed to send notification"),
                        _ => {}
                    }
                    // debug!("{evt:?}");
                }
                Err(err) => {
                    error!("notify watcher error: {err}")
                }
            })
            .expect("failed to start file watcher");
        watcher
            .watch(
                &config::get_config_path(),
                notify::RecursiveMode::NonRecursive,
            )
            .expect("error starting watcher");

        //start application
        gtk::main();
        Ok(())
    }

    pub fn load_css(&mut self) {
        let css_content = grass::from_path(
            config::get_config_path().join("dynisland.scss"),
            &grass::Options::default(),
        );
        if let Err(err) = css_content {
            error!(
                "{} {:?}",
                "failed to parse css:".red(),
                err.to_string().red()
            );
            return;
        }

        gtk::StyleContext::remove_provider_for_screen(
            &gdk::Screen::default().unwrap(),
            &self.css_provider,
        );
        //setup static css style
        self.css_provider //TODO save previous state before trying to update
            .load_from_data(css_content.unwrap().as_bytes())
            .unwrap_or_else(|err| {
                error!(
                    "{} {:?}",
                    "failed to load css:".red(),
                    err.to_string().red()
                )
            });
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().unwrap(),
            &self.css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    pub fn load_modules(&mut self) {
        for module_new in MODULES.iter() {
            let module = module_new(self.app_send.as_ref().unwrap().clone(), None);

            self.module_map
                .blocking_lock()
                .insert(module.get_name().to_string(), module);
        }
        // let example_mod =
        //     example::ExampleModule::new(self.app_send.as_ref().unwrap().clone(), None);
        // self.module_map
        //     .blocking_lock()
        //     .insert(example_mod.get_name().to_string(), example_mod);

        // let example_mod2 =
        //     example::ExampleModule2::new(self.app_send.as_ref().unwrap().clone(), None);
        // self.module_map
        //     .blocking_lock()
        //     .insert(example_mod2.get_name().to_string(), Box::new(example_mod2));

        info!(
            "loaded modules: {:?}",
            self.module_map.blocking_lock().keys()
        );
    }

    fn load_configs(&mut self) {
        self.config = config::get_config();
        debug!("general_config: {:#?}", self.config.general_config);
        for module in self.module_map.blocking_lock().values_mut() {
            let config_parsed = match self.config.module_config.get(module.get_name()) {
                Some(conf) => module.parse_config(conf.clone()),
                None => Ok(()),
            };
            match config_parsed {
                Err(err) => {
                    error!(
                        "failed to parse config for module {}: {err:?}",
                        module.get_name()
                    )
                }
                Ok(()) => {
                    debug!("{}: {:#?}", module.get_name(), module.get_config());
                }
            }
        }
    }

    async fn update_general_configs(&mut self) {
        for module in self.module_map.blocking_lock().values_mut() {
            for activity in module.get_registered_activities().lock().await.values() {
                Self::update_general_configs_on_activity(&self.config.general_config, activity)
                    .await;
            }
        }
    }

    async fn update_general_configs_on_activity(
        config: &GeneralConfig,
        activity: &Mutex<dynisland_core::base_module::DynamicActivity>,
    ) {
        let activity = activity.lock().await;
        activity
            .get_activity_widget()
            .set_minimal_height(config.minimal_height as i32, false)
            .expect("failed to set minimal-height");
        activity
            .get_activity_widget()
            .set_transition_duration(config.transition_duration, false)
            .expect("failed to set transition-duration");
        activity
            .get_activity_widget()
            .set_transition_size(Box::new(config.transition_size), false)
            .expect("failed to set transition-size");
        activity
            .get_activity_widget()
            .set_transition_bigger_blur(Box::new(config.transition_bigger_blur), false)
            .expect("failed to set transition-bigger-blur");
        activity
            .get_activity_widget()
            .set_transition_bigger_stretch(Box::new(config.transition_bigger_stretch), false)
            .expect("failed to set transition-bigger-stretch");
        activity
            .get_activity_widget()
            .set_transition_bigger_opacity(Box::new(config.transition_bigger_opacity), false)
            .expect("failed to set transition-bigger-opacity");
        activity
            .get_activity_widget()
            .set_transition_smaller_blur(Box::new(config.transition_smaller_blur), false)
            .expect("failed to set transition-smaller-blur");
        activity
            .get_activity_widget()
            .set_transition_smaller_stretch(Box::new(config.transition_smaller_stretch), false)
            .expect("failed to set transition-smaller-stretch");
        activity
            .get_activity_widget()
            .set_transition_smaller_opacity(Box::new(config.transition_smaller_opacity), false)
            .expect("failed to set transition-smaller-opacity");
    }

    fn init_loaded_modules(&self) {
        for module in self.module_map.blocking_lock().values() {
            module.init();
        }
    }

    fn restart_producer_rt(&mut self) {
        self.producers_shutdown
            .blocking_send(())
            .expect("failed to shutdown old producer runtime"); //stop current producers_runtime
        let (handle, shutdown) = get_new_tokio_rt(); //start new producers_runtime
        self.producers_handle = handle;
        self.producers_shutdown = shutdown;
        for module in self.module_map.blocking_lock().values() {
            //restart producers
            for producer in module.get_registered_producers().blocking_lock().iter() {
                producer(
                    module.get_registered_activities(),
                    &self.producers_handle,
                    self.app_send.clone().unwrap(),
                    module.get_prop_send(),
                    module.get_config(),
                )
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        let (hdl, shutdown) = get_new_tokio_rt();
        App {
            window: get_window(),
            module_map: Rc::new(Mutex::new(HashMap::new())),
            producers_handle: hdl,
            producers_shutdown: shutdown,
            app_send: None,
            config: config::Config::default(),
            css_provider: gtk::CssProvider::new(),
        }
    }
}

// // doesn't work when called trough a function, idk why
// fn init_notifiers(server_send: UnboundedSender<BackendServerCommand>) {
//     let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
//         match res {
//             Ok(evt) => {
//                 match evt.kind {
//                     notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
//                         debug!("filesystem event");
//                         server_send.send(BackendServerCommand::ReloadConfig()).expect("failed to send notification")
//                     },
//                     _ => {}
//                 }
//                 debug!("{evt:?}");
//             },
//             Err(err) => {error!("notify watcher error: {err}")},
//         }
//     }).expect("failed to start file watcher");
//     watcher.watch(Path::new(config::CONFIG_FILE), notify::RecursiveMode::NonRecursive).expect("error starting watcher");
// }

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

pub fn get_new_tokio_rt() -> (Handle, tokio::sync::mpsc::Sender<()>) {
    let (rt_send, rt_recv) =
        tokio::sync::oneshot::channel::<(Handle, tokio::sync::mpsc::Sender<()>)>();
    let (shutdown_send, mut shutdown_recv) = tokio::sync::mpsc::channel::<()>(1);
    std::thread::Builder::new()
        .name("dyn-producers".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("idk tokio rt failed");
            let handle = rt.handle();
            rt_send
                .send((handle.clone(), shutdown_send))
                .expect("failed to send rt");
            rt.block_on(async { shutdown_recv.recv().await }); //keep thread alive
        })
        .expect("failed to spawn new trhread");

    rt_recv.blocking_recv().expect("failed to receive rt")
}