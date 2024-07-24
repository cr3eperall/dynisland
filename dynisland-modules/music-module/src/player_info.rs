use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use mpris::{PlaybackStatus, TrackID};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(serde::Deserialize, Debug)]
pub struct CurrentSongMinimal {
    pub info: CurrentSongMinimalInfo,
}
#[derive(serde::Deserialize, Debug)]
pub struct CurrentSongMinimalInfo {
    #[serde(rename = "durationInMillis")]
    pub duration_in_millis: u64,
    #[serde(rename = "albumName")]
    pub album_name: String,
    pub name: String,
    pub artwork: Option<CurrentSongArtwork>,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "currentPlaybackTime")]
    /// in seconds
    pub current_playback_time: f64,
    #[serde(rename = "currentPlaybackProgress")]
    /// in percentage 0.0-1.0
    pub current_playback_progress: f32,
}
#[derive(serde::Deserialize, Debug)]
pub struct CurrentSongArtwork {
    pub url: String, //TODO should be path
    pub width: Option<u64>,
    pub height: Option<u64>,
}

pub enum MprisProgressEvent {
    PlayerQuit,
    Progress(MprisProgress),
}

#[derive(Clone, Debug)]
pub struct MprisProgress {
    pub progress_changed: bool,
    pub track_list_changed: bool,
    pub metadata: mpris::Metadata,
    pub playback_status: mpris::PlaybackStatus,
    pub shuffle: bool,
    pub loop_status: mpris::LoopStatus,
    pub can_go_next: bool,
    pub can_go_prev: bool,

    /// When this Progress was constructed, in order to calculate how old it is.
    instant: Instant,

    pub position: Duration,
    pub rate: f64,
    pub current_volume: f64,
}

impl<'a> From<mpris::ProgressTick<'a>> for MprisProgress {
    fn from(progress: mpris::ProgressTick) -> Self {
        Self {
            progress_changed: progress.progress_changed,
            track_list_changed: progress.track_list_changed,
            metadata: progress.progress.metadata().clone(),
            playback_status: progress.progress.playback_status(),
            shuffle: progress.progress.shuffle(),
            loop_status: progress.progress.loop_status(),
            can_go_next: true,
            can_go_prev: true,
            instant: *progress.progress.created_at(),
            position: progress.progress.position(),
            rate: progress.progress.playback_rate(),
            current_volume: progress.progress.current_volume(),
        }
    }
}

impl MprisProgress {
    pub fn age(&self) -> Duration {
        self.instant.elapsed()
    }

    pub fn elapsed(&self) -> Duration {
        let elapsed_ms = match self.playback_status {
            PlaybackStatus::Playing => Duration::as_millis(&self.age()) as f64 * self.rate,
            _ => 0.0,
        };
        Duration::from_millis(elapsed_ms as u64)
    }

    pub fn created_at(&self) -> &Instant {
        &self.instant
    }
}

pub struct MprisPlayer {
    player: Rc<std::sync::Mutex<mpris::Player>>,
}

impl MprisPlayer {
    ///uses active player as fallback
    pub fn new(name: &str) -> Self {
        let player = mpris::PlayerFinder::new()
            .expect("Could not connect to D-Bus")
            .find_by_name(name)
            .unwrap_or_else(|_| {
                mpris::PlayerFinder::new()
                    .expect("Could not connect to D-Bus")
                    .find_active()
                    .unwrap()
            });

        Self {
            player: Rc::new(std::sync::Mutex::new(player)),
        }
    }
}

unsafe impl Send for MprisPlayer {}
impl MprisPlayer {
    pub fn play(&self) -> Result<()> {
        self.player.lock().unwrap().play()?;
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        self.player.lock().unwrap().pause()?;
        Ok(())
    }

    pub fn play_pause(&self) -> Result<()> {
        self.player.lock().unwrap().play_pause()?;
        Ok(())
    }

    pub fn next(&self) -> Result<()> {
        self.player.lock().unwrap().next()?;
        Ok(())
    }

    pub fn previous(&self) -> Result<()> {
        self.player.lock().unwrap().previous()?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.player.lock().unwrap().stop()?;
        Ok(())
    }

