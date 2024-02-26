use abi_stable::{declare_root_module_statics, external_types::crossbeam_channel::RSender, library::RootModule, package_version_strings, sabi_trait, sabi_types::VersionStrings, std_types::{RBox, RBoxError, RResult, RString}, StableAbi};
use glib::{object::Cast, translate::{FromGlibPtrNone, ToGlibPtr}};

use crate::graphics::activity_widget::ActivityWidget;


pub type ModuleType = SabiModule_TO<'static, RBox<()>>;

#[sabi_trait]
pub trait SabiModule{
    fn init(&self);

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError>;

    fn restart_producers(&self);

}
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = ModuleBuilderRef)))]
#[sabi(missing_field(panic))]
pub struct ModuleBuilder {
    #[sabi(last_prefix_field)]
    pub new: extern "C" fn(RSender<UIServerCommand>)-> RResult<ModuleType, RBoxError>
}

impl RootModule for ModuleBuilderRef {
    declare_root_module_statics! {ModuleBuilderRef}
    const BASE_NAME: &'static str = "module";
    const NAME: &'static str = "module";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

#[sabi_trait]
pub trait Application {

}

#[repr(C)]
#[derive(StableAbi)]
pub enum UIServerCommand {
    AddActivity(ActivityIdentifier, SabiActivityWidget),
    // AddProducer(RString, Producer),
    RemoveActivity(ActivityIdentifier), //TODO needs to be tested
}


#[repr(C)]
#[derive(StableAbi, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActivityIdentifier {
    pub(crate) module: RString,
    pub(crate) activity: RString,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct SabiActivityWidget{ //FIXME check if lifetimes are actually needed
    pub widget_ref: *mut core::ffi::c_void,
}

impl From<ActivityWidget> for SabiActivityWidget{
    fn from(widget: ActivityWidget) -> Self {

        let widget_ptr: *mut gtk::ffi::GtkWidget=(widget.upcast::<gtk::Widget>()).to_glib_none().0;
        Self{
            widget_ref: widget_ptr as *mut core::ffi::c_void,
        }
    }
}

impl TryInto<ActivityWidget> for SabiActivityWidget{
    type Error=();
    fn try_into(self) -> Result<ActivityWidget, Self::Error> {
        unsafe{
            let widget: *mut gtk::ffi::GtkWidget=self.widget_ref as *mut gtk::ffi::GtkWidget;
            Ok(gtk::Widget::from_glib_none(widget).downcast().unwrap())
        }
    }
}