use dynisland_core::base_module::{Module, ModuleDefinition, MODULES};
use linkme::distributed_slice;

use self::module::ExampleModule;

pub mod module;


pub const NAME: &str = "ExampleModule";

//add to modules to be loaded
#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: ModuleDefinition = (NAME, ExampleModule::new);