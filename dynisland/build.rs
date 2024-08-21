use clap::{Command, CommandFactory};
use clap_complete::{generate_to, Shell};
use std::io::Error;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let outdir = "../target";
    let bin_name = "dynisland";

    let mut cmd: Command = Cli::command_for_update();
    let shells: [Shell; 4] = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Elvish];
    for shell in shells {
        let _path = generate_to(shell, &mut cmd, bin_name, outdir)?;

        // println!(
        //     "cargo:warning=completion file for {shell} is generated in {}",
        //     _path.to_str().unwrap()
        // );
    }
    Ok(())
}
