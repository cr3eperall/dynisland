use dynisland_core::base_module::{Module, UIServerCommand, MODULES};
use linkme::distributed_slice;
use ron::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::music::module::MusicModule;

pub mod module;
pub mod player_info;
pub mod visualizer;

//add to modules to be loaded
#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module> =
    MusicModule::new;
