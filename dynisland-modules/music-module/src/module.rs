use std::{sync::Arc, time::Duration};

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
use mpris::TrackID;
use serde::{Deserialize, Serialize};

use dynisland_core::{
    base_module::{BaseModule, ProducerRuntime},
    cast_dyn_any,
};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::{
    utils, player_info::MprisPlayer, widget::{self, UIAction, UIPlaybackStatus}, NAME
};

//FIXME remove after testing/get from config/auto detect
pub const PLAYER: &str = "cider";

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MusicConfig {
    //allowed_players: cider2, (?cider1, ?spotify...)
    enabled_player_override: Vec<String>,
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
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let base_module = BaseModule::new(NAME, app_send);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let this = MusicModule {
        base_module,
        producers_rt: ProducerRuntime::new(),
        config: MusicConfig::default(),
        action_channel: (tx, Arc::new(Mutex::new(rx))),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
    #[allow(clippy::let_and_return)]
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

    #[allow(clippy::let_and_return)]
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

    #[allow(clippy::let_and_return)]
    fn restart_producers(&self) {
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
    // debug!("starting task");
    let config = config.clone();
    let (mut event_rx, seek_tx) = MprisPlayer::new(PLAYER)
        .start_progress_tracker(Duration::from_millis(200))
        .unwrap();

    // Update UI data
    let (metadata_1, time_1, playback_1) = (metadata.clone(), time.clone(), playback.clone());
    module.producers_rt.handle().spawn(async move {
        let player = MprisPlayer::new(PLAYER);
        let metadata=player.get_metadata();
        if let Ok(metadata)=metadata {
            album_art.lock().await.set(utils::get_album_art_from_url(metadata.art_url().unwrap_or("")).await.unwrap_or(Vec::new())).unwrap();
        }
        
        let mut track_id=player.get_current_track_id().unwrap_or(TrackID::no_track());
        while let Some(event) = event_rx.recv().await {
            match event {
                crate::player_info::MprisProgressEvent::PlayerQuit => {
                    todo!("not implemented yet");
                }
                crate::player_info::MprisProgressEvent::Progress(prog) => {
                    // log::warn!("recieved time: {}",prog.position.as_millis());
                    time_1
                        .lock()
                        .await
                        .set::<(Duration, Duration)>((
                            prog.position,
                            prog.metadata.length().unwrap(),
                        ))
                        .unwrap();
                    let opt_playback_status = playback_1.lock().await;
                    let playback_status =
                        cast_dyn_any!(opt_playback_status.get(), UIPlaybackStatus);
                    let mut playback_status = if let Some(val) = playback_status {
                        playback_status.unwrap().clone()
                    } else {
                        UIPlaybackStatus {
                            playback_status: prog.playback_status,
                            can_go_next: true,     //TODO change
                            can_go_previous: true, //TODO change
                            shuffle: prog.shuffle,
                            loop_status: prog.loop_status,
                        }
                    };
                    drop(opt_playback_status);

                    playback_status.playback_status = prog.playback_status;
                    playback_status.shuffle = prog.shuffle;
                    playback_status.loop_status = prog.loop_status;
                    playback_status.can_go_next = prog.can_go_next;
                    playback_status.can_go_previous = prog.can_go_prev;

                    if prog.metadata.track_id().unwrap_or(TrackID::no_track())!=track_id{
                        album_art.lock().await.set(utils::get_album_art_from_url(prog.metadata.art_url().unwrap_or("")).await.unwrap_or(Vec::new())).unwrap();
                        track_id=prog.metadata.track_id().unwrap_or(TrackID::no_track());
                    }

                    playback_1
                        .lock()
                        .await
                        .set::<UIPlaybackStatus>(playback_status)
                        .unwrap();
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
    let action_rx = module.action_channel.1.clone();
    module.producers_rt.handle().spawn(async move {
        let music_player = MprisPlayer::new(PLAYER);
        while let Some(action) = action_rx.lock().await.recv().await {
            match action {
                UIAction::Shuffle => {
                    music_player
                        .set_shuffle(!music_player.get_shuffle().unwrap())
                        .unwrap();
                }
                UIAction::Previous => {
                    music_player.previous().unwrap();
                }
                UIAction::PlayPause => {
                    music_player.play_pause().unwrap();
                }
                UIAction::Next => {
                    music_player.next().unwrap();
                }
                UIAction::Loop => {
                    music_player
                        .set_loop(match music_player.get_loop().unwrap() {
                            mpris::LoopStatus::None => mpris::LoopStatus::Track,
                            mpris::LoopStatus::Track => mpris::LoopStatus::Playlist,
                            mpris::LoopStatus::Playlist => mpris::LoopStatus::None,
                        })
                        .unwrap();
                }
                UIAction::SetPosition(pos) => {
                    let tid = music_player.get_current_track_id().expect("no track id");
                    music_player
                        .set_position(tid.as_str(), pos)
                        .expect("failed to seek");
                    // log::warn!("seeked to {:?}", pos);
                    seek_tx.send(pos).expect("failed to refresh time");
                }
            }
        }
    });
}
