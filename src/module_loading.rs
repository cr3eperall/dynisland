use std::{
    collections::HashMap,
    path::Path,
    rc::Rc,
};
#[cfg(all(debug_assertions, not(feature = "embed_modules")))]
use std::path::PathBuf;

use abi_stable::{
    external_types::crossbeam_channel::RSender,
    library::{lib_header_from_path, LibraryError},
    std_types::{
        RBoxError, RResult,
        RResult::{RErr, ROk},
    },
    type_layout::TypeLayout,
    StableAbi,
};
use dynisland_core::abi::{
    abi_stable,
    layout::{LayoutManagerBuilderRef, LayoutManagerType},
    log,
    module::{ModuleBuilderRef, ModuleType, UIServerCommand},
    SabiApplication,
};
use tokio::sync::Mutex;

use crate::{
    app::App,
    layout_manager::{self, simple_layout},
};

impl App {
    pub(crate) fn load_modules(&mut self, config_dir: &Path) -> Vec<String> {
        let mut module_order = vec![];
        let module_def_map = crate::module_loading::get_module_definitions(config_dir);

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
                        log::warn!("module {} not found, skipping", module_name);
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

    pub(crate) fn load_layout_manager(&mut self, config_dir: &Path) {
        let layout_manager_definitions = crate::module_loading::get_lm_definitions(config_dir);

        if self.config.layout.is_none() {
            log::info!("no layout manager in config, using default: SimpleLayout");
            self.load_simple_layout();
            return;
        }
        let lm_name = self.config.layout.as_ref().unwrap();
        if lm_name == layout_manager::NAME {
            log::info!("using layout manager: SimpleLayout");
            self.load_simple_layout();
            return;
        }
        let lm_constructor = layout_manager_definitions.get(lm_name);
        let lm_constructor = match lm_constructor {
            None => {
                log::warn!(
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
            layout_manager::NAME.to_string(),
            layout,
        ))));
    }
}

pub fn get_module_definitions(
    _config_dir: &Path,
) -> HashMap<String, extern "C" fn(RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>> {
    let mut module_def_map = HashMap::<
        String,
        extern "C" fn(RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError>,
    >::new();

    let module_path = {
        #[cfg(all(debug_assertions, not(feature = "embed_modules")))]
        {
            PathBuf::from("./target/debug/")
        }
        #[cfg(any(not(debug_assertions), feature = "embed_modules"))]
        {
            _config_dir.join("modules")
        }
    };

    #[cfg(feature = "embed_modules")]
    {
        let clock_module = clock_module::instantiate_root_module();
        module_def_map.insert(clock_module.name().into(), clock_module.new());

        let music_module = music_module::instantiate_root_module();
        module_def_map.insert(music_module.name().into(), music_module.new());

        let script_module = script_module::instantiate_root_module();
        module_def_map.insert(script_module.name().into(), script_module.new());
    }

    let files = match std::fs::read_dir(&module_path) {
        Ok(files) => files,
        Err(err) => {
            log::error!("failed to read module directory ({module_path:?}): {err}");
            return module_def_map;
        }
    };
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

pub fn get_lm_definitions(
    _config_dir: &Path,
) -> HashMap<
    String,
    extern "C" fn(SabiApplication) -> RResult<LayoutManagerType, abi_stable::std_types::RBoxError>,
> {
    let mut lm_def_map = HashMap::<
        String,
        extern "C" fn(SabiApplication) -> RResult<LayoutManagerType, RBoxError>,
    >::new();

    let lm_path = {
        #[cfg(all(debug_assertions, not(feature = "embed_modules")))]
        {
            PathBuf::from("./target/debug/")
        }
        #[cfg(any(not(debug_assertions), feature = "embed_modules"))]
        {
            _config_dir.join("layouts")
        }
    };

    #[cfg(feature = "embed_modules")]
    {
        let dynamic_layout = dynamic_layoutmanager::instantiate_root_module();
        lm_def_map.insert(dynamic_layout.name().into(), dynamic_layout.new());
    }

    let files = match std::fs::read_dir(&lm_path) {
        Ok(files) => files,
        Err(err) => {
            log::error!("failed to read layout manager directory ({lm_path:?}): {err}");
            return lm_def_map;
        }
    };
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
