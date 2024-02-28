use anyhow::{Context, Result};
use dynisland::app::App;
use env_logger::Env;
use log::Level;

// [ ] TODO remove some unnecessary arc and mutexes
// [ ] TODO remove some unwraps and handle errors better
// [ ] TODO add docs
// [ ] TODO remove some unnecessary clones

// [ ] TODO detect nvidia gpu and display warning (if dynisland uses too much ram, use GSK_RENDERER=vulkan)

fn main() -> Result<()> {
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;

    env_logger::Builder::new()
        .filter_module("dynisland", log::LevelFilter::Debug)
        .filter_module("dynisland_core", log::LevelFilter::Debug)
        // .filter_module("dynisland_modules", log::LevelFilter::Debug)
        .parse_env(Env::default().default_filter_or(Level::Warn.as_str()))
        .init();

    let app = App::default();
    app.initialize_server()?;
    Ok(())
}
