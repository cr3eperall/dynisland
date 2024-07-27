use std::{
    process::Stdio,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use abi_stable::{
    external_types::crossbeam_channel::RSender,
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError,
        RResult::{self, ROk},
        RString,
    },
};
use anyhow::Context;
use dynisland_abi::module::{ModuleType, SabiModule, SabiModule_TO, UIServerCommand};
use env_logger::Env;
use log::Level;
use mpris::{DBusError, TrackID};

use dynisland_core::{
    base_module::{BaseModule, ProducerRuntime},
    cast_dyn_any,
    dynamic_property::DynamicPropertyAny,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

use crate::{
    player_info::{MprisPlayer, MprisProgress},
    utils, visualizer,
    widget::{self, UIAction, UIPlaybackStatus},
    NAME,
};

pub const DEFAULT_ALBUM_ART_PATH: &str = "file:///home/david/Pictures/Music_not_playing.svg";
const CHECK_DELAY: u64 = 5;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MusicConfig {
    //allowed_players: cider2, (?cider1, ?spotify...)
    player_name: String,
}

pub struct MusicModule {
    base_module: BaseModule<MusicModule>,
    producers_rt: ProducerRuntime,
    config: MusicConfig,
    action_channel: (
        UnboundedSender<UIAction>,
        Arc<Mutex<UnboundedReceiver<UIAction>>>,
    ),
    find_new_player: UnboundedSender<()>,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let base_module = BaseModule::new(NAME, app_send.clone());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (restart_tx, mut restart_rx) = tokio::sync::mpsc::unbounded_channel();
    let producers_rt = ProducerRuntime::new();
    let prod_hdl = producers_rt.clone();
    std::thread::spawn(move || {
        let mut last_attempt = Instant::now();
        loop {
            match restart_rx.blocking_recv() {
                Some(_) => {
                    if last_attempt.elapsed() < Duration::from_millis(1000) {
                        log::info!("no player found: sleeping for {} seconds", CHECK_DELAY);
                        thread::sleep(Duration::from_secs(CHECK_DELAY));
                    }
                    last_attempt = Instant::now();
                    log::info!("searching for a new player");
                    prod_hdl.shutdown_blocking();
                    app_send
                        .send(UIServerCommand::RestartProducers(NAME.into()))
                        .unwrap();
                }
                None => todo!(),
            }
        }
    });
    let this = MusicModule {
        base_module,
        producers_rt,
        config: MusicConfig::default(),
        action_channel: (tx, Arc::new(Mutex::new(rx))),
        find_new_player: restart_tx,
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
    fn init(&self) {
        let base_module = self.base_module.clone();
        let action_tx = self.action_channel.0.clone();
        glib::MainContext::default().spawn_local(async move {
            //create activity
            let act =
                widget::get_activity(base_module.prop_send(), NAME, "music-activity", action_tx);

            //register activity and data producer
            base_module.register_activity(act).unwrap();
            base_module.register_producer(producer);
        });
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();

        self.config = conf
            .into_rust()
            .with_context(|| "failed to parse config to struct")
            .unwrap();
        ROk(())
    }

    fn restart_producers(&self) {
        self.producers_rt.shutdown_blocking();
        self.producers_rt.reset_blocking();
        //restart producers
        for producer in self
            .base_module
            .registered_producers()
            .blocking_lock()
            .iter()
        {
            producer(self);
        }
    }
}

#[allow(unused_variables)]
fn producer(module: &MusicModule) {
    let config = &module.config;
    let activities = &module.base_module.registered_activities();
    let mode = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "mode")
        .unwrap();
    let album_art = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "album-art")
        .unwrap();
    let visualizer_gradient = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "visualizer-gradient")
        .unwrap();
    let visualizer_data = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "visualizer-data")
        .unwrap();
    let metadata = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "music-metadata")
        .unwrap();
    let time = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "music-time")
        .unwrap();
    let playback = activities
        .blocking_lock()
        .get_property_any_blocking("music-activity", "playback-status")
        .unwrap();

    log::info!("starting producer");
    let (album_art1, visualizer_gradient1)=(album_art.clone(), visualizer_gradient.clone());
    tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async move{
        set_album_art(None, &album_art1, &visualizer_gradient1).await;
    });
    let player = match MprisPlayer::new(&config.player_name) {
        Ok(player) => player,
        Err(_) => {
            module.find_new_player.send(()).unwrap();
            return;
        }
    };
    let (mut event_rx, seek_tx) = player
        .start_progress_tracker(Duration::from_millis(200))
        .unwrap();

    visualizer_task(module, visualizer_data);

    // Update UI data
    let (metadata_1, time_1) = (metadata.clone(), time.clone());
    let producer_rt = module.producers_rt.clone();
    let find_new_player_channel = module.find_new_player.clone();
    module.producers_rt.handle().spawn(async move {
        //init UI
        let metadata = player.get_metadata();
        if let Ok(metadata) = metadata {
            set_album_art(metadata.art_url(), &album_art, &visualizer_gradient).await;
        }

        let mut track_id = player
            .get_current_track_id()
            .unwrap_or(TrackID::no_track())
            .to_string();
        track_id.push_str(
            &player
                .get_metadata()
                .map(|meta| meta.title().unwrap_or("").to_owned())
                .unwrap_or(String::from("")),
        );

        while let Some(event) = event_rx.recv().await {
            match event {
                crate::player_info::MprisProgressEvent::PlayerQuit => {
                    log::warn!("player has quit");

                    set_album_art(None, &album_art, &visualizer_gradient).await;

                    time_1
                        .lock()
                        .await
                        .set::<(Duration, Duration)>((Duration::ZERO, Duration::from_nanos(1)))
                        .unwrap();

                    metadata_1
                        .lock()
                        .await
                        .set::<(String, String)>(("".to_string(), "".to_string()))
                        .unwrap();

                    playback
                        .lock()
                        .await
                        .set(UIPlaybackStatus {
                            playback_status: mpris::PlaybackStatus::Stopped,
                            can_playpause: false,
                            can_go_next: false,
                            can_go_previous: false,
                            can_loop: false,
                            can_shuffle: false,
                            shuffle: true,
                            loop_status: mpris::LoopStatus::Playlist,
                        })
                        .unwrap();

                    find_new_player_channel.send(()).unwrap();
                    return;
                }
                crate::player_info::MprisProgressEvent::Progress(prog) => {
                    time_1
                        .lock()
                        .await
                        .set::<(Duration, Duration)>((
                            prog.position,
                            prog.metadata.length().unwrap_or(Duration::ZERO),
                        ))
                        .unwrap();
                    set_playback_status(&playback, &prog).await;
                    let (song_name, artist_name) = (
                        match prog.metadata.title() {
                            Some(title) => title.to_string(),
                            None => "".to_string(),
                        },
                        match prog.metadata.artists() {
                            Some(artist) => artist
                                .first()
                                .map(|val| val.to_string())
                                .unwrap_or("".to_string()),
                            None => "".to_string(),
                        },
                    );
                    let mut new_trackid = prog
                        .metadata
                        .track_id()
                        .unwrap_or(TrackID::no_track())
                        .to_string();
                    new_trackid.push_str(prog.metadata.title().unwrap_or(""));
                    if new_trackid != track_id {
                        set_album_art(prog.metadata.art_url(), &album_art, &visualizer_gradient)
                            .await;
                    }

                    metadata_1
                        .lock()
                        .await
                        .set::<(String, String)>((song_name, artist_name))
                        .unwrap();
                }
            }
        }
    });

    // Execute actions from UI
    action_task(module, seek_tx);

    // Check if config player came back
    wait_for_new_player_task(module);
}

