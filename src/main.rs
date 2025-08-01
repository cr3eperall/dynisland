use std::{
    io::ErrorKind,
    os::{fd::AsFd, unix::net::UnixStream},
    path::Path,
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use dynisland::{
    app::App,
    cli::{
        Cli,
        SubCommands::{self, *},
    },
    config, ipc,
};
use dynisland_core::abi::{abi_stable, log, module::UIServerCommand};
use env_logger::Env;
use log::Level;
use nix::unistd::Pid;

// [ ] TODO remove some unnecessary arc and mutexes
// [ ] TODO remove some unwraps and handle errors better
// [x] TODO add docs
// [x] TODO remove some unnecessary clones

// [ ] TODO detect nvidia gpu and display warning (if dynisland uses too much ram, use GSK_RENDERER=vulkan or GSK_RENDERER=gl)

// FIXME Gsk-WARNING **: 13:09:06.082: Clipping is broken, everything is clipped, but we didn't early-exit.
// maybe it's in ScrollingLabel

fn main() -> Result<()> {
    system_mimalloc::use_mimalloc!();
    env_logger::Builder::new()
        // .filter_module("dynisland", log::LevelFilter::Debug)
        // .filter_module("dynisland_core", log::LevelFilter::Debug)
        // .filter_module("dynisland_modules", log::LevelFilter::Debug)
        .filter(Some("reqwest"), log::LevelFilter::Warn)
        .parse_env(Env::default().default_filter_or(Level::Info.as_str()))
        .init();

    let cli = Cli::parse();
    let config_dir = cli
        .config_path
        .clone()
        .unwrap_or(config::get_default_config_path());
    let config = config::get_config(&config_dir);
    log::debug!("{cli:?}");
    match cli.command {
        Daemon { no_daemonize } => {
            let runtime_dir = config.get_runtime_dir();
            if let Ok(stream) = UnixStream::connect(runtime_dir.join("dynisland.sock")) {
                match ipc::send_recv_message(stream, &HealthCheck) {
                    Ok(_) => {
                        //app is already runnig
                        log::error!("Application is already running");
                    }
                    Err(_) => {
                        log::error!("Error sending HealthCheck");
                    }
                };
                return Ok(());
            } else {
                let _ = std::fs::remove_file(runtime_dir.join("dynisland.sock"));
            }
            let pid = if !no_daemonize {
                let log_path = runtime_dir.join("dynisland.log");
                detach(&log_path)?
            } else {
                Pid::from_raw(std::process::id() as i32)
            };
            //init GTK
            gtk::init().with_context(|| "failed to init gtk")?;
            let app = App::default();
            log::info!("pid: {pid}");
            app.run(&config_dir)?;
        }
        Reload
        | Inspector
        | HealthCheck
        | ActivityNotification {
            activity_identifier: _,
            mode: _,
            duration: _,
        }
        | Module {
            module_name: _,
            args: _,
        }
        | Layout { args: _ }
        | ListActivities
        | ListLoadedModules => {
            let socket_path = config.get_runtime_dir().join("dynisland.sock");
            match UnixStream::connect(socket_path.clone()) {
                Ok(stream) => {
                    if let Some(response) = ipc::send_recv_message(stream, &cli.command)? {
                        println!("Response: \n{response}");
                    }
                    // if cli.command == HealthCheck {
                    //     println!("OK");
                    // }
                }
                Err(err) => {
                    log::error!("Error opening dynisland socket: {err}");
                    if matches!(err.kind(), ErrorKind::ConnectionRefused) {
                        log::info!("Connection refused, deleting old socket file");
                        std::fs::remove_file(socket_path.clone())?;
                    }
                }
            };
        }
        Kill => {
            let socket_path = config.get_runtime_dir().join("dynisland.sock");
            match UnixStream::connect(socket_path.clone()) {
                Ok(stream) => {
                    let response = ipc::send_recv_message(stream, &cli.command)?;
                    println!("Kill message sent");
                    let has_responded = if let Some(response) = response {
                        println!("Response: \n{response}");
                        true
                    } else {
                        false
                    };
                    let mut tries = 0;
                    while socket_path.exists() && tries < 10 {
                        thread::sleep(Duration::from_millis(500));
                        if !has_responded {
                            print!(".");
                        }
                        tries += 1;
                    }
                    println!();
                    if tries == 10 {
                        log::error!("Failed to stop the old instance, manual kill needed");
                    } else if !has_responded {
                        println!("OK");
                    }
                }
                Err(err) => {
                    if matches!(err.kind(), ErrorKind::ConnectionRefused) {
                        log::info!("Connection refused, deleting old socket file");
                        std::fs::remove_file(socket_path.clone())?;
                    } else {
                        log::warn!(
                            "Error connecting to socket, app is probably not running: {err}"
                        );
                    }
                }
            };
        }
        Restart { no_daemonize } => {
            let socket_path = config.get_runtime_dir().join("dynisland.sock");
            match UnixStream::connect(socket_path.clone()) {
                Ok(stream) => {
                    let response = ipc::send_recv_message(stream, &SubCommands::Kill)?;
                    let has_responded = if let Some(response) = response {
                        log::info!("Response: \n{response}");
                        true
                    } else {
                        false
                    };
                    log::info!("Waiting for daemon to die");
                    let mut tries = 0;
                    while socket_path.exists() && tries < 10 {
                        thread::sleep(Duration::from_millis(500));
                        if !has_responded {
                            print!(".");
                        }
                        tries += 1;
                    }
                    println!();
                    if tries == 10 {
                        log::error!("failed to stop the old instance, manual kill needed");
                    } else if !has_responded {
                        println!("OK");
                    }
                }
                Err(err) => {
                    log::error!("Error opening dynisland socket: {err}");
                    if matches!(err.kind(), ErrorKind::ConnectionRefused) {
                        log::info!("Connection refused, trying to delete old socket file");
                        std::fs::remove_file(socket_path.clone())?;
                    }
                }
            };

            let pid = if !no_daemonize {
                let path = config.get_runtime_dir().join("dynisland.log");
                detach(&path)?
            } else {
                Pid::from_raw(std::process::id() as i32)
            };
            //init GTK
            gtk::init().with_context(|| "failed to init gtk")?;
            let app = App::default();
            log::info!("pid: {pid}");
            app.run(&config_dir)?;
        }
        DefaultConfig {
            replace_current_config,
        } => {
            gtk::init().with_context(|| "failed to init gtk")?;
            let mut app = App {
                config_dir,
                ..Default::default()
            };
            let (abi_app_send, _abi_app_recv) =
                abi_stable::external_types::crossbeam_channel::unbounded::<UIServerCommand>();
            app.app_send = Some(abi_app_send);
            let (_conf, conf_str) = app.get_default_config();
            println!("{conf_str}");
            if replace_current_config {
                todo!();
            }
        }
    }
    Ok(())
}

fn detach(log_file_path: &Path) -> Result<Pid> {
    std::fs::create_dir_all(log_file_path.parent().expect("invalid log path"))?;

    // detach from terminal
    let pid = match unsafe { nix::unistd::fork()? } {
        nix::unistd::ForkResult::Child => nix::unistd::setsid(),
        nix::unistd::ForkResult::Parent { .. } => {
            // nix::unistd::daemon(false, false);
            std::process::exit(0);
        }
    }?;

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(false)
        .truncate(true)
        .open(log_file_path)
        .unwrap_or_else(|err| {
            panic!(
                "Error opening log file ({}), for writing: {err}",
                log_file_path.to_string_lossy()
            )
        });
    let fd = file.as_fd();

    if nix::unistd::isatty(std::io::stdout().as_fd())? {
        nix::unistd::dup2_stdout(fd)?;
    }
    if nix::unistd::isatty(std::io::stderr().as_fd())? {
        nix::unistd::dup2_stderr(fd)?;
    }
    Ok(pid)
}
