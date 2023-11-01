#![feature(async_closure)]
#![feature(trait_upcasting)]

mod filters;

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use dynisland::app::App;
use tokio::sync::Mutex;

fn main() -> Result<()> {
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;
    let (hdl, shutdown) = dynisland::app::get_new_tokio_rt(); //TODO it's ugly, should change it
    let app = App {
        window: dynisland::app::get_window(),
        module_map: Arc::new(Mutex::new(HashMap::new())),
        producers_handle: hdl,
        producers_shutdown: shutdown,
        app_send: None,
        config: dynisland::config::Config::default(),
    };
    app.initialize_server()?;
    Ok(())
}
