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
    ActivityNotification {
        activity_identifier: String,
        #[arg(help = "0: Minimal, 1: Compact, 2: Expanded, 3: Overlay")]
        mode: u8,
        duration: Option<u64>,
    },
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
    ListActivities,
    Module {
        module_name: String,
        // #[arg(required = true, value_delimiter = ' ', num_args = 1..)]
        args: Vec<String>,
    },
    Layout {
        args: Vec<String>,
    },
}
