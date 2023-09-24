#![feature(async_closure)]
#![feature(trait_upcasting)]

use anyhow::Result;

fn main() -> Result<()> {
    dynisland::app::initialize_server()
}