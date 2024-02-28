use std::{collections::HashSet, rc::Rc, sync::Arc, vec};

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
use glib::{self, object::Cast};
use gtk::Widget;
use log::Level;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use dynisland_core::{
    base_module::{ActivityMap, DynamicActivity, Module, Producer, PropertyUpdate},
    graphics::activity_widget::boxed_activity_mode::ActivityMode,
};
//FIXME fix logging

use super::{widget, NAME};

/// for now this is just used to test new code
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExampleConfig {
    #[serde(default)]
    pub int: i32,
    #[serde(default)]
    pub string: String,
    #[serde(default)]
    pub vec: Vec<String>,

    pub duration: u64,
}

// impl ModuleConfig for ExampleConfig {}
impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
        }
    }
}
pub struct ExampleModule {
    // name: String,
    app_send: RSender<UIServerCommand>,
    prop_send: UnboundedSender<PropertyUpdate>,
    pub registered_activities: Rc<Mutex<ActivityMap>>,
    registered_producers: Arc<Mutex<HashSet<Producer<Self>>>>,
    pub producers_handle: Mutex<Handle>,
    pub producers_shutdown: Mutex<tokio::sync::mpsc::Sender<()>>,
    config: ExampleConfig,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let registered_activities = Rc::new(Mutex::new(ActivityMap::new()));
    let prop_send = ExampleModule::spawn_property_update_loop(&registered_activities);
    let (hdl, shutdown) = get_new_tokio_rt();
    let this = ExampleModule {
        // name: "ExampleModule".to_string(),
        app_send,
        prop_send,
        registered_activities,
        registered_producers: Arc::new(Mutex::new(HashSet::new())),
        producers_handle: Mutex::new(hdl),
        producers_shutdown: Mutex::new(shutdown),
        config: ExampleConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ExampleModule {
    #[allow(clippy::let_and_return)]
    fn init(&self) {
        let app_send = self.app_send.clone();
        let prop_send = self.prop_send.clone();
        let registered_activities = self.registered_activities.clone();
        let registered_producers = self.registered_producers.clone();
        glib::MainContext::default().spawn_local(async move {
            //FIXME this assumes that init is called from the main thread, now it is but it may change

            //create activity
            let act = widget::get_activity(prop_send, NAME, "exampleActivity1");

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

impl Module for ExampleModule {
    // fn new(app_send: UnboundedSender<UIServerCommand> ) -> Box<dyn Module> {

    // }

    // fn restart_producers(&mut self) {
    //     self.restart_producer_rt();
    // }

    // fn update_config(&mut self, config: Value) -> Result<()> {
    //     self.config = config
    //         .into_rust()
    //         .with_context(|| "failed to parse config")
    //         .unwrap();
    //     Ok(())
    // }

    // fn init(&self) {
    //     let app_send = self.app_send.clone();
    //     let prop_send = self.prop_send.clone();
    //     let registered_activities = self.registered_activities.clone();
    //     // glib::MainContext::default().spawn_local(async move { //FIXME this assumes that init is called from the main thread, now it is but it may change

    //         //create activity
    //         let act = widget::get_activity(
    //             prop_send,
    //             NAME,
    //             "exampleActivity1",
    //         );

    //         //register activity and data producer
    //         register_activity(registered_activities, &app_send, act);
    //     // });
    //     self.register_producer(producer);
    // }
}

impl ExampleModule {
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
fn producer(module: &ExampleModule, rt: &Handle, _app_send: RSender<UIServerCommand>) {
    // let module = cast_dyn_any!(module, ExampleModule).unwrap();
    //data producer
    let config: &ExampleConfig = &module.config;

    //TODO shouldn't be blocking locks, maybe execute async with glib::MainContext
    let act = module.registered_activities.blocking_lock();
    let mode = act
        .get_property_blocking("exampleActivity1", "mode")
        .unwrap();
    let label = act
        .get_property_blocking("exampleActivity1", "comp-label")
        .unwrap();
    let scrolling_text = act
        .get_property_blocking("exampleActivity1", "scrolling-label-text")
        .unwrap();
    let rolling_char = act
        .get_property_blocking("exampleActivity1", "rolling-char")
        .unwrap();
    // label.blocking_lock().set(config.string.clone()).unwrap();

    // let activity = Rc::new(Mutex::new(Self::get_activity(
    //     _prop_send.clone(),
    //     "exampleActivity2",
    // )));
    // _app_send
    //     .send(UIServerCommand::AddActivity(
    //         "ExampleModule".to_string(),
    //         activity,
    //     ))
    //     .unwrap();

    let config = config.clone();
    // debug!("starting task");
    rt.spawn(async move {
        // debug!("task started");
        mode.lock().await.set(ActivityMode::Minimal).unwrap();
        loop {
            rolling_char.lock().await.set('0').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('1').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('2').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('3').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('4').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;

            scrolling_text
                .lock()
                .await
                .set(
                    "Hello long text, very long text. Hello long text, very long text.    end"
                        .to_string(),
                )
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(12000)).await;

            scrolling_text
                .lock()
                .await
                .set("Hello shorterer e e e e text e.    end".to_string())
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
            // mode.lock().await.set(ActivityMode::Minimal).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

            // mode.lock().await.set(ActivityMode::Compact).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
            // let old_label_val;
            // {
            //     let label_val = label.lock().await;
            //     let str_val: &String = cast_dyn_any!(label_val.get(), String).unwrap();
            //     old_label_val = str_val.clone();
            // }

            // tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            // label.lock().await.set("sdkjvksdv1 tryt etvcbssrfh".to_string()).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
            // label.lock().await.set("fghn".to_string()).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;

            // label.lock().await.set(old_label_val).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;

            // prop_send
            //     .send(PropertyUpdate {
            //         activity_id: "*".to_string(),
            //         property_name: "mode".to_string(),
            //         value: Box::new(ActivityMode::Compact),
            //     })
            //     .unwrap();
            // mode.lock().await.set(ActivityMode::Expanded).unwrap();

            // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            // prop_send
            //     .send(PropertyUpdate {
            //         activity_id: "*".to_string(),
            //         property_name: "mode".to_string(),
            //         value: Box::new(ActivityMode::Expanded),
            //     })
            //     .unwrap();
            // mode.lock().await.set(ActivityMode::Compact).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            // mode.lock().await.set(ActivityMode::Expanded).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            // mode.lock().await.set(ActivityMode::Overlay).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            // mode.lock().await.set(ActivityMode::Expanded).unwrap();
            // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        }
    });
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