fn action_task(module: &MusicModule, seek_tx: UnboundedSender<Duration>) {
    let player = match MprisPlayer::new(&module.config.player_name) {
        Ok(player) => player,
        Err(_) => {
            module.find_new_player.send(()).unwrap();
            return;
        }
    };
    let action_rx = module.action_channel.1.clone();
    let find_new_player_channel = module.find_new_player.clone();
    module.producers_rt.handle().spawn(async move {
        while let Some(action) = action_rx.lock().await.recv().await {
            match action {
                UIAction::Shuffle => {
                    log::warn!("shuffle {}", player.get_shuffle().unwrap());
                    let res = player.set_shuffle(!player.get_shuffle().unwrap());
                    if matches!(res, Err(DBusError::TransportError(_))) {
                        find_new_player_channel.send(()).unwrap();
                        break;
                    }
                    log::warn!("{:?} {}", res, player.get_shuffle().unwrap());
                }
                UIAction::Previous => {
                    if matches!(player.previous(), Err(DBusError::TransportError(_))) {
                        find_new_player_channel.send(()).unwrap();
                        break;
                    }
                }
                UIAction::PlayPause => {
                    if matches!(player.play_pause(), Err(DBusError::TransportError(_))) {
                        find_new_player_channel.send(()).unwrap();
                        break;
                    }
                }
                UIAction::Next => {
                    if matches!(player.next(), Err(DBusError::TransportError(_))) {
                        find_new_player_channel.send(()).unwrap();
                        break;
                    }
                }
                UIAction::Loop => {
                    if matches!(
                        player.set_loop(
                            match player.get_loop().unwrap_or(mpris::LoopStatus::None) {
                                mpris::LoopStatus::None => mpris::LoopStatus::Track,
                                mpris::LoopStatus::Track => mpris::LoopStatus::Playlist,
                                mpris::LoopStatus::Playlist => mpris::LoopStatus::None,
                            }
                        ),
                        Err(DBusError::TransportError(_))
                    ) {
                        find_new_player_channel.send(()).unwrap();
                        break;
                    }
                }
                UIAction::SetPosition(pos) => {
                    let tid = player.get_current_track_id().expect("no track id");
                    let _ = player.set_position(tid.as_str(), pos);
                    // log::warn!("seeked to {:?}", pos);
                    seek_tx.send(pos).expect("failed to refresh time");
                }
            }
        }
    });
}

