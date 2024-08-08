use abi_stable::external_types::crossbeam_channel::RSender;
use std::{collections::HashMap, path::PathBuf};

use abi_stable::{
    library::{lib_header_from_path, LibraryError},
    std_types::{RBoxError, RResult},
    type_layout::TypeLayout,
    StableAbi,
};
use dynisland_abi::{
    layout::{LayoutManagerBuilderRef, LayoutManagerType},
    module::{ModuleBuilderRef, ModuleType, UIServerCommand},
    SabiApplication,
};

pub fn get_module_definitions(
) -> HashMap<String, extern "C" fn(RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>> {
    let mut module_def_map = HashMap::<
        String,
        extern "C" fn(RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>,
    >::new();

    let module_path = {
        #[cfg(debug_assertions)]
        {
            // TODO don't use hardcoded value
            PathBuf::from("/home/david/dev/rust/dynisland/dynisland-core/target/debug/")
        }

        #[cfg(not(debug_assertions))]
        {
            config::get_config_path().join("modules")
        }
    };

    let files = std::fs::read_dir(module_path).unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if !path.is_file() {
            continue;
        }
        match file
            .file_name()
            .to_str()
            .unwrap()
            .to_lowercase()
            .strip_suffix(".so")
        {
            Some(name) => {
                if !name.ends_with("module") {
                    continue;
                }
            }
            None => continue,
        }
        log::debug!("loading module file: {:#?}", path);

        let res = (|| {
            let header = lib_header_from_path(&path)?;
            // header.init_root_module::<ModuleBuilderRef>()
            let layout1 = ModuleBuilderRef::LAYOUT;
            let layout2 = header.layout().unwrap();
            ensure_compatibility(layout1, layout2).and_then(|_| unsafe {
                header
                    .unchecked_layout::<ModuleBuilderRef>()
                    .map_err(|err| err.into_library_error::<ModuleBuilderRef>())
            })
        })();

        let module_builder = match res {
            Ok(x) => x,
            Err(e) => {
                log::error!(
                    "error while loading {}: {e:#?}",
                    path.file_name().unwrap().to_str().unwrap()
                );
                continue;
            }
        };
        let name = module_builder.name();
        let constructor = module_builder.new();

        module_def_map.insert(name.into(), constructor);
    }
    module_def_map
}

pub fn get_lm_definitions() -> HashMap<
    String,
    extern "C" fn(SabiApplication) -> RResult<LayoutManagerType, abi_stable::std_types::RBoxError>,
> {
    let mut lm_def_map = HashMap::<
        String,
        extern "C" fn(SabiApplication) -> RResult<LayoutManagerType, RBoxError>,
    >::new();

    let lm_path = {
        #[cfg(debug_assertions)]
        {
            // TODO don't use hardcoded value
            PathBuf::from("/home/david/dev/rust/dynisland/dynisland-core/target/debug/")
        }
        #[cfg(not(debug_assertions))]
        {
            config::get_config_path().join("layouts")
        }
    };

    let files = std::fs::read_dir(lm_path).unwrap();
    for file in files {
        let file = file.unwrap();
        let path = file.path();
        if !path.is_file() {
            continue;
        }
        match file
            .file_name()
            .to_str()
            .unwrap()
            .to_lowercase()
            .strip_suffix(".so")
        {
            Some(name) => {
                if !name.ends_with("layoutmanager") {
                    continue;
                }
            }
            None => continue,
        }
        log::debug!("loading layout manager file: {:#?}", path);

        let res = (|| {
            let header = lib_header_from_path(&path)?;
            // header.init_root_module::<ModuleBuilderRef>()
            let layout1 = LayoutManagerBuilderRef::LAYOUT;
            let layout2 = header.layout().unwrap();
            ensure_compatibility(layout1, layout2).and_then(|_| unsafe {
                header
                    .unchecked_layout::<LayoutManagerBuilderRef>()
                    .map_err(|err| err.into_library_error::<LayoutManagerBuilderRef>())
            })
        })();

        let lm_builder = match res {
            Ok(x) => x,
            Err(e) => {
                log::error!(
                    "error while loading {}: {e:#?}",
                    path.file_name().unwrap().to_str().unwrap()
                );
                continue;
            }
        };
        let name = lm_builder.name();
        let constructor = lm_builder.new();

        lm_def_map.insert(name.into(), constructor);
    }
    lm_def_map
}

pub fn ensure_compatibility(
    interface: &'static TypeLayout,
    implementation: &'static TypeLayout,
) -> Result<(), abi_stable::library::LibraryError> {
    let compatibility = abi_stable::abi_stability::abi_checking::check_layout_compatibility(
        interface,
        implementation,
    );
    if let Err(err) = compatibility {
        let incompatibilities = err.errors.iter().filter(|e| !e.errs.is_empty());
        let fatal_incompatibilities = incompatibilities.filter(|err| {
            err.errs.iter().any(|err| {
                !matches!(
                    err,
                    abi_stable::abi_stability::abi_checking::AbiInstability::FieldCountMismatch(assert) if assert.expected > assert.found
                )
            })
        });
        if fatal_incompatibilities.count() > 0 {
            return Err(LibraryError::AbiInstability(RBoxError::new(err)));
        }
    }
    Ok(())
}
