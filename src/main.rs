use anyhow::{Context, Result};
use dynisland::app::App;

extern crate dynisland_modules; //need this to force the modules to be linked

// [ ]  TODO remove some unnecessary arc and mutexes
// [ ] TODO remove some unwraps and handle errors better

fn main() -> Result<()> {
    //init GTK
    gtk::init().with_context(|| "failed to init gtk")?;
    let app = App::default();
    app.initialize_server()?;
    Ok(())
}
