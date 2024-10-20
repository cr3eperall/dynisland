use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Path, PathBuf},
    rc::Rc,
    thread,
};

use abi_stable::{
    external_types::crossbeam_channel::RSender,
    std_types::{
        RBoxError, ROption,
        RResult::{self, RErr, ROk},
        RString,
    },
};
use anyhow::Result;
use dynisland_core::{
    abi::{
        abi_stable, gdk, glib,
        layout::LayoutManagerType,
        log,
        module::{ActivityIdentifier, ModuleType, UIServerCommand},
    },
    graphics::activity_widget::boxed_activity_mode::ActivityMode,
    ron,
};
use gtk::{prelude::*, CssProvider, Widget};
use notify::{RecommendedWatcher, Watcher};
use ron::{extensions::Extensions, ser::PrettyConfig};
use tokio::sync::{mpsc::unbounded_channel, Mutex};

use crate::{
    config::{self, Config, GeneralConfig},
    ipc::open_socket,
    layout_manager::{self, fallback_layout},
};

pub enum BackendServerCommand {
    ReloadConfig,
    Stop,
    OpenInspector,
    ActivityNotification(ActivityIdentifier, ActivityMode, Option<u64>),
    ListActivities,
    ListLoadedModules,
    ModuleCliCommand(String, String),
    LayoutCliCommand(String),
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
    pub config_dir: PathBuf,
}

impl App {
    pub fn run(mut self, config_dir: &Path) -> Result<()> {
        self.config = config::get_config(config_dir);
        self.config_dir = config_dir.to_path_buf();

        let (server_send, server_recv) = unbounded_channel::<BackendServerCommand>();
        let (server_response_send, server_response_recv) = unbounded_channel::<Option<String>>();
        let runtime_path = self.config.get_runtime_dir();

        let mut app_recv_async = self.init_abi_app_channel();

        // load layout manager and init modules
        self.load_layout_manager(config_dir);
        self.load_layout_config();

        let module_order = self.load_modules(config_dir);
        self.load_configs(config_dir);
        self.init_loaded_modules(&module_order);

        // init layout manager and send start signal
        let (start_signal_tx, start_signal_rx) = tokio::sync::broadcast::channel::<()>(1);
        let open_debugger = self
            .config
            .debug
            .clone()
            .map(|d| d.open_debugger_at_start)
            .unwrap_or(false);
        let layout = self.layout.clone().unwrap();
        self.application.connect_activate(move |_app| {
            log::info!("Loading LayoutManager");
            layout.blocking_lock().1.init();
            start_signal_tx.send(()).unwrap();
            gtk::Window::set_interactive_debugging(open_debugger);
        });

        //UI command consumer
        let mut start_signal = start_signal_rx.resubscribe();
        let layout = self.layout.clone().unwrap();
        let module_map = self.module_map.clone();
        glib::MainContext::default().spawn_local(async move {
            start_signal.recv().await.unwrap();

            // TODO check if there are too many tasks on the UI thread and it begins to lag
            while let Some(command) = app_recv_async.recv().await {
                match command {
                    UIServerCommand::AddActivity{activity_id, widget} => {
                        let activity: Widget = match widget.try_into() {
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

                        if layout
                            .lock()
                            .await
                            .1
                            .get_activity(&activity_id)
                            .is_some()
                        {
                            log::debug!("activity already registered on {}", activity_id.module());
                            continue;
                        }

                        layout
                            .lock()
                            .await
                            .1
                            .add_activity(&activity_id, activity.into());
                        log::info!("registered activity on {}", activity_id.module());
                    }
                    UIServerCommand::RemoveActivity { activity_id } => {
                        let mut layout = layout.lock().await;
                        if layout.1.get_activity(&activity_id).is_some(){
                            layout.1.remove_activity(&activity_id);
                            log::info!("unregistered activity on {}", activity_id.module());
                        }else{
                            log::warn!("error removing activity, not found: {:?}", activity_id);
                        }
                    }
                    UIServerCommand::RestartProducers { module_name } => {
                        if let Some(module) = module_map.lock().await.get(module_name.as_str()) {
                            module.restart_producers();
                        }
                    }
                    UIServerCommand::RequestNotification { activity_id, mode, duration} => {
                        if mode>3{
                            continue;
                        }
                        let layout = layout.lock().await;
                        if layout.1.get_activity(&activity_id).is_none(){
                            continue;
                        }
                        layout.1.activity_notification(&activity_id, mode, duration);
                    }
                }
            }
        });

        let app = self.application.clone();
        let mut start_signal = start_signal_rx.resubscribe();
        let conf_dir = config_dir.to_path_buf();
        //server command consumer
        glib::MainContext::default().spawn_local(async move {
            start_signal.recv().await.unwrap();

            let renderer_name = match self.application.windows()[0]
                .native()
                .expect("Layout manager has no windows")
                .renderer()
            {
                Some(renderer_type) => renderer_type.type_().name(),
                None => "no renderer found",
            };

            log::info!("Using renderer: {}", renderer_name);

            //init css providers
            let fallback_provider = gtk::CssProvider::new();
            let css =
                grass::from_string(include_str!("../default.scss"), &grass::Options::default())
                    .unwrap();
            fallback_provider.load_from_string(&css);
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &fallback_provider,
                gtk::STYLE_PROVIDER_PRIORITY_SETTINGS,
            );

            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &self.css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
            self.load_css(&conf_dir); //load user's scss

            self.restart_producer_runtimes(); // start producers

            self.start_backend_server(server_recv, server_response_send, conf_dir)
                .await;
        });

        let _wathcer = start_config_dir_watcher(server_send.clone(), &config_dir);

        //start application
        app.register(None as Option<&gtk::gio::Cancellable>)?;
        let running = app.is_remote();
        if running {
            log::error!("dynisland is already running");
        } else {
            start_ipc_server(runtime_path.clone(), server_send, server_response_recv);
        }
        app.run_with_args::<String>(&[]);
        if !running {
            std::fs::remove_file(runtime_path.join("dynisland.sock"))?;
        }
        Ok(())
    }

