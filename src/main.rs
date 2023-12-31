use anyhow::{Context, Result};
use dynisland::app::App;
use env_logger::Env;
use log::Level;

extern crate dynisland_modules; //need this to force the modules to be linked

// [ ] TODO remove some unnecessary arc and mutexes
// [ ] TODO remove some unwraps and handle errors better

fn main() -> Result<()> {
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;

    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Error.as_str()))
        .filter_module("dynisland", log::LevelFilter::Warn)
        .filter_module("dynisland_core", log::LevelFilter::Debug)
        .filter_module("dynisland_modules", log::LevelFilter::Debug)
        .init();

    let app = App::default();
    app.initialize_server()?;
    Ok(())
}
