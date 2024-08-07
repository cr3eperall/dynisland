use std::{collections::HashMap, path::PathBuf, rc::Rc, thread};

use abi_stable::{
    external_types::crossbeam_channel::RSender,
    library::{lib_header_from_path, LibraryError},
    std_types::{
        RBoxError,
        RResult::{self, RErr, ROk},
        RString,
    },
    type_layout::TypeLayout,
    StableAbi,
};
use anyhow::Result;
use colored::Colorize;
use dynisland_core::graphics::activity_widget::boxed_activity_mode::ActivityMode;
use gtk::{prelude::*, CssProvider, Widget};
use notify::Watcher;
use ron::ser::PrettyConfig;
use tokio::sync::{mpsc::unbounded_channel, Mutex};

use crate::{
    config::{self, Config, GeneralConfig},
    layout_manager::simple_layout,
};

use dynisland_abi::{
    layout::{LayoutManagerBuilderRef, LayoutManagerType},
    module::{ModuleBuilderRef, ModuleType, UIServerCommand},
    SabiApplication,
};

pub enum BackendServerCommand {
    ReloadConfig(),
}

pub struct App {
    pub application: gtk::Application,
    // pub window: gtk::Window,
    pub module_map: Rc<Mutex<HashMap<String, ModuleType>>>,
    pub layout: Option<Rc<Mutex<(String, LayoutManagerType)>>>,
    // pub producers_handle: Handle,
    // pub producers_shutdown: tokio::sync::mpsc::Sender<()>,
    pub app_send: Option<RSender<UIServerCommand>>,
    pub config: Config,
    pub css_provider: CssProvider,
}

pub const MODS_DEBUG_PATH: &str =
    "/home/david/dev/rust/dynisland/dynisland-core/target/debug/libexample_module.so";

