use std::io::Error;

#[cfg(feature = "completions")]
use clap::{Command, CommandFactory};
#[cfg(feature = "completions")]
use clap_complete::{generate_to, Shell};

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    #[cfg(feature = "completions")]
    {
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
    }
    Ok(())
}
