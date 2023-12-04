use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use dynisland::{
    app::{self, App},
    config,
};
use tokio::sync::Mutex;

extern crate dynisland_modules; //need this to force the modules to be linked

fn main() -> Result<()> {
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;
    let (hdl, shutdown) = app::get_new_tokio_rt(); //TODO it's ugly, should change it
    let app = App {
        window: app::get_window(),
        module_map: Arc::new(Mutex::new(HashMap::new())),
        producers_handle: hdl,
        producers_shutdown: shutdown,
        app_send: None,
        config: config::Config::default(),
    };
    app.initialize_server()?;
    Ok(())
}
