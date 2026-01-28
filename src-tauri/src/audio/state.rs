use serde::{Deserialize, Serialize};

/// Current state of the audio player
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Information about the currently playing track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: f64,
    pub cover_image: Option<String>,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
}

impl Default for TrackInfo {
    fn default() -> Self {
        Self {
            path: String::new(),
            title: String::from("Unknown"),
            artist: String::from("Unknown Artist"),
            album: String::from("Unknown Album"),
            duration_secs: 0.0,
            cover_image: None,
            disc_number: None,
            track_number: None,
        }
    }
}

/// Complete player status for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatus {
    pub state: PlayerState,
    pub track: Option<TrackInfo>,
    pub position_secs: f64,
    pub volume: f32,
}

impl Default for PlayerStatus {
    fn default() -> Self {
        Self {
            state: PlayerState::Stopped,
            track: None,
            position_secs: 0.0,
            volume: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub query: String,
    pub content_type: Option<String>,
    pub max_results: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnreleasedTrack {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub duration_secs: f64,
    pub thumbnail_url: Option<String>,
    pub content_type: String,
    pub channel_name: Option<String>,
    pub view_count: Option<u64>,
    pub added_at: Option<String>,
}
