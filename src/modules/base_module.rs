use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use linkme::distributed_slice;
use ron::Value;
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use crate::{app::UIServerCommand, widgets::activity_widget::ActivityWidget};

#[distributed_slice]
pub static MODULES: [fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module>];

pub type ActivityMap = Arc<Mutex<HashMap<String, Arc<Mutex<DynamicActivity>>>>>;
pub type Producer = fn(
    activities: ActivityMap,
    rt: &Handle,
    app_send: UnboundedSender<UIServerCommand>,
    config: &dyn ModuleConfig,
);

pub trait ModuleConfig: Any + Debug {}

#[async_trait(?Send)]
pub trait Module {
    fn new(app_send: UnboundedSender<UIServerCommand>, config: Option<Value>) -> Box<dyn Module>
    where
        Self: Sized;

    fn get_name(&self) -> &str;

    fn get_config(&self) -> &dyn ModuleConfig;

    fn get_registered_activities(&self) -> ActivityMap;
    async fn register_activity(&self, activity: Arc<Mutex<DynamicActivity>>);
    async fn unregister_activity(&self, activity: &str);

    fn get_registered_producers(&self) -> Arc<Mutex<HashSet<Producer>>>;
    async fn register_producer(&self, producer: Producer);

    fn init(&self);
    fn parse_config(&mut self, config: Value) -> Result<()>;
}

pub struct SubscribableProperty {
    pub property: Arc<Mutex<DynamicProperty>>,
    pub subscribers: Vec<Box<dyn ValidDynamicClosure>>,
}

pub struct DynamicActivity {
    //TODO change to getters and setters
    pub(crate) widget: ActivityWidget,
    pub(crate) property_dictionary: HashMap<String, SubscribableProperty>,
    pub(crate) ui_send: UnboundedSender<PropertyUpdate>,
    pub(crate) identifier: String,
}

pub struct PropertyUpdate(pub String, pub Box<dyn ValidDynType>);

pub trait ValidDynType: Any + Sync + Send + DynClone {}
impl<T: Any + Sync + Send + Clone> ValidDynType for T {}

pub trait ValidDynamicClosure: Fn(&dyn ValidDynType) + DynClone {}
impl<T: Fn(&dyn ValidDynType) + DynClone + Clone> ValidDynamicClosure for T {}

pub struct DynamicProperty {
    pub backend_channel: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    pub name: String,
    pub value: Box<dyn ValidDynType>,
}
