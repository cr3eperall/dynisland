use std::{
    io::{Read, Write},
    path::Path,
    time::Duration,
};

use anyhow::{anyhow, Result};
use dynisland_core::{
    abi::{log, module::ActivityIdentifier},
    graphics::activity_widget::boxed_activity_mode::ActivityMode,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::mpsc::UnboundedSender,
};

use crate::{app::BackendServerCommand, cli::SubCommands};

pub async fn open_socket(
    runtime_path: &Path,
    server_send: UnboundedSender<BackendServerCommand>,
    server_response_recv: &mut tokio::sync::mpsc::UnboundedReceiver<Option<String>>,
) -> Result<()> {
    let _ = std::fs::remove_file(runtime_path.join("dynisland.sock"));
    let listener = UnixListener::bind(runtime_path.join("dynisland.sock"))?;
    loop {
        let (mut stream, _socket) = listener.accept().await?;
        let message = read_message(&mut stream).await?;
        log::debug!("IPC message received: {message:?}");
        match message {
            SubCommands::Reload => {
                server_send.send(BackendServerCommand::ReloadConfig)?;
            }
            SubCommands::Inspector => {
                server_send.send(BackendServerCommand::OpenInspector)?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
            }
            SubCommands::Kill => {
                server_send.send(BackendServerCommand::Stop)?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
                break;
            }
            SubCommands::HealthCheck => {
                log::info!("received HealthCheck, Everything OK");
                let _ = send_response(&mut stream, None).await;
            }
            SubCommands::ActivityNotification {
                activity_identifier,
                mode,
                duration,
            } => {
                let components: Vec<&str> = activity_identifier.split('@').collect();
                if components.len() != 2 {
                    log::error!("invalid activity identifier: {activity_identifier}");
                    continue;
                }
                let id = ActivityIdentifier::new(components[1], components[0]);
                let mode = ActivityMode::try_from(mode).map_err(|e| anyhow!(e))?;
                server_send.send(BackendServerCommand::ActivityNotification(
                    id, mode, duration,
                ))?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
            }
            SubCommands::ListActivities => {
                server_send.send(BackendServerCommand::ListActivities)?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
            }
            SubCommands::ListLoadedModules => {
                server_send.send(BackendServerCommand::ListLoadedModules)?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
            }
            SubCommands::Module { module_name, args } => {
                server_send.send(BackendServerCommand::ModuleCliCommand(
                    module_name,
                    args.join(" "),
                ))?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
            }
            SubCommands::Layout { args } => {
                server_send.send(BackendServerCommand::LayoutCliCommand(args.join(" ")))?;
                if let Ok(Some(response)) =
                    tokio::time::timeout(Duration::from_millis(800), server_response_recv.recv())
                        .await
                {
                    let _ = send_response(&mut stream, response).await;
                }
            }
            SubCommands::DefaultConfig {
                replace_current_config: _,
            }
            | SubCommands::Daemon { no_daemonize: _ }
            | SubCommands::Restart { no_daemonize: _ } => {
                log::error!("invalid message passed to ipc");
            }
        }
        stream.shutdown().await?;
    }

    Ok(())
}

pub async fn read_message(stream: &mut UnixStream) -> Result<SubCommands> {
    let mut message_len_bytes = [0u8; 4];
    stream.read_exact(&mut message_len_bytes).await?;
    let message_len = u32::from_be_bytes(message_len_bytes) as usize;
    let mut message: Vec<u8> = Vec::with_capacity(message_len);
    while message.len() < message_len {
        stream.read_buf(&mut message).await?;
    }

    Ok(bincode::decode_from_slice(&message,bincode::config::standard())?.0)
}

pub async fn send_response(stream: &mut UnixStream, message: Option<String>) -> Result<()> {
    let response = bincode::encode_to_vec(&message.unwrap_or("OK".to_string()),bincode::config::standard())?;
    stream.write_all(&response).await?;
    Ok(())
}

pub fn send_recv_message(
    mut stream: std::os::unix::net::UnixStream,
    message: &SubCommands,
) -> Result<Option<String>> {
    stream.set_nonblocking(false)?;

    let message = bincode::encode_to_vec(&message,bincode::config::standard())?;
    let message_len_bytes = (message.len() as u32).to_be_bytes();
    stream.write_all(&message_len_bytes)?;
    stream.write_all(&message)?;
    let mut buf = Vec::new();
    stream.set_read_timeout(Some(Duration::from_millis(1000)))?;
    stream.read_to_end(&mut buf)?;

    Ok(if buf.is_empty() {
        None
    } else {
        let (buf,_) = bincode::decode_from_slice(&buf,bincode::config::standard())?;
        Some(buf)
    })
}
