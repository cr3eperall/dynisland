use std::{io::Write, path::Path};

use anyhow::{anyhow, Ok, Result};
use dynisland_abi::module::ActivityIdentifier;
use dynisland_core::graphics::activity_widget::boxed_activity_mode::ActivityMode;
use tokio::{
    io::AsyncReadExt,
    net::{UnixListener, UnixStream},
    sync::mpsc::UnboundedSender,
};

use crate::{app::BackendServerCommand, cli::SubCommands};

pub async fn open_socket(
    runtime_path: &Path,
    server_send: UnboundedSender<BackendServerCommand>,
) -> Result<()> {
    let _ = std::fs::remove_file(runtime_path.join("dynisland.sock"));
    let listener = UnixListener::bind(runtime_path.join("dynisland.sock"))?;
    loop {
        let (stream, _socket) = listener.accept().await?;
        let message = read_message(stream).await?;
        log::debug!("IPC message recieved: {message:?}");
        match message {
            SubCommands::Reload => {
                server_send.send(BackendServerCommand::ReloadConfig)?;
            }
            SubCommands::Inspector => {
                server_send.send(BackendServerCommand::OpenInspector)?;
            }
            SubCommands::Kill => {
                server_send.send(BackendServerCommand::Stop)?;
                break;
            }
            SubCommands::HealthCheck => {
                log::info!("Recieved HealthCheck, Everything OK");
            }
            SubCommands::ActivityNotification {
                activity_identifier,
                mode,
            } => {
                let components: Vec<&str> = activity_identifier.split('@').collect();
                if components.len() != 2 {
                    log::error!("invalid activity identifier: {activity_identifier}");
                    continue;
                }
                let id = ActivityIdentifier::new(components[1], components[0]);
                let mode = ActivityMode::try_from(mode).map_err(|e| anyhow!(e))?;
                server_send.send(BackendServerCommand::ActivityNotification(id, mode))?;
            }
            SubCommands::DefaultConfig {
                replace_current_config: _,
            }
            | SubCommands::Daemon { no_daemonize: _ }
            | SubCommands::Restart { no_daemonize: _ } => {
                log::error!("invalid message passed to ipc");
            }
        }
    }

    Ok(())
}

pub async fn read_message(mut stream: UnixStream) -> Result<SubCommands> {
    let mut message_len_bytes = [0u8; 4];
    stream.read_exact(&mut message_len_bytes).await?;
    let message_len = u32::from_be_bytes(message_len_bytes) as usize;
    let mut message: Vec<u8> = Vec::with_capacity(message_len);
    while message.len() < message_len {
        stream.read_buf(&mut message).await?;
    }

    Ok(bincode::deserialize(&message)?)
}

pub fn send_message(
    mut stream: std::os::unix::net::UnixStream,
    message: &SubCommands,
) -> Result<()> {
    stream.set_nonblocking(false)?;

    let message = bincode::serialize(&message)?;
    let message_len_bytes = (message.len() as u32).to_be_bytes();
    stream.write_all(&message_len_bytes)?;
    stream.write_all(&message)?;

    Ok(())
}
