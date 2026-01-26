pub mod media_controls;
pub mod player;
pub mod state;

pub use media_controls::MediaCmd;
#[cfg(target_os = "windows")]
pub use media_controls::MediaControlService;
pub use player::AudioPlayer;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnreleasedTrack {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub duration_secs: f64,
    pub thumbnail_url: Option<String>,
    pub content_type: String,
    pub channel_name: Option<String>,
    pub view_count: Option<u64>,
    pub added_at: Option<i64>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SearchFilter {
    pub query: String,
    pub content_type: Option<String>,
    pub max_results: Option<u32>,
}
