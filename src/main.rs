#![feature(async_closure)]
#![feature(trait_upcasting)]

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use dynisland::app::App;
use tokio::sync::Mutex;

fn main() -> Result<()> {
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;

    let mut app = App {
        window: dynisland::app::get_window(),
        module_map: Arc::new(Mutex::new(HashMap::new())),
        producers_runtime: dynisland::app::get_new_tokio_rt(),
        app_send: None,
    };
    app.initialize_server()?;
    Ok(())
}
