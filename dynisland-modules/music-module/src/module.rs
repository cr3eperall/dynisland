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
    base_module::{BaseModule, ProducerRuntime},
    cast_dyn_any,
    graphics::activity_widget::boxed_activity_mode::ActivityMode,
};

use crate::{widget, NAME};


#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MusicConfig {
    //allowed_players: cider2, (?cider1, ?spotify...)
    enabled_player_override: Vec<String>,
}

pub struct MusicModule {
    base_module: BaseModule<MusicModule>,
    producers_rt: ProducerRuntime,
    config: MusicConfig,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let base_module=BaseModule::new(NAME, app_send);
    let this = MusicModule {
        base_module,
        producers_rt: ProducerRuntime::new(),
        config: MusicConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
    #[allow(clippy::let_and_return)]
    fn init(&self) {
        let base_module=self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {

            //create activity
            let act = widget::get_activity(base_module.prop_send(), NAME, "music-activity");

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
        self.producers_rt.reset_blocking();
        //restart producers
        for producer in self.base_module.registered_producers().blocking_lock().iter() {
            producer(self);
        }
    }
}


//TODO add reference to module and recieve messages from main
#[allow(unused_variables)]
fn producer(module: &MusicModule) {
    //data producer
    let config = &module.config;
    // let module: &mut MusicModule = cast_dyn_any_mut!(module, MusicModule).unwrap();
    let activities = &module.base_module.registered_activities();
    let mode = activities
        .blocking_lock()
        .get_property_blocking("music-activity", "mode")
        .unwrap();
    // debug!("starting task");
    let config = config.clone();
    module.producers_rt.handle().spawn(async move {
        let prev_mode = *cast_dyn_any!(mode.lock().await.get(), ActivityMode).unwrap();
        if !matches!(prev_mode, ActivityMode::Expanded) {
            mode.lock().await.set(ActivityMode::Expanded).unwrap();
        }
    });
}
