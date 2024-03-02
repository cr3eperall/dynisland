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
    fn init(&self);

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError>;

    fn restart_producers(&self);
}

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = ModuleBuilderRef)))]
#[sabi(missing_field(panic))]
pub struct ModuleBuilder {
    pub new: extern "C" fn(RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>,

    #[sabi(last_prefix_field)]
    pub name: RStr<'static>,
}

impl RootModule for ModuleBuilderRef {
    declare_root_module_statics! {ModuleBuilderRef}
    const BASE_NAME: &'static str = "module";
    const NAME: &'static str = "module";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

#[repr(C)]
#[derive(StableAbi)]
pub enum UIServerCommand {
    AddActivity(ActivityIdentifier, SabiWidget),
    // AddProducer(RString, Producer),
    RemoveActivity(ActivityIdentifier), //TODO needs to be tested
}

#[repr(C)]
#[derive(StableAbi, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActivityIdentifier {
    pub(crate) module: RString,
    pub(crate) activity: RString,
}
