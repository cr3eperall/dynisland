use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use ron::Value;
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use crate::{app::UIServerCommand, widgets::dynamic_activity::DynamicActivity};

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
    fn new(app_send: UnboundedSender<UIServerCommand>, config: Option<Value>) -> Self
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
