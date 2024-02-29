use std::vec;

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
use dynisland_abi::{ModuleType, SabiModule, SabiModule_TO, UIServerCommand};
use env_logger::Env;
use log::Level;
use serde::{Deserialize, Serialize};

use dynisland_core::{
    base_module::{ProducerRuntime, BaseModule},
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
    #[serde(default)]
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
    base_module: BaseModule<ExampleModule>,
    producers_rt: ProducerRuntime,
    config: ExampleConfig,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let base_module= BaseModule::new(NAME, app_send);
    let this = ExampleModule {
        // name: "ExampleModule".to_string(),
        base_module,
        producers_rt: ProducerRuntime::new(),
        config: ExampleConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ExampleModule {
    #[allow(clippy::let_and_return)]
    fn init(&self) {
        let base_module= self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            //create activity
            let act = widget::get_activity(base_module.prop_send(), NAME, "exampleActivity1");

            //register activity and data producer
            base_module.register_activity(act).unwrap();
            base_module.register_producer(producer);
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

impl ExampleModule {
    fn restart_producer_rt(&self) {
        self.producers_rt.reset_blocking();
        //restart producers
        for producer in self.base_module.registered_producers().blocking_lock().iter() {
            producer(self)
        }
    }
}

//TODO add reference to module and recieve messages from main
#[allow(unused_variables)]
fn producer(module: &ExampleModule) {
    // let module = cast_dyn_any!(module, ExampleModule).unwrap();
    //data producer
    let config: &ExampleConfig = &module.config;

    //TODO shouldn't be blocking locks, maybe execute async with glib::MainContext
    let registered_activities = module.base_module.registered_activities();
    let registered_activities_lock = registered_activities.blocking_lock();
    let mode = registered_activities_lock
        .get_property_blocking("exampleActivity1", "mode")
        .unwrap();
    let label = registered_activities_lock
        .get_property_blocking("exampleActivity1", "comp-label")
        .unwrap();
    let scrolling_text = registered_activities_lock
        .get_property_blocking("exampleActivity1", "scrolling-label-text")
        .unwrap();
    let rolling_char = registered_activities_lock
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
    module.producers_rt.handle().spawn(async move {
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