    pub fn get_playback_status(&self) -> Result<mpris::PlaybackStatus> {
        let playback_status = self.player.lock().unwrap().get_playback_status()?;
        Ok(playback_status)
    }

    pub fn can_go_next(&self) -> Result<bool> {
        let res = self.player.lock().unwrap().can_go_next()?;
        Ok(res)
    }

    pub fn can_go_prev(&self) -> Result<bool> {
        let res = self.player.lock().unwrap().can_go_previous()?;
        Ok(res)
    }

    pub fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.player.lock().unwrap().set_shuffle(shuffle)?;
        Ok(())
    }

    pub fn get_shuffle(&self) -> Result<bool> {
        let shuffle = self.player.lock().unwrap().get_shuffle()?;
        Ok(shuffle)
    }

    pub fn set_loop(&self, repeat: mpris::LoopStatus) -> Result<()> {
        self.player.lock().unwrap().set_loop_status(repeat)?;
        Ok(())
    }

    pub fn get_loop(&self) -> Result<mpris::LoopStatus> {
        let loop_status = self.player.lock().unwrap().get_loop_status()?;
        Ok(loop_status)
    }

    pub fn seek(&self, offset: i64) -> Result<()> {
        self.player.lock().unwrap().seek(offset * 1_000_000)?;
        Ok(())
    }

    pub fn get_position(&self) -> Result<Duration> {
        let position = self.player.lock().unwrap().get_position()?;
        Ok(position)
    }

    pub fn set_position(&self, track_id: &str, position: Duration) -> Result<()> {
        let track_id = match mpris::TrackID::new(track_id) {
            Ok(track_id) => track_id,
            Err(err) => bail!("error creating track id: {:?}", err),
        };
        self.player
            .lock()
            .unwrap()
            .set_position(track_id, &position)?;
        Ok(())
    }
    pub fn get_length(&self) -> Result<Duration> {
        //TODO add fallback to cider2 api
        let metadata = self.get_metadata()?;
        let length = match metadata.get("mpris:length") {
            Some(length) => length,
            None => bail!("Length not found in metadata"),
        };
        Ok(Duration::from_micros(
            length.as_i64().unwrap().max(0).unsigned_abs(),
        ))
    }

    pub fn set_volume(&self, volume: f64) -> Result<()> {
        self.player.lock().unwrap().set_volume(volume)?;
        Ok(())
    }

    pub fn get_volume(&self) -> Result<f64> {
        let volume = self.player.lock().unwrap().get_volume()?;
        Ok(volume)
    }

    pub fn get_metadata(&self) -> Result<mpris::Metadata> {
        let metadata = self.player.lock().unwrap().get_metadata()?;
        Ok(metadata)
    }
    pub fn get_current_track_id(&self) -> Result<TrackID> {
        let metadata = self.get_metadata()?;
        let track_id = match metadata.get("mpris:trackid") {
            Some(track_id) => track_id.as_str().unwrap(),
            None => bail!("TrackId not found in metadata"),
        };
        match TrackID::new(track_id) {
            Ok(track_id) => Ok(track_id),
            Err(err) => bail!("error creating track id: {:?}", err),
        }
    }
    pub fn get_current_song_info(&self) -> Result<CurrentSongMinimalInfo> {
        let duration_millis = self.get_length()?.as_millis() as u64;
        let metadata = self.get_metadata()?;
        let name = match metadata.get("xesam:title") {
            Some(name) => name.as_str().unwrap(),
            None => "",
        };
        let album_name = match metadata.get("xesam:album") {
            Some(album_name) => album_name.as_str().unwrap(),
            None => "",
        };
        let artist_name = match metadata.get("xesam:artist") {
            Some(artist_name) => {
                let arr = artist_name.as_array().unwrap();
                arr.first().unwrap().as_str().unwrap()
            }
            None => "",
        };
        let artwork = metadata
            .get("mpris:artUrl")
            .map(|artwork| CurrentSongArtwork {
                url: artwork.as_str().unwrap().to_string(),
                width: None,
                height: None,
            });
        let current_playback_time = self.get_position()?.as_secs_f64();
        let current_playback_progress = current_playback_time as f32 / duration_millis as f32;
        Ok(CurrentSongMinimalInfo {
            duration_in_millis: duration_millis,
            name: name.to_string(),
            album_name: album_name.to_string(),
            artwork,
            artist_name: artist_name.to_string(),
            current_playback_time,
            current_playback_progress,
        })
    }

    pub fn start_signal_listener(&self) -> Result<UnboundedReceiver<mpris::Event>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let player_id = self.player.lock().unwrap().identity().to_string();
        std::thread::spawn(move || {
            let player = mpris::PlayerFinder::new()
                .expect("Could not connect to D-Bus")
                .find_by_name(&player_id)
                .unwrap_or_else(|err| {
                    log::warn!("error finding player: {:?}", err);
                    panic!()
                });
            let iter = player.events().unwrap_or_else(|err| {
                log::warn!("error getting events: {:?}", err);
                panic!()
            });
            for event in iter {
                if tx.is_closed() {
                    break;
                }
                match event {
                    Ok(event) => {
                        tx.send(event).unwrap();
                    }
                    Err(err) => {
                        log::warn!("mpris event error: {:?}", err);
                    }
                }
            }
        });
        Ok(rx)
    }

    pub fn start_progress_tracker(
        &self,
        interval: Duration,
    ) -> Result<(
        UnboundedReceiver<MprisProgressEvent>,
        UnboundedSender<Duration>,
    )> {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (refresh_tx, mut seek_rx) = tokio::sync::mpsc::unbounded_channel::<Duration>();
        let player_id = self.player.lock().unwrap().identity().to_string();
        std::thread::spawn(move || {
            let player = mpris::PlayerFinder::new()
                .expect("Could not connect to D-Bus")
                .find_by_name(&player_id)
                .unwrap_or_else(|err| {
                    log::warn!("error finding player: {:?}", err);
                    panic!()
                });
            let mut prog_tracker = player
                .track_progress(interval.as_millis() as u32)
                .unwrap_or_else(|err| {
                    log::warn!("error getting progress tracker: {:?}", err);
                    panic!()
                });

            let mut last_refresh = Instant::now();
            let tick = prog_tracker.tick();
            let mut progress = MprisProgress::from(tick);
            loop {
                // FIXME it's too convoluted
                let mut refresh = false;
                if interval.saturating_sub(last_refresh.elapsed()).is_zero() {
                    last_refresh = Instant::now();
                    refresh = true;
                    prog_tracker
                        .force_refresh()
                        .expect("failed to refresh player");
                }
                let tick = prog_tracker.tick();
                if tick.player_quit {
                    event_tx.send(MprisProgressEvent::PlayerQuit).unwrap();
                }
                if refresh || tick.progress_changed {
                    progress = MprisProgress::from(tick);
                }
                if progress.playback_status == PlaybackStatus::Playing || progress.progress_changed
                {
                    if let Ok(val) = seek_rx.try_recv() {
                        progress.position = val;
                        last_refresh = Instant::now().checked_add(interval).unwrap();
                    }
                    if refresh {
                        progress.can_go_next =
                            player.can_go_next().expect("failed to reach player");
                        progress.can_go_prev =
                            player.can_go_previous().expect("failed to reach player");
                    }
                    if event_tx
                        .send(MprisProgressEvent::Progress(progress.clone()))
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
        Ok((event_rx, refresh_tx))
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     // #[test]
//     // fn test_play_pause() {
//     //     let player=Cider2PlayerInfo::new();
//     //     player.play().unwrap();

//     //     std::thread::sleep(Duration::from_millis(500));
//     //     assert_eq!(player.get_playback_status().unwrap(), mpris::PlaybackStatus::Playing);
//     //     player.play_pause().unwrap();
//     //     std::thread::sleep(Duration::from_millis(500));
//     //     assert_eq!(player.get_playback_status().unwrap(), mpris::PlaybackStatus::Paused);
//     // }
//     #[test]
//     fn test_cider_current_song_info() {
//         let player = Cider2Player::new();
//         let current_song_info = player.get_current_song_info().await.unwrap();
//         println!("song info: {:?}", current_song_info);
//     }

//     #[test]
//     fn test_mpris_current_song_info() {
//         let player = MprisPlayer::new("cider");
//         let current_song_info = player.get_current_song_info().await.unwrap();
//         println!("song info: {:?}", current_song_info);
//     }
// }