impl App {
    pub fn initialize_server(mut self) -> Result<()> {
        log::info!("pid: {}", std::process::id());
        //default css

        let (abi_app_send, abi_app_recv) =
            abi_stable::external_types::crossbeam_channel::unbounded::<UIServerCommand>();

        self.app_send = Some(abi_app_send);

        let (app_send_async, mut app_recv_async) = unbounded_channel::<UIServerCommand>();

        //forward message to app reciever
        thread::spawn(move || {
            while let Ok(msg) = abi_app_recv.recv() {
                app_send_async.send(msg).expect("failed to send message");
            }
        });

        self.config = config::get_config();

        let module_order = self.load_modules();
        self.load_layout_manager();

        self.load_layout_config();
        self.load_configs();

        self.init_loaded_modules(&module_order);

        log::debug!("Default config: {}",self.get_default_config());

        // let app_send1=self.app_send.clone().unwrap();
        // glib::MainContext::default().spawn_local(async move {
        //     glib::timeout_future_seconds(10).await;
        //     debug!("reloading config");
        //     app_send1.send(UIServerCommand::ReloadConfig()).unwrap();
        // });

        let (start_signal_tx, start_signal_rx) = tokio::sync::broadcast::channel::<()>(1);

        let layout = self.layout.clone().unwrap();
        // let act_container1 = act_container.clone();
        self.application.connect_activate(move |_app| {
            log::info!("Loading LayoutManager");
            layout.blocking_lock().1.init();
            start_signal_tx.send(()).unwrap();
            // let window = gtk::ApplicationWindow::new(app);
            // window.set_child(Some(&widget));

            // init_window(&window.clone().upcast());
            // // gtk::Window::set_interactive_debugging(true);

            // //show window
            // window.connect_destroy(|_| std::process::exit(0));
            // window.present();

            // crate::start_fps_counter(&window, log::Level::Trace, Duration::from_millis(200));
        });
        let layout = self.layout.clone().unwrap();
        let module_map = self.module_map.clone();
        //UI command consumer
        let mut start_signal = start_signal_rx.resubscribe();
        glib::MainContext::default().spawn_local(async move {
            start_signal.recv().await.unwrap();
            // TODO check if there are too many tasks on the UI thread and it begins to lag
            while let Some(command) = app_recv_async.recv().await {
                match command {
                    UIServerCommand::AddActivity(activity_identifier, activity) => {
                        let activity: Widget = match activity.try_into() {
                            Ok(act) => {act},
                            Err(err) => {
                                log::error!("error while converting SabiWidget to Widget, maybe it was deallocated after UIServerCommand::AddActivity was sent: {err:#?}");
                                continue;
                            },
                        };

                        Self::update_general_configs_on_activity(
                            &self.config.general_style_config,
                            &activity,
                        );

                        layout
                            .lock()
                            .await
                            .1
                            .add_activity(&activity_identifier, activity.into());
                        log::info!("registered activity on {}", activity_identifier.module());
                    }
                    UIServerCommand::RemoveActivity(activity_identifier) => {
                        layout.lock().await.1.remove_activity(&activity_identifier);
                    }
                    UIServerCommand::RestartProducers(module) => {
                        if let Some(module) = module_map.lock().await.get(module.as_str()) {
                            module.restart_producers();
                        }
                    }
                }
            }
        });

        let (server_send, mut server_recv) = unbounded_channel::<BackendServerCommand>();
        let app = self.application.clone();
        let mut start_signal = start_signal_rx.resubscribe();
        //server command consumer
        glib::MainContext::default().spawn_local(async move {
            start_signal.recv().await.unwrap();
            let renderer_name = self.application.windows()[0]
                .native()
                .unwrap()
                .renderer()
                .type_()
                .name();

            log::info!("using renderer: {}", renderer_name);

            let fallback_provider = gtk::CssProvider::new();
            fallback_provider.load_from_string(include_str!("../default.css"));
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &fallback_provider,
                gtk::STYLE_PROVIDER_PRIORITY_SETTINGS,
            );

            //init css provider
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &self.css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
            self.load_css(); //load user's scss

            self.restart_producer_runtimes();
            while let Some(command) = server_recv.recv().await {
                match command {
                    BackendServerCommand::ReloadConfig() => {
                        //FIXME split config and css reload (producers don't need to be restarted if only css changed)

                        // without this sleep, reading the config file sometimes gives an empty file.
                        glib::timeout_future(std::time::Duration::from_millis(50)).await;
                        self.load_configs();
                        self.update_general_configs();
                        self.load_layout_config();
                        self.load_css();

                        self.restart_producer_runtimes();
                    }
                }
            }
        });
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(evt) => {
                    // log::info!("config event: {:?}",evt.kind);
                    match evt.kind {
                        notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                            server_send
                                .send(BackendServerCommand::ReloadConfig())
                                .expect("failed to send notification")
                        }
                        notify::EventKind::Create(_) => {
                            // log::info!("file create event");
                        }
                        _ => {}
                    }
                    // debug!("{evt:?}");
                }
                Err(err) => {
                    log::error!("notify watcher error: {err}")
                }
            })
            .expect("failed to get file watcher");
        if let Err(err) = watcher.watch(
            &config::get_config_path(),
            notify::RecursiveMode::NonRecursive,
        ) {
            log::warn!("failed to start config file watcher, restart dynisland to get automatic config updates: {err}")
        }
        //start application
        app.run();
        Ok(())
    }

    pub fn load_css(&mut self) {
        let css_content = grass::from_path(
            config::get_config_path().join("dynisland.scss"),
            &grass::Options::default(),
        );
        match css_content {
            Ok(content) => {
                self.css_provider //TODO maybe save previous state before trying to update
                    .load_from_string(&content);
            }
            Err(err) => {
                log::warn!("{} {}", "failed to parse css:".red(), err.to_string().red());
            }
        }
    }

    pub fn load_modules(&mut self) -> Vec<String> {
        let mut module_order = vec![];
        let mut module_def_map = HashMap::<
            String,
            extern "C" fn(RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>,
        >::new();
        let module_path = {
            #[cfg(debug_assertions)]
            {
                PathBuf::from("/home/david/dev/rust/dynisland/dynisland-core/target/debug/")
            }

            #[cfg(not(debug_assertions))]
            {
                config::get_config_path().join("modules")
            }
        };
        let files = std::fs::read_dir(module_path).unwrap();
        for file in files {
            let file = file.unwrap();
            let path = file.path();
            if !path.is_file() {
                continue;
            }
            match file
                .file_name()
                .to_str()
                .unwrap()
                .to_lowercase()
                .strip_suffix(".so")
            {
                Some(name) => {
                    if !name.ends_with("module") {
                        continue;
                    }
                }
                None => continue,
            }
            log::debug!("loading module file: {:#?}", path);

            let res = (|| {
                let header = lib_header_from_path(&path)?;
                // header.init_root_module::<ModuleBuilderRef>()
                let layout1 = ModuleBuilderRef::LAYOUT;
                let layout2 = header.layout().unwrap();
                ensure_compatibility(layout1, layout2).and_then(|_| unsafe {
                    header
                        .unchecked_layout::<ModuleBuilderRef>().map_err(|err|{
                            err.into_library_error::<ModuleBuilderRef>()
                        })
                })
            })();

            let module_builder = match res {
                Ok(x) => x,
                Err(e) => {
                    log::error!(
                        "error while loading {}: {e:#?}",
                        path.file_name().unwrap().to_str().unwrap()
                    );
                    continue;
                }
            };
            let name = module_builder.name();
            let constructor = module_builder.new();

            module_def_map.insert(name.into(), constructor);
        }

        if self.config.loaded_modules.contains(&"all".to_string()) {
            //load all modules available in order of hash (random order)
            for module_def in module_def_map {
                let module_name = module_def.0;
                let module_constructor = module_def.1;

                let built_module = match module_constructor(self.app_send.clone().unwrap()) {
                    ROk(x) => x,
                    RErr(e) => {
                        log::error!("error during creation of {module_name}: {e:#?}");
                        continue;
                    }
                };

                module_order.push(module_name.to_string());
                self.module_map
                    .blocking_lock()
                    .insert(module_name.to_string(), built_module);
            }
        } else {
            //load only modules in the config in order of definition
            for module_name in self.config.loaded_modules.iter() {
                let module_constructor = module_def_map.get(module_name);
                let module_constructor = match module_constructor {
                    None => {
                        log::info!("module {} not found, skipping", module_name);
                        continue;
                    }
                    Some(x) => x,
                };

                let built_module = match module_constructor(self.app_send.clone().unwrap()) {
                    ROk(x) => x,
                    RErr(e) => {
                        log::error!("error during creation of {module_name}: {e:#?}");
                        continue;
                    }
                };
                module_order.push(module_name.to_string());
                // info!("loading module {}", module.get_name());
                self.module_map
                    .blocking_lock()
                    .insert(module_name.to_string(), built_module);
            }
        }

        log::info!("loaded modules: {:?}", module_order);
        module_order
    }

    //TODO layout loading from .so not tested yet but it should work identically to module loading
    fn load_layout_manager(&mut self) {
        let mut lm_def_map = HashMap::<
            String,
            extern "C" fn(SabiApplication) -> RResult<LayoutManagerType, RBoxError>,
        >::new();
        let lm_path = {
            #[cfg(debug_assertions)]
            {
                PathBuf::from("/home/david/dev/rust/dynisland/dynisland-core/target/debug/")
            }
            #[cfg(not(debug_assertions))]
            {
                config::get_config_path().join("layouts")
            }
        };
        let files = std::fs::read_dir(lm_path).unwrap();
        for file in files {
            let file = file.unwrap();
            let path = file.path();
            if !path.is_file() {
                continue;
            }
            match file
                .file_name()
                .to_str()
                .unwrap()
                .to_lowercase()
                .strip_suffix(".so")
            {
                Some(name) => {
                    if !name.ends_with("layoutmanager") {
                        continue;
                    }
                }
                None => continue,
            }
            log::debug!("loading layout manager file: {:#?}", path);

            let res = (|| {
                let header = lib_header_from_path(&path)?;
                header.init_root_module::<LayoutManagerBuilderRef>()
            })();

            let lm_builder = match res {
                Ok(x) => x,
                Err(e) => {
                    log::error!(
                        "error while loading {}: {e:#?}",
                        path.file_name().unwrap().to_str().unwrap()
                    );
                    continue;
                }
            };
            let name = lm_builder.name();
            let constructor = lm_builder.new();

            lm_def_map.insert(name.into(), constructor);
        }

        if self.config.layout.is_none() {
            log::info!("no layout manager in config, using default: SimpleLayout");
            self.load_simple_layout();
            return;
        }
        let lm_name = self.config.layout.as_ref().unwrap();
        if lm_name == simple_layout::NAME {
            log::info!("using layout manager: SimpleLayout");
            self.load_simple_layout();
            return;
        }
        let lm_constructor = lm_def_map.get(lm_name);
        let lm_constructor = match lm_constructor {
            None => {
                log::info!(
                    "layout manager {} not found, using default: SimpleLayout",
                    lm_name
                );
                self.load_simple_layout();
                return;
            }
            Some(x) => x,
        };

        let built_lm = match lm_constructor(self.application.clone().into()) {
            ROk(x) => x,
            RErr(e) => {
                log::error!("error during creation of {lm_name}: {e:#?}");
                log::info!("using default layout manager SimpleLayout");
                self.load_simple_layout();
                return;
            }
        };
        log::info!("using layout manager: {lm_name}");
        self.layout = Some(Rc::new(Mutex::new((lm_name.clone(), built_lm))));
    }

    fn load_simple_layout(&mut self) {
        let layout_builder = simple_layout::new(self.application.clone().into());
        let layout = layout_builder.unwrap();
        self.layout = Some(Rc::new(Mutex::new((
            simple_layout::NAME.to_string(),
            layout,
        ))));
    }

    #[allow(dead_code)]
    fn get_default_config(&self) -> Config {
        let mut base_conf = Config::default();
        let (layout_name, layout_config) = {
            let layout = self.layout.as_ref().unwrap().blocking_lock();
            (layout.0.to_owned(), layout.1.default_config())
        };
        let module_configs: Vec<(String, RResult<RString, RBoxError>)> = self
            .module_map
            .blocking_lock()
            .iter()
            .map(|(k, v)| (k.to_owned(), v.default_config()))
            .collect();
        base_conf.layout = Some(layout_name.clone());
        match layout_config {
            ROk(conf) => match ron::de::from_str(conf.as_str()) {
                Ok(value) => {
                    base_conf.layout_configs.insert(layout_name, value);
                }
                Err(err) => {
                    log::error!("deserializing error: {err}");
                }
            },
            RErr(err) => {
                log::error!("get default config error: {err}");
            }
        }
        for (module_name, config) in module_configs {
            match config {
                ROk(conf) => {
                    log::debug!("string config for {module_name}: {}",conf.as_str()); //TODO find a way to get the original string in Config
                    match ron::de::from_str(conf.as_str()) {
                        Ok(value) => {
                            base_conf.module_config.insert(module_name, value);
                        }
                        Err(err) => {
                            log::warn!("cannot get default config for {module_name}: {err}");
                        },
                    }
                }
                RErr(err) => {
                    log::warn!("cannot get default config for {module_name}: {err}");
                },
            }
        }
        base_conf
    }

    fn load_configs(&mut self) {
        self.config = config::get_config();
        log::debug!("general_config: {:#?}", self.config.general_style_config);
        for (module_name, module) in self.module_map.blocking_lock().iter_mut() {
            log::info!("loading config for module: {:#?}", module_name);
            let config_to_parse = self.config.module_config.get(module_name);
            let config_parsed = match config_to_parse {
                Some(conf) => {
                    let confs: RString = ron::ser::to_string_pretty(&conf, PrettyConfig::default())
                        .unwrap()
                        .into();
                    // log::debug!("config for : {}", confs);
                    module.update_config(confs)
                }
                None => {
                    log::debug!("no config for module: {:#?}", module_name);
                    ROk(())
                }
            };
            match config_parsed {
                RErr(err) => {
                    log::error!("failed to parse config for module {}: {err:?}", module_name)
                }
                ROk(()) => {
                    // debug!("{}: {:#?}", module_name, config_to_parse);
                }
            }
        }
    }

    //TODO let the modules handle this, something like module.update_general_config or module.update_config itself
    fn update_general_configs(&self) {
        let layout = self.layout.clone().unwrap();
        let layout = layout.blocking_lock();
        let activities = layout.1.list_activities();
        for activity in activities {
            let activity: Widget = layout.1.get_activity(activity).unwrap().try_into().unwrap();
            Self::update_general_configs_on_activity(&self.config.general_style_config, &activity);
        }
    }

    fn update_general_configs_on_activity(config: &GeneralConfig, activity: &Widget) {
        // let activity_widget = activity.clone().downcast::<ActivityWidget>();
        // match activity_widget {
        //     Ok(activity) => log::debug!("widget is an ActivityWidget"),
        //     Err(err) => log::debug!("widget is NOT an ActivityWidget"),
        // }
        //TODO define property names as constants
        activity.set_property("config-minimal-height-app", config.minimal_height as i32);
        activity.set_property("config-minimal-width-app", config.minimal_width as i32);
        activity.set_property("config-blur-radius-app", config.blur_radius);
        activity.set_property("mode", activity.property::<ActivityMode>("mode"));
        activity.set_property("config-enable-drag-stretch-app", config.enable_drag_stretch);
    }

    fn init_loaded_modules(&self, order: &Vec<String>) {
        let module_map = self.module_map.blocking_lock();
        for module_name in order {
            let module = module_map.get(module_name).unwrap();
            module.init();
        }
    }

    fn load_layout_config(&self) {
        let layout = self.layout.clone().unwrap();
        let mut layout = layout.blocking_lock();
        let layout_name = layout.0.clone();
        if let Some(config) = self.config.layout_configs.get(&layout_name) {
            let confs: RString = ron::ser::to_string_pretty(&config, PrettyConfig::default())
                .unwrap()
                .into();
            match layout.1.update_config(confs) {
                ROk(()) => {
                    log::info!("loaded layout config for {layout_name}");
                }
                RErr(err) => {
                    log::error!("failed to parse layout config for {layout_name}, {err}");
                }
            }
        } else {
            log::info!("no layout config found for {layout_name}, using Default");
        }
    }

    fn restart_producer_runtimes(&self) {
        for module in self.module_map.blocking_lock().values_mut() {
            module.restart_producers();
        }
    }
}