async fn set_playback_status(
    playback: &Arc<Mutex<DynamicPropertyAny>>,
    prog: &crate::player_info::MprisProgress,
) {
    let old_playback_status = playback.lock().await;
    let playback_status = cast_dyn_any!(old_playback_status.get(), UIPlaybackStatus);
    let mut playback_status = if let Some(val) = playback_status {
        val.clone()
    } else {
        UIPlaybackStatus {
            playback_status: prog.playback_status,
            can_playpause: false,
            can_go_next: false,     //TODO change
            can_go_previous: false, //TODO change
            can_loop: false,
            can_shuffle: false,
            shuffle: prog.shuffle,
            loop_status: prog.loop_status,
        }
    };
    drop(old_playback_status);

    playback_status.playback_status = prog.playback_status;
    playback_status.shuffle = prog.shuffle;
    playback_status.loop_status = prog.loop_status;
    playback_status.can_go_next = prog.can_go_next;
    playback_status.can_go_previous = prog.can_go_prev;
    playback_status.can_loop = prog.can_loop;
    playback_status.can_shuffle = prog.can_shuffle;
    playback_status.can_playpause = prog.can_playpause;

    playback
        .lock()
        .await
        .set::<UIPlaybackStatus>(playback_status)
        .unwrap();
}

async fn set_album_art(
    art_url: Option<&str>,
    album_art: &Arc<Mutex<DynamicPropertyAny>>,
    visualizer_gradient: &Arc<Mutex<DynamicPropertyAny>>,
) {
    let image = utils::get_album_art_from_url(art_url.unwrap_or_else(|| {
        log::info!("no album art");
        DEFAULT_ALBUM_ART_PATH
    }))
    .await
    .unwrap_or(
        utils::get_album_art_from_url(DEFAULT_ALBUM_ART_PATH)
            .await
            .unwrap_or(Vec::new()),
    );
    let gradient = visualizer::gradient_from_image_bytes(&image);
    album_art.lock().await.set(image).unwrap();
    visualizer_gradient.lock().await.set(gradient).unwrap();
}

fn wait_for_new_player_task(module: &MusicModule) {
    let player_bus_name = module.config.player_name.clone();
    let find_new_player_channel = module.find_new_player.clone();
    module.producers_rt.handle().spawn(async move {
        if let Ok(pl) = MprisPlayer::find_new_player(&player_bus_name) {
            if pl.bus_name_player_name_part() == player_bus_name {
                return;
            }
        } else {
            find_new_player_channel.send(()).unwrap();
            return;
        }
        loop {
            if let Ok(pl) = MprisPlayer::find_new_player(&player_bus_name) {
                if pl.bus_name_player_name_part() == player_bus_name {
                    find_new_player_channel.send(()).unwrap();
                    return;
                }
            } else {
                find_new_player_channel.send(()).unwrap();
                return;
            }
            tokio::time::sleep(Duration::from_secs(CHECK_DELAY)).await;
        }
    });
}

fn visualizer_task(module: &MusicModule, visualizer_data: Arc<Mutex<DynamicPropertyAny>>) {
    let mut cleanup = module.producers_rt.cleanup_notifier.subscribe();
    module.producers_rt.handle().spawn(async move{
            //FIXME replace with new script
            let child=Command::new("sh")
            .arg("-c")
            .arg("/home/david/.config/eww/scripts/music-wave")
            .stdout(Stdio::piped())
            .spawn();
            if let Err(err) = child{
                log::error!("failed to start visualizer command: {:?}",err);
                return;
            }
            let mut child = child.unwrap();
            let reader = BufReader::new( child.stdout.take().unwrap());
            let mut lines=reader.lines();
            tokio::select! {
                _ = async {
                    while let Ok(line)=lines.next_line().await {
                        let line =match line {
                            Some(line) => line.strip_prefix('[').unwrap().strip_suffix(']').unwrap().to_string(),
                            None => break,
                        };
                        visualizer_data.lock().await.set(visualizer::parse_input(&line)).unwrap();
                    }
                }=> {
                    log::info!("visualizer command has exited")
                },
                _ = async {
                    let tx=cleanup.recv().await.unwrap();
                    child.kill().await.unwrap();
                    tx.send(()).unwrap();
                } => {
                    log::info!("visualizer cleanup done");
                }
            }
    });
}
