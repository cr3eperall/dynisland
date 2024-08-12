use abi_stable::{
    external_types::crossbeam_channel::RSender,
    std_types::RResult::{RErr, ROk},
};
use std::{collections::HashMap, path::PathBuf, rc::Rc};
use tokio::sync::Mutex;

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

use crate::{app::App, layout_manager::simple_layout};

impl App {
    pub(crate) fn load_modules(&mut self) -> Vec<String> {
        let mut module_order = vec![];
        let module_def_map = crate::module_loading::get_module_definitions();

        if self.config.loaded_modules.contains(&"all".to_string()) {
            //load all modules available in order of hash (random order)
            for module_def in module_def_map {
                let module_name = module_def.0;
                let module_constructor = module_def.1;

                let built_module = match module_constructor(self.app_send.clone().unwrap()) {
                    ROk(x) => x,
                    RErr(e) => {
                        log::error!("error during creation of {module_name}: {e:#?}");
                        continue;
                    }
                };

                module_order.push(module_name.to_string());
                self.module_map
                    .blocking_lock()
                    .insert(module_name.to_string(), built_module);
            }
        } else {
            //load only modules in the config in order of definition
            for module_name in self.config.loaded_modules.iter() {
                let module_constructor = module_def_map.get(module_name);
                let module_constructor = match module_constructor {
                    None => {
                        log::info!("module {} not found, skipping", module_name);
                        continue;
                    }
                    Some(x) => x,
                };

                let built_module = match module_constructor(self.app_send.clone().unwrap()) {
                    ROk(x) => x,
                    RErr(e) => {
                        log::error!("error during creation of {module_name}: {e:#?}");
                        continue;
                    }
                };
                module_order.push(module_name.to_string());
                // info!("loading module {}", module.get_name());
                self.module_map
                    .blocking_lock()
                    .insert(module_name.to_string(), built_module);
            }
        }

        log::info!("loaded modules: {:?}", module_order);
        module_order
    }

    //TODO layout loading from .so not tested yet but it should work identically to module loading
    pub(crate) fn load_layout_manager(&mut self) {
        let layout_manager_definitions = crate::module_loading::get_lm_definitions();

        if self.config.layout.is_none() {
            log::info!("no layout manager in config, using default: SimpleLayout");
            self.load_simple_layout();
            return;
        }
        let lm_name = self.config.layout.as_ref().unwrap();
        if lm_name == simple_layout::NAME {
            log::info!("using layout manager: SimpleLayout");
            self.load_simple_layout();
            return;
        }
        let lm_constructor = layout_manager_definitions.get(lm_name);
        let lm_constructor = match lm_constructor {
            None => {
                log::info!(
                    "layout manager {} not found, using default: SimpleLayout",
                    lm_name
                );
                self.load_simple_layout();
                return;
            }
            Some(x) => x,
        };

        let built_lm = match lm_constructor(self.application.clone().into()) {
            ROk(x) => x,
            RErr(e) => {
                log::error!("error during creation of {lm_name}: {e:#?}");
                log::info!("using default layout manager SimpleLayout");
                self.load_simple_layout();
                return;
            }
        };
        log::info!("using layout manager: {lm_name}");
        self.layout = Some(Rc::new(Mutex::new((lm_name.clone(), built_lm))));
    }

    pub(crate) fn load_simple_layout(&mut self) {
        let layout_builder = simple_layout::new(self.application.clone().into());
        let layout = layout_builder.unwrap();
        self.layout = Some(Rc::new(Mutex::new((
            simple_layout::NAME.to_string(),
            layout,
        ))));
    }
}

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
