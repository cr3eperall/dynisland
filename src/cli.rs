use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: SubCommands,

    #[arg(long, short)]
    pub config_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubCommands {
    Daemon {
        #[arg(short, long, required = false, default_value_t = false)]
        no_daemonize: bool,
    },
    Reload,
    Inspector,
    HealthCheck,
    Kill,
    Restart {
        #[arg(short, long, required = false, default_value_t = false)]
        no_daemonize: bool,
    },
    DefaultConfig {
        // #[arg(short, long, required = false, default_value_t = false)]
        #[arg(skip = false)]
        replace_current_config: bool,
    },
}
