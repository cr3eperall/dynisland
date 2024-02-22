use dynisland_core::base_module::{Module, ModuleDefinition, MODULES};
use linkme::distributed_slice;

use crate::music::module::MusicModule;

pub mod module;
pub mod player_info;
pub mod visualizer;

//add to modules to be loaded
#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: ModuleDefinition = (module::NAME, MusicModule::new);
