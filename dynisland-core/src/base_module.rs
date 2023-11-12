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

use crate::widgets::activity_widget::ActivityWidget;

/// Slice of loaded modules
#[distributed_slice]
pub static MODULES: [fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module>];

pub enum UIServerCommand {
    //TODO change to APIServerCommand
    AddActivity(String, Arc<Mutex<DynamicActivity>>),
    AddProducer(String, Producer),
    RemoveActivity(String, String), //TODO needs to be tested
}

/// This type stores all the registered activities for a module with their name
pub type ActivityMap = Arc<Mutex<HashMap<String, Arc<Mutex<DynamicActivity>>>>>;

/// This is a function that can be registered by the module on the backend.
/// 
/// It's used to:
/// - set `DynamicProperty` values registerted on the activities
/// - register/unregister activities using the `app_send` channel
/// - register other producers
/// 
/// when some kind of event occours.
/// 
/// You should use `rt` to spawn async tasks and return as soon as possible
/// 
/// Every time the configuration file changes, the task running in `rt` is killed and this function is re-executed with a new runtime
pub type Producer = fn(
    activities: ActivityMap,
    rt: &Handle,
    app_send: UnboundedSender<UIServerCommand>,
    prop_send: UnboundedSender<PropertyUpdate>,
    config: &dyn ModuleConfig,
);

/// This trait must be implemented by the module configuration struct
/// 
/// This will be used by [ron] to create a [ron::Value] object from the configuration file, that will be parsed using [Module::parse_config]
pub trait ModuleConfig: Any + Debug {}

/// This trait must be implemented by the main module struct
/// 
/// It should contain these fields:
/// ```ignore
/// app_send: UnboundedSender<UIServerCommand>,
/// prop_send: UnboundedSender<PropertyUpdate>,
/// registered_activities: ActivityMap,
/// registered_producers: Arc<Mutex<HashSet<Producer>>>,
/// config: ModuleConfig,
/// ```
/// 
/// # Examples
/// it can be loaded using this snippet
/// ```ignore
/// use crate::modules::base_module::MODULES;
/// use linkme::distributed_slice;
/// 
/// #[distributed_slice(MODULES)]
/// static SOMETHING: fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module> = ModuleName::new;
/// ```
#[async_trait(?Send)]
pub trait Module {
    /// Creates a new instance of the plugin
    /// 
    /// This is called once at the beginning of the execution.
    /// 
    /// if `config` is [None], a default value should be used
    fn new(app_send: UnboundedSender<UIServerCommand>, config: Option<Value>) -> Box<dyn Module>
    where Self: Sized;

    /// gets the name of the Module
    fn get_name(&self) -> &str;

    /// gets the current config struct, used when starting registered `Producer`s 
    fn get_config(&self) -> &dyn ModuleConfig;
    
    /// gets the registered activities for this Module
    fn get_registered_activities(&self) -> ActivityMap;

    /// This is called after `UIServerCommand::AddActivity` if the activity was registered successfully
    /// 
    /// It should put `activity` inside `self.registered_activities`
    async fn register_activity(&self, activity: Arc<Mutex<DynamicActivity>>);

    /// This is called after `UIServerCommand::RemoveActivity` if the activity was removed successfully
    /// 
    /// It should remove the activity with this name from `self.registered_activities`
    async fn unregister_activity(&self, activity: &str);

    /// gets the registered producers for this Module
    fn get_registered_producers(&self) -> Arc<Mutex<HashSet<Producer>>>;

    /// This is called after `UIServerCommand::AddProducer` if the producer was registered successfully
    /// 
    /// It should put `producer` inside `self.registered_producers`
    async fn register_producer(&self, producer: Producer);

    /// gets the channel used to update `DynamicProperty`s
    fn get_prop_send(&self)-> UnboundedSender<PropertyUpdate>;

    /// This is the module initialization function
    /// 
    /// It should:
    /// - register the first producers and activities
    /// - spawn the dynymic property 
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
    // pub(crate) identifier: String,
}

pub struct PropertyUpdate{
    pub activity_id: String,
    pub property_name: String, 
    pub value: Box<dyn ValidDynType>
}

pub trait ValidDynType: Any + Sync + Send + DynClone {}
impl<T: Any + Sync + Send + Clone> ValidDynType for T {}

pub trait ValidDynamicClosure: Fn(&dyn ValidDynType) + DynClone {}
impl<T: Fn(&dyn ValidDynType) + DynClone + Clone> ValidDynamicClosure for T {}

pub struct DynamicProperty {
    pub backend_channel: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    pub activity_id: String,
    pub property_name: String,
    pub value: Box<dyn ValidDynType>,
}
