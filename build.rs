use clap::{Command, CommandFactory};
use clap_complete::{generate_to, Shell};
use std::io::Error;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let outdir = "../target"; //idk if it works
    let bin_name = "dynisland";

    let mut cmd: Command = Cli::command_for_update();
    let shells: [Shell; 4] = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::Elvish];
    for shell in shells {
        let path = generate_to(
            shell, &mut cmd, // We need to specify what generator to use
            bin_name, // We need to specify the bin name manually
            outdir,   // We need to specify where to write to
        )?;

        println!(
            "cargo:warning=completion file for {shell} is generated in {}",
            path.to_str().unwrap()
        );
    }
    Ok(())
}