    fn init_abi_app_channel(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<UIServerCommand> {
        let (abi_app_send, abi_app_recv) =
            abi_stable::external_types::crossbeam_channel::unbounded::<UIServerCommand>();
        self.app_send = Some(abi_app_send);
        let (app_send_async, app_recv_async) = unbounded_channel::<UIServerCommand>();

        //forward message to app receiver
        thread::Builder::new()
            .name("abi-app-forwarder".to_string())
            .spawn(move || {
                while let Ok(msg) = abi_app_recv.recv() {
                    app_send_async.send(msg).expect("failed to send message");
                }
            })
            .expect("failed to spawn abi-app-forwarder thread");
        app_recv_async
    }

    async fn start_backend_server(
        mut self,
        mut server_recv: tokio::sync::mpsc::UnboundedReceiver<BackendServerCommand>,
        server_response_send: tokio::sync::mpsc::UnboundedSender<Option<String>>,
        config_dir: std::path::PathBuf,
    ) {
        while let Some(command) = server_recv.recv().await {
            match command {
                BackendServerCommand::ReloadConfig => {
                    log::info!("Reloading Config");
                    //TODO split config and css reload (producers don't need to be restarted if only css changed)

                    // without this sleep, reading the config file sometimes gives an empty file.
                    glib::timeout_future(std::time::Duration::from_millis(50)).await;
                    self.load_configs(&config_dir);
                    self.update_general_configs();
                    self.load_layout_config();
                    self.load_css(&config_dir);

                    self.restart_producer_runtimes();
                }
                BackendServerCommand::Stop => {
                    log::info!("Quitting");
                    let _ = server_response_send.send(None);
                    self.application.quit();
                }
                BackendServerCommand::OpenInspector => {
                    log::info!("Opening inspector");
                    let _ = server_response_send.send(None);
                    gtk::Window::set_interactive_debugging(true);
                }
                BackendServerCommand::ActivityNotification(id, mode, duration) => {
                    if let Err(err) =
                        self.app_send
                            .clone()
                            .unwrap()
                            .send(UIServerCommand::RequestNotification {
                                activity_id: id,
                                mode: mode as u8,
                                duration: ROption::from(duration),
                            })
                    {
                        let _ = server_response_send.send(Some(err.to_string()));
                        log::error!("{err}");
                    } else {
                        let _ = server_response_send.send(None);
                    }
                }
                BackendServerCommand::ListActivities => match self.layout.clone() {
                    Some(layout) => {
                        let activities = layout.lock().await.1.list_activities();
                        let mut response = String::new();
                        for activity in activities {
                            response += &activity.to_string();
                            response += "\n";
                        }
                        let _ = server_response_send.send(Some(response));
                    }
                    None => {
                        let _ = server_response_send.send(Some("no layout loaded".to_string()));
                    }
                },
                BackendServerCommand::ListLoadedModules => {
                    let mut response = String::new();
                    let mod_map = self.module_map.lock().await;
                    for module in mod_map.keys() {
                        response += &format!("{module}\n");
                    }
                    let _ = server_response_send.send(Some(response));
                }
                BackendServerCommand::ModuleCliCommand(module_name, args) => {
                    match self.module_map.lock().await.get(&module_name) {
                        Some(module) => {
                            let response = match module.cli_command(args.into()) {
                                ROk(response) => response.into_string(),
                                RErr(err) => format!("Error:\n{err}"),
                            };
                            let _ = server_response_send.send(Some(response));
                        }
                        None => {
                            let _ = server_response_send.send(Some("module not found".to_string()));
                        }
                    }
                }
                BackendServerCommand::LayoutCliCommand(args) => {
                    let layout = self.layout.clone().unwrap();
                    let response = match layout.lock().await.1.cli_command(RString::from(args)) {
                        ROk(response) => response.into_string(),
                        RErr(err) => format!("Error:\n{err}"),
                    };
                    let _ = server_response_send.send(Some(response));
                }
            }
        }
    }

    pub fn load_css(&mut self, config_dir: &Path) {
        let css_content = grass::from_path(
            config_dir.join("dynisland.scss"),
            &grass::Options::default(),
        );
        match css_content {
            Ok(content) => {
                self.css_provider //TODO maybe save previous state before trying to update
                    .load_from_string(&content);
            }
            Err(err) => {
                log::warn!("failed to parse css: {}", err.to_string());
            }
        }
    }

    fn load_configs(&mut self, config_dir: &Path) {
        self.config = config::get_config(config_dir);
        log::debug!("general_config: {:#?}", self.config.general_style_config);
        for (module_name, module) in self.module_map.blocking_lock().iter_mut() {
            log::info!("loading config for module: {:#?}", module_name);
            let config_to_parse = self.config.module_config.get(module_name);
            let config_parsed = match config_to_parse {
                Some(conf) => {
                    let confs: String = ron::ser::to_string_pretty(&conf, PrettyConfig::default())
                        .unwrap()
                        .into();
                    log::trace!("{module_name} config before strip comments: {}", confs);
                    let mut confs = confs.replace("\\'", "\'");
                    if let Err(err) = json_strip_comments::strip(&mut confs) {
                        log::warn!("failed to strip trailing commas from {module_name} err: {err}");
                    };
                    log::trace!("{module_name} config: {}", confs);
                    module.update_config(confs.into())
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
                    // log::debug!("{}: {:#?}", module_name, config_to_parse);
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
            let activity: Widget = layout
                .1
                .get_activity(&activity)
                .unwrap()
                .try_into()
                .unwrap();
            Self::update_general_configs_on_activity(&self.config.general_style_config, &activity);
        }
    }

    fn update_general_configs_on_activity(config: &GeneralConfig, activity: &Widget) {
        //TODO define property names as constants
        activity.set_property("config-minimal-height", config.minimal_height as i32);
        activity.set_property("config-minimal-width", config.minimal_width as i32);
        activity.set_property("config-blur-radius", config.blur_radius);
        activity.set_property("config-enable-drag-stretch", config.enable_drag_stretch);
        // activity.set_property("config-transition-duration", config.hide_widget_timeout_ms);
        // update widget size
        activity.set_property("mode", activity.property::<ActivityMode>("mode"));
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
            let mut confs: String = ron::ser::to_string_pretty(&config, PrettyConfig::default())
                .unwrap()
                .into();

            if let Err(err) = json_strip_comments::strip(&mut confs) {
                log::warn!("failed to strip trailing commas from {layout_name} err: {err}");
            };
            log::debug!("{layout_name} config: {}", confs);
            match layout.1.update_config(confs.into()) {
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

    pub fn get_default_config(self) -> (Config, String) {
        let mut base_conf = Config::default();

        // get all the loadable LayoutManager configs
        let lm_defs = crate::module_loading::get_lm_definitions(&self.config_dir);
        let mut layout_configs: Vec<(String, RResult<RString, RBoxError>)> = Vec::new();
        for (lm_name, lm_constructor) in lm_defs {
            let built_lm = match lm_constructor(self.application.clone().into()) {
                ROk(x) => x,
                RErr(e) => {
                    log::error!("error during creation of {lm_name}: {e:#?}");
                    continue;
                }
            };
            layout_configs.push((lm_name, built_lm.default_config()));
        }
        layout_configs.push((
            layout_manager::NAME.to_owned(),
            fallback_layout::new(self.application.clone().into())
                .unwrap()
                .default_config(),
        ));

        base_conf.layout = Some(layout_configs.first().unwrap().0.clone());

        // get all the loadable Module configs
        let mod_defs = crate::module_loading::get_module_definitions(&self.config_dir);
        let mut module_configs: Vec<(String, RResult<RString, RBoxError>)> = Vec::new();
        for (mod_name, mod_constructor) in mod_defs {
            match mod_constructor(self.app_send.clone().unwrap()) {
                ROk(built_mod) => {
                    module_configs.push((mod_name, built_mod.default_config()));
                }
                RErr(e) => log::error!("error during creation of {mod_name}: {e:#?}"),
            };
        }
        base_conf.loaded_modules = module_configs.iter().map(|v| v.0.to_owned()).collect();
        let conf_str = "Config".to_owned() + &base_conf.to_string();

        // put the LayoutManager configs into base_conf and a string
        let mut lm_config_str = String::from("{\n");
        for (lm_name, lm_config) in layout_configs {
            match lm_config {
                RErr(err) => log::debug!("cannot get default config: {err}"),
                ROk(lm_config) => {
                    lm_config_str += &format!("\"{lm_name}\": {lm_config},\n")
                        .lines()
                        .map(|l| "        ".to_owned() + l + "\n")
                        .collect::<String>();
                    match ron::de::from_str(lm_config.as_str()) {
                        Err(err) => log::warn!("cannot get default config for {lm_name}: {err}"),
                        Ok(value) => {
                            base_conf.layout_configs.insert(lm_name, value);
                        }
                    }
                }
            }
        }
        lm_config_str += "    },";

        // put all the Module configs into base_conf and a string
        let mut mod_config_str = String::from("{\n");
        for (mod_name, mod_config) in module_configs {
            match mod_config {
                ROk(mod_config) => {
                    log::debug!("string config for {mod_name}: {}", mod_config.as_str());
                    mod_config_str += &format!("\"{mod_name}\": {mod_config},\n")
                        .lines()
                        .map(|l| "        ".to_owned() + l + "\n")
                        .collect::<String>();
                    match ron::de::from_str(mod_config.as_str()) {
                        Err(err) => log::warn!("cannot get default config for {mod_name}: {err}"),
                        Ok(value) => {
                            base_conf.module_config.insert(mod_name, value);
                        }
                    }
                }
                RErr(err) => {
                    log::warn!("cannot get default config for {mod_name}: {err}");
                }
            }
        }
        mod_config_str += "    },";
        let conf_str = conf_str
            .replace(
                "layout_configs: {},",
                &("layout_configs: ".to_owned() + &lm_config_str),
            )
            .replace(
                "module_config: {},",
                &("module_config: ".to_owned() + &mod_config_str),
            );
        // check that the generated config is valid
        let options = ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME);
        if options.from_str::<Config>(&conf_str).is_ok() {
            (base_conf, conf_str)
        } else {
            (
                base_conf.clone(),
                "Config".to_owned() + &base_conf.to_string(),
            )
        }
    }
}

impl Default for App {
    fn default() -> Self {
        // let (hdl, shutdown) = get_new_tokio_rt();
        let flags = gtk::gio::ApplicationFlags::default();
        let app = gtk::Application::new(Some("com.github.cr3eperall.dynisland"), flags);
        App {
            application: app,
            module_map: Rc::new(Mutex::new(HashMap::new())),
            layout: None,
            // producers_handle: hdl,
            // producers_shutdown: shutdown,
            app_send: None,
            config: config::Config::default(),
            css_provider: gtk::CssProvider::new(),
            config_dir: config::get_default_config_path(),
        }
    }
}

fn start_config_dir_watcher(
    server_send: tokio::sync::mpsc::UnboundedSender<BackendServerCommand>,
    config_dir: &Path,
) -> RecommendedWatcher {
    log::info!("starting config watcher");
    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(evt) => {
                // log::info!("config event: {:?}",evt.kind);
                match evt.kind {
                    notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                        log::debug!("Config change detected");
                        server_send
                            .send(BackendServerCommand::ReloadConfig)
                            .expect("Failed to send notification")
                    }
                    notify::EventKind::Create(_) => {
                        // log::info!("file create event");
                    }
                    _ => {}
                }
                // log::debug!("{evt:?}");
            }
            Err(err) => {
                log::error!("Notify watcher error: {err}")
            }
        })
        .expect("Failed to get file watcher");
    if let Err(err) = watcher.watch(&config_dir, notify::RecursiveMode::NonRecursive) {
        log::warn!("Failed to start config file watcher, restart dynisland to get automatic config updates: {err}")
    }
    watcher
}

fn start_ipc_server(
    runtime_path: std::path::PathBuf,
    server_send: tokio::sync::mpsc::UnboundedSender<BackendServerCommand>,
    mut server_response_recv: tokio::sync::mpsc::UnboundedReceiver<Option<String>>,
) {
    let thread = thread::Builder::new().name("ipc-server".to_string());
    thread
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                loop {
                    std::fs::create_dir_all(&runtime_path).expect("invalid runtime path");
                    log::info!(
                        "starting ipc socket at {}",
                        runtime_path.canonicalize().unwrap().to_str().unwrap()
                    );
                    if let Err(err) = open_socket(
                        &runtime_path,
                        server_send.clone(),
                        &mut server_response_recv,
                    )
                    .await
                    {
                        log::error!("socket closed: {err}");
                        if matches!(
                            err.downcast::<std::io::Error>().unwrap().kind(),
                            ErrorKind::AddrInUse
                        ) {
                            log::error!("app was already started");
                            break;
                        }
                    } else {
                        log::info!("kill message received");
                        break;
                    }
                }
            });
        })
        .expect("failed to spawn file-watcher thread");
}
