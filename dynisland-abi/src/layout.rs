use abi_stable::{
    declare_root_module_statics,
    library::RootModule,
    package_version_strings, sabi_trait,
    sabi_types::VersionStrings,
    std_types::{RBox, RBoxError, ROption, RResult, RStr, RString, RVec},
    StableAbi,
};

use crate::{module::ActivityIdentifier, SabiApplication, SabiWidget};

pub type LayoutManagerType = SabiLayoutManager_TO<'static, RBox<()>>;

#[sabi_trait]
pub trait SabiLayoutManager {
    fn init(&mut self);
    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError>;

    fn add_activity(&mut self, activity_id: &ActivityIdentifier, widget: SabiWidget);
    fn remove_activity(&mut self, activity: &ActivityIdentifier);
    fn list_activities(&self) -> RVec<&ActivityIdentifier>;
    #[sabi(last_prefix_field)]
    fn get_activity(&self, activity: &ActivityIdentifier) -> ROption<SabiWidget>;
}

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = LayoutManagerBuilderRef)))]
#[sabi(missing_field(panic))]
pub struct LayoutManagerBuilder {
    pub new: extern "C" fn(SabiApplication) -> RResult<LayoutManagerType, RBoxError>,

    #[sabi(last_prefix_field)]
    pub name: RStr<'static>,
}

impl RootModule for LayoutManagerBuilderRef {
    declare_root_module_statics! {LayoutManagerBuilderRef}
    const BASE_NAME: &'static str = "layout_manager";
    const NAME: &'static str = "layout_manager";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}