impl Default for App {
    fn default() -> Self {
        // let (hdl, shutdown) = get_new_tokio_rt();
        let app =
            gtk::Application::new(Some("com.github.cr3eperall.dynisland"), Default::default());

        App {
            application: app,
            module_map: Rc::new(Mutex::new(HashMap::new())),
            layout: None,
            // producers_handle: hdl,
            // producers_shutdown: shutdown,
            app_send: None,
            config: config::Config::default(),
            css_provider: gtk::CssProvider::new(),
        }
    }
}

fn ensure_compatibility(
    interface: &'static TypeLayout,
    implementation: &'static TypeLayout,
) -> Result<(), abi_stable::library::LibraryError> {
    let compatibility = abi_stable::abi_stability::abi_checking::check_layout_compatibility(
        interface,
        implementation,
    );
    if let Err(err) = compatibility {
        let incompatibilities = err.errors.iter().filter(|e| !e.errs.is_empty());
        let fatal_incompatibilities = incompatibilities.filter(|err| {
            err.errs.iter().any(|err| {
                !matches!(
                    err,
                    abi_stable::abi_stability::abi_checking::AbiInstability::FieldCountMismatch(assert) if assert.expected > assert.found
                )
            })
        });
        if fatal_incompatibilities.count() > 0 {
            return Err(LibraryError::AbiInstability(RBoxError::new(err)));
        }
    }
    Ok(())
}
