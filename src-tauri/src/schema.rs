
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaKind {
    Mp4,
    M3u8,
    Ts,
    Dash,
    
    Audio,
    Other,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub source: String,
    pub kind: MediaKind,
    pub quality: String,
    pub size_bytes: i64,
    pub status: String,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Merging,
    Done,
    Failed,
    Paused,
    Cancelled,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub title: String,
    pub out_path: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub speed_bps: f64,
    pub downloaded: i64,
    pub total: i64,
    pub error: String,
    
    pub format: Option<String>,
    
    #[serde(default)]
    pub file_path: String,
    
    
    #[serde(default)]
    pub pair_url: String,
}


#[derive(Debug, Clone, Deserialize)]
pub struct DownloadOptions {
    pub format: String,
    pub quality: String,
    pub out_dir: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub yt_dlp: bool,
    pub ffmpeg: bool,
    pub yt_dlp_version: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub addr: String,
    pub ca_installed: bool,
}
