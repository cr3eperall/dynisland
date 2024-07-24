use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait};
use dynisland_abi::module::{ModuleBuilder, ModuleBuilderRef};

pub mod module;
pub mod player_info;
pub mod visualizer;
pub mod widget;
pub mod utils;

use module::new;

pub const NAME: &str = "MusicModule";

#[export_root_module]
fn instantiate_root_module() -> ModuleBuilderRef {
    ModuleBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
