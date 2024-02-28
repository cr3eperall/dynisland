use std::time::Duration;

use anyhow::{bail, Result};
use mpris::TrackID;

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
    pub current_playback_time: f64,
    #[serde(rename = "currentPlaybackProgress")]
    pub current_playback_progress: f32,
}
#[derive(serde::Deserialize, Debug)]
pub struct CurrentSongArtwork {
    pub url: String,
    pub width: Option<u64>,
    pub height: Option<u64>,
}

pub trait Playerinfo {
    //TODO add signals for when the song changes / progress updates
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn play_pause(&self) -> Result<()>;
    fn next(&self) -> Result<()>;
    fn previous(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn get_playback_status(&self) -> Result<mpris::PlaybackStatus>;

    fn set_shuffle(&self, shuffle: bool) -> Result<()>;
    fn get_shuffle(&self) -> Result<bool>;
    fn set_loop(&self, repeat: mpris::LoopStatus) -> Result<()>;
    fn get_loop(&self) -> Result<mpris::LoopStatus>;
    fn seek(&self, offset: i64) -> Result<()>;
    fn get_position(&self) -> Result<Duration>;
    fn set_position(&self, track_id: &str, position: Duration) -> Result<()>;
    fn get_length(&self) -> Result<Duration>;
    fn set_volume(&self, volume: f64) -> Result<()>;
    fn get_volume(&self) -> Result<f64>;
    fn get_metadata(&self) -> Result<mpris::Metadata>;
    fn get_current_track_id(&self) -> Result<TrackID>;
    fn get_current_song_info(&self) -> Result<CurrentSongMinimalInfo>;
}

pub struct MprisPlayerInfo {
    player: mpris::Player,
}

impl MprisPlayerInfo {
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

        Self { player }
    }
}
impl Playerinfo for MprisPlayerInfo {
    fn play(&self) -> Result<()> {
        self.player.play()?;
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        self.player.pause()?;
        Ok(())
    }

    fn play_pause(&self) -> Result<()> {
        self.player.play_pause()?;
        Ok(())
    }

    fn next(&self) -> Result<()> {
        self.player.next()?;
        Ok(())
    }

    fn previous(&self) -> Result<()> {
        self.player.previous()?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.player.stop()?;
        Ok(())
    }
    fn get_playback_status(&self) -> Result<mpris::PlaybackStatus> {
        let playback_status = self.player.get_playback_status()?;
        Ok(playback_status)
    }

    fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.player.set_shuffle(shuffle)?;
        Ok(())
    }

    fn get_shuffle(&self) -> Result<bool> {
        let shuffle = self.player.get_shuffle()?;
        Ok(shuffle)
    }

    fn set_loop(&self, repeat: mpris::LoopStatus) -> Result<()> {
        self.player.set_loop_status(repeat)?;
        Ok(())
    }

    fn get_loop(&self) -> Result<mpris::LoopStatus> {
        let loop_status = self.player.get_loop_status()?;
        Ok(loop_status)
    }

    fn seek(&self, offset: i64) -> Result<()> {
        self.player.seek(offset * 1_000_000)?;
        Ok(())
    }

    fn get_position(&self) -> Result<Duration> {
        let position = self.player.get_position()?;
        Ok(position)
    }

    fn set_position(&self, track_id: &str, position: Duration) -> Result<()> {
        let track_id = match mpris::TrackID::new(track_id) {
            Ok(track_id) => track_id,
            Err(err) => bail!("error creating track id: {:?}", err),
        };
        self.player.set_position(track_id, &position)?;
        Ok(())
    }
    fn get_length(&self) -> Result<Duration> {
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

    fn set_volume(&self, volume: f64) -> Result<()> {
        self.player.set_volume(volume)?;
        Ok(())
    }

    fn get_volume(&self) -> Result<f64> {
        let volume = self.player.get_volume()?;
        Ok(volume)
    }

    fn get_metadata(&self) -> Result<mpris::Metadata> {
        let metadata = self.player.get_metadata()?;
        Ok(metadata)
    }
    fn get_current_track_id(&self) -> Result<TrackID> {
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
    fn get_current_song_info(&self) -> Result<CurrentSongMinimalInfo> {
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
}

pub const CIDER2_API_URL: &str = "http://localhost:10769";
pub struct Cider2PlayerInfo {
    mpris_player: mpris::Player,
    client: reqwest::blocking::Client,
}

impl Cider2PlayerInfo {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for Cider2PlayerInfo {
    fn default() -> Self {
        let player = mpris::PlayerFinder::new()
            .expect("Could not connect to D-Bus")
            .find_by_name("cider2")
            .unwrap();

        Self {
            mpris_player: player,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Playerinfo for Cider2PlayerInfo {
    fn play(&self) -> Result<()> {
        self.mpris_player.play()?;
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        self.mpris_player.pause()?;
        Ok(())
    }

    fn play_pause(&self) -> Result<()> {
        self.mpris_player.play_pause()?;
        Ok(())
    }

    fn next(&self) -> Result<()> {
        self.mpris_player.next()?;
        Ok(())
    }

    fn previous(&self) -> Result<()> {
        self.mpris_player.previous()?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.mpris_player.stop()?;
        Ok(())
    }

    fn get_playback_status(&self) -> Result<mpris::PlaybackStatus> {
        let playback_status = self.mpris_player.get_playback_status()?;
        Ok(playback_status)
    }

    fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.mpris_player.set_shuffle(shuffle)?;
        Ok(())
    }

    fn get_shuffle(&self) -> Result<bool> {
        let shuffle = self.mpris_player.get_shuffle()?;
        Ok(shuffle)
    }

    fn set_loop(&self, repeat: mpris::LoopStatus) -> Result<()> {
        self.mpris_player.set_loop_status(repeat)?;
        Ok(())
    }

    fn get_loop(&self) -> Result<mpris::LoopStatus> {
        let loop_status = self.mpris_player.get_loop_status()?;
        Ok(loop_status)
    }

    fn seek(&self, offset: i64) -> Result<()> {
        let current_position = self.get_position()?;
        let length = self.get_length()?;
        let new_position = if offset.is_negative() {
            current_position.saturating_sub(Duration::from_secs(offset.unsigned_abs()))
        } else {
            current_position
                .saturating_add(Duration::from_secs(offset as u64))
                .min(length)
        };
        let response = self
            .client
            .get(format!(
                "{CIDER2_API_URL}/seekto/{}",
                new_position.as_secs()
            ))
            .send()?;
        if matches!(response.status(), reqwest::StatusCode::NO_CONTENT) {
            Ok(())
        } else {
            bail!("error seeking: {:?}", response)
        }
    }

    fn get_position(&self) -> Result<Duration> {
        let position = self.get_current_song_info()?.current_playback_time;
        Ok(Duration::from_secs_f64(position))
    }

    fn set_position(&self, track_id: &str, position: Duration) -> Result<()> {
        //TODO add fallback to cider2 api
        let track_id = match mpris::TrackID::new(track_id) {
            Ok(track_id) => track_id,
            Err(err) => bail!("error creating track id: {:?}", err),
        };
        self.mpris_player.set_position(track_id, &position)?;
        Ok(())
    }

    fn get_length(&self) -> Result<Duration> {
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

    fn set_volume(&self, volume: f64) -> Result<()> {
        self.mpris_player.set_volume(volume)?;
        Ok(())
    }

    fn get_volume(&self) -> Result<f64> {
        let volume = self.mpris_player.get_volume()?;
        Ok(volume)
    }

    fn get_metadata(&self) -> Result<mpris::Metadata> {
        let metadata = self.mpris_player.get_metadata()?;
        Ok(metadata)
    }
    fn get_current_track_id(&self) -> Result<TrackID> {
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
    fn get_current_song_info(&self) -> Result<CurrentSongMinimalInfo> {
        let response = self
            .client
            .get(format!("{CIDER2_API_URL}/currentPlayingSong"))
            .send()?;
        let current_song: CurrentSongMinimal = serde_json::from_str(&response.text()?)?;
        Ok(current_song.info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_play_pause() {
    //     let player=Cider2PlayerInfo::new();
    //     player.play().unwrap();

    //     std::thread::sleep(Duration::from_millis(500));
    //     assert_eq!(player.get_playback_status().unwrap(), mpris::PlaybackStatus::Playing);
    //     player.play_pause().unwrap();
    //     std::thread::sleep(Duration::from_millis(500));
    //     assert_eq!(player.get_playback_status().unwrap(), mpris::PlaybackStatus::Paused);
    // }
    #[test]
    fn test_cider_current_song_info() {
        let player = Cider2PlayerInfo::new();
        let current_song_info = player.get_current_song_info().unwrap();
        println!("song info: {:?}", current_song_info);
    }

    #[test]
    fn test_mpris_current_song_info() {
        let player = MprisPlayerInfo::new("cider2");
        let current_song_info = player.get_current_song_info().unwrap();
        println!("song info: {:?}", current_song_info);
    }
}
