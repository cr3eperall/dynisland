use std::{collections::HashSet, rc::Rc, sync::Arc};

use abi_stable::{
    external_types::crossbeam_channel::RSender,
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError,
        RResult::{self, ROk},
        RString,
    },
};
use anyhow::Context;
use dynisland_abi::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand};
use env_logger::Env;
use gtk::{prelude::*, Widget};
use log::Level;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use dynisland_core::{
    base_module::{ActivityMap, DynamicActivity, Module, Producer, PropertyUpdate},
    cast_dyn_any,
    graphics::activity_widget::boxed_activity_mode::ActivityMode,
};

use crate::{widget, NAME};

/// for now this is just used to test new code
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MusicConfig {
    //allowed_players: cider2, (?cider1, ?spotify...)
    enabled_player_override: Vec<String>,
}

pub struct MusicModule {
    app_send: RSender<UIServerCommand>,
    prop_send: UnboundedSender<PropertyUpdate>,
    registered_activities: Rc<Mutex<ActivityMap>>,
    registered_producers: Arc<Mutex<HashSet<Producer<Self>>>>,
    pub producers_handle: Mutex<Handle>,
    pub producers_shutdown: Mutex<tokio::sync::mpsc::Sender<()>>,
    config: MusicConfig,
}

fn register_activity(
    registered_activities: Rc<Mutex<ActivityMap>>,
    app_send: &RSender<UIServerCommand>,
    activity: DynamicActivity,
) {
    let widget = activity.get_activity_widget();
    let id = activity.get_identifier();
    let activity = Rc::new(Mutex::new(activity));

    app_send
        .send(UIServerCommand::AddActivity(
            id,
            widget.upcast::<Widget>().into(),
        ))
        .unwrap();
    let mut reg = registered_activities.blocking_lock();
    reg.insert_activity(activity)
        .with_context(|| "failed to register activity")
        .unwrap();
}
fn _unregister_activity(
    registered_activities: Rc<Mutex<ActivityMap>>,
    app_send: &UnboundedSender<UIServerCommand>,
    activity_name: &str,
) {
    app_send
        .send(UIServerCommand::RemoveActivity(ActivityIdentifier::new(
            NAME,
            activity_name,
        )))
        .unwrap();

    registered_activities
        .blocking_lock()
        .map
        .remove(activity_name)
        .expect("activity isn't registered");
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

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let registered_activities = Rc::new(Mutex::new(ActivityMap::new()));
    let prop_send = MusicModule::spawn_property_update_loop(&registered_activities);
    let (hdl, shutdown) = get_new_tokio_rt();
    let this = MusicModule {
        // name: "ExampleModule".to_string(),
        app_send,
        prop_send,
        registered_activities,
        registered_producers: Arc::new(Mutex::new(HashSet::new())),
        producers_handle: Mutex::new(hdl),
        producers_shutdown: Mutex::new(shutdown),
        config: MusicConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
    #[allow(clippy::let_and_return)]
    fn init(&self) {
        let app_send = self.app_send.clone();
        let prop_send = self.prop_send.clone();
        let registered_activities = self.registered_activities.clone();
        let registered_producers = self.registered_producers.clone();
        glib::MainContext::default().spawn_local(async move {
            //FIXME this assumes that init is called from the main thread, now it is but it may change

            //create activity
            let act = widget::get_activity(prop_send, NAME, "music-activity");

            //register activity and data producer
            register_activity(registered_activities, &app_send, act);
            Self::register_producer(registered_producers, producer);
        });
    }

    #[allow(clippy::let_and_return)]
    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();

        self.config = conf
            .into_rust()
            .with_context(|| "failed to parse config to struct")
            .unwrap();
        ROk(())
    }

    #[allow(clippy::let_and_return)]
    fn restart_producers(&self) {
        self.restart_producer_rt();
    }
}

impl Module for MusicModule {
    // fn new(app_send: UnboundedSender<UIServerCommand> ) -> Box<dyn Module> {
    //     let registered_activities = Rc::new(Mutex::new(ActivityMap::new()));

    //     let prop_send = MusicModule::spawn_property_update_loop(&registered_activities);
    //     let (hdl, shutdown) = get_new_tokio_rt();
    //     Box::new(Self {
    //         app_send,
    //         prop_send,
    //         registered_activities,
    //         registered_producers: Arc::new(Mutex::new(HashSet::new())),
    //         producers_handle: hdl,
    //         producers_shutdown: shutdown,
    //         config: MusicConfig::default(),
    //     })
    // }

    // fn restart_producers(&mut self) {
    //     self.restart_producer_rt();
    // }

    // fn init(&self) {
    //     let app_send = self.app_send.clone();
    //     let prop_send = self.prop_send.clone();
    //     let registered_activities = self.registered_activities.clone();
    //     // glib::MainContext::default().spawn_local(async move {
    //         //create activity
    //         let activity = Self::get_activity(
    //             prop_send,
    //             NAME,
    //             "music-activity",
    //         );

    //         //register activity and data producer
    //         register_activity(registered_activities, &app_send, activity);
    //         self.register_producer(Self::producer);
    //     // });
    // }

    // fn update_config(&mut self, config: Value) -> Result<()> {
    //     self.config = config
    //         .into_rust()
    //         .with_context(|| "failed to parse config")
    //         .unwrap();
    //     Ok(())
    // }
}
//TODO move to base_module
impl MusicModule {
    fn register_producer(
        registered_producers: Arc<Mutex<HashSet<Producer<Self>>>>,
        producer: Producer<Self>,
    ) {
        registered_producers.blocking_lock().insert(producer);
    }

    fn restart_producer_rt(&self) {
        let mut producers_shutdown = self.producers_shutdown.blocking_lock();
        producers_shutdown
            .blocking_send(())
            .expect("failed to shutdown old producer runtime"); //stop current producers_runtime
        let (handle, shutdown) = get_new_tokio_rt(); //start new producers_runtime

        *self.producers_handle.blocking_lock() = handle.clone();
        *producers_shutdown = shutdown;
        //restart producers
        for producer in self.get_registered_producers().blocking_lock().iter() {
            producer(self, &handle, self.app_send.clone())
        }
    }

    fn get_registered_producers(&self) -> Arc<Mutex<HashSet<Producer<Self>>>> {
        self.registered_producers.clone()
    }
}

//TODO add reference to module and recieve messages from main
#[allow(unused_variables)]
fn producer(module: &MusicModule, rt: &Handle, _app_send: RSender<UIServerCommand>) {
    //data producer
    let config = &module.config;
    // let module: &mut MusicModule = cast_dyn_any_mut!(module, MusicModule).unwrap();
    let activities = &module.registered_activities;
    let mode = activities
        .blocking_lock()
        .get_property_blocking("music-activity", "mode")
        .unwrap();
    // debug!("starting task");
    let config = config.clone();
    rt.spawn(async move {
        let prev_mode = *cast_dyn_any!(mode.lock().await.get(), ActivityMode).unwrap();
        if !matches!(prev_mode, ActivityMode::Expanded) {
            mode.lock().await.set(ActivityMode::Expanded).unwrap();
        }
    });
}
