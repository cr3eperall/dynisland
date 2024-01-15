use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("modules.rs");
    fs::write(
        dest_path, //TODO find a way to add the dependency to Cargo.toml
        "
        extern crate dynisland_modules;
        ",
    )
    .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
