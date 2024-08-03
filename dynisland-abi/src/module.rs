use abi_stable::{
    declare_root_module_statics,
    external_types::crossbeam_channel::RSender,
    library::RootModule,
    package_version_strings, sabi_trait,
    sabi_types::VersionStrings,
    std_types::{RBox, RBoxError, RResult, RStr, RString},
    StableAbi,
};

use crate::SabiWidget;

pub type ModuleType = SabiModule_TO<'static, RBox<()>>;

#[sabi_trait]
pub trait SabiModule {
    /// Register activities and producers that should appear when dynisland starts
    /// When this function is called the config was already loaded from the config file
    /// Functions using the gtk api should be run inside `glib::MainContext::default().spawn_local()` because gtk has yet to be initialized
    ///
    /// # Examples
    /// ```
    /// fn init(&self) {
    ///     let base_module = self.base_module.clone();
    ///     let config = self.config.clone();
    ///     glib::MainContext::default().spawn_local(async move {
    ///         if config.example_value==42{
    ///             //create activity
    ///             let act: DynamicActivity = /* ... */;
    ///             //register activity and data producer
    ///             base_module.register_activity(act).unwrap();
    ///         }
    ///         base_module.register_producer(self::producer);
    ///     });
    /// }
    /// ```
    fn init(&self);

    /// Update the config struct from the section of the config file for this module
    ///
    /// # Examples
    /// ```
    /// #[derive(Serialize, Deserialize, Clone)]
    /// #[serde(default)]
    /// pub struct ModuleConfig{
    ///     example_value: i32,
    /// }
    ///
    /// impl Default for ModuleConfig{
    ///     fn default() -> Self {
    ///         Self { example_value: 42 }
    ///     }
    /// }
    ///
    /// fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
    ///     let conf = ron::from_str::<ron::Value>(&config)
    ///         .with_context(|| "failed to parse config to value")
    ///         .unwrap();
    ///     let old_config = self.config.clone();
    ///     self.config = conf
    ///         .into_rust()
    ///         .unwrap_or_else(|err| {
    ///             log::error!("parsing error, using old config: {:#?}", err);
    ///             old_config
    ///         }
    ///     );
    ///     ROk(())
    /// }
    /// ```
    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError>;

    /// Restart the producers registered on the BaseModule
    ///
    /// # Examples
    /// ```
    /// impl SabiModule for Module {
    ///     fn restart_producers(&self) {
    ///         self.producers_rt.shutdown_blocking();
    ///         self.producers_rt.reset_blocking();
    ///         for producer in self
    ///             .base_module
    ///             .registered_producers()
    ///             .blocking_lock()
    ///             .iter()
    ///         {
    ///             producer(self);
    ///         }
    ///     }
    /// }
    /// ```
    #[sabi(last_prefix_field)]
    fn restart_producers(&self);
}

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = ModuleBuilderRef)))]
#[sabi(missing_field(panic))]
pub struct ModuleBuilder {
    /// Create a new instance of a module
    ///
    /// # Examples
    /// ```
    /// pub struct Module{
    ///     base_module: BaseModule<MusicModule>,
    ///     producers_rt: ProducerRuntime,
    ///     config: ModuleConfig,
    /// }
    /// impl SabiModule for Module{/* ... */}
    ///
    /// #[sabi_extern_fn]
    /// pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    ///     let base_module = BaseModule::new(NAME, app_send.clone());
    ///     let producers_rt = ProducerRuntime::new();
    ///     let module = Module{
    ///         base_module,
    ///         producers_rt,
    ///         config: ModuleConfig::default(),
    ///     };
    ///     ROk(SabiModule_TO::from_value(module, TD_CanDowncast))
    /// }
    /// ```
    pub new: extern "C" fn(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>,

    /// The name of the module
    #[sabi(last_prefix_field)]
    pub name: RStr<'static>,
}

impl RootModule for ModuleBuilderRef {
    declare_root_module_statics! {ModuleBuilderRef}
    const BASE_NAME: &'static str = "module";
    const NAME: &'static str = "module";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

/// A command from a module to the app thread
#[repr(C)]
#[derive(StableAbi)]
pub enum UIServerCommand {
    AddActivity(ActivityIdentifier, SabiWidget),
    // AddProducer(RString, Producer),
    RemoveActivity(ActivityIdentifier), //TODO needs to be tested
    RestartProducers(RString),
}

/// Module and activity name, used to uniquely identify a dynamic activity
#[repr(C)]
#[derive(StableAbi, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActivityIdentifier {
    pub(crate) module: RString,
    #[sabi(last_prefix_field)]
    pub(crate) activity: RString,
}
