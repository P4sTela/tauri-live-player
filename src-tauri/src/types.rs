use serde::{Deserialize, Serialize};

// ========================================
// メディアアイテム
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub name: String,
    pub path: String,
    pub output_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_end: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Audio,
}

// ========================================
// キュー
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    pub name: String,
    pub items: Vec<MediaItem>,
    pub duration: f64,
    #[serde(rename = "loop")]
    pub loop_playback: bool,
    pub auto_advance: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

// ========================================
// 出力先
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTarget {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: OutputType,

    // 明るさ (None = Master連動)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f64>,

    // Display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fullscreen: Option<bool>,

    // NDI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndi_name: Option<String>,

    // Audio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_driver: Option<AudioDriver>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Display,
    Ndi,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioDriver {
    Auto,
    Asio,
    Wasapi,
    #[serde(rename = "coreaudio")]
    CoreAudio,
    Jack,
    Alsa,
}

// ========================================
// プロジェクト設定
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub default_brightness: f64,
    pub auto_save: bool,
    pub preview_quality: PreviewQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewQuality {
    Low,
    Medium,
    High,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            default_brightness: 100.0,
            auto_save: true,
            preview_quality: PreviewQuality::Medium,
        }
    }
}

// ========================================
// プロジェクト
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub master_brightness: f64,
    pub outputs: Vec<OutputTarget>,
    pub cues: Vec<Cue>,
    #[serde(default)]
    pub settings: ProjectSettings,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Untitled Project".to_string(),
            master_brightness: 100.0,
            outputs: Vec::new(),
            cues: Vec::new(),
            settings: ProjectSettings::default(),
        }
    }
}

// ========================================
// プレイヤー状態
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerStatus {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: PlayerStatus,
    pub current_cue_index: i32,
    pub current_time: f64,
    pub duration: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            status: PlayerStatus::Idle,
            current_cue_index: -1,
            current_time: 0.0,
            duration: 0.0,
            error: None,
        }
    }
}

// ========================================
// モニター情報
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

// ========================================
// NDIソース情報
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NdiSource {
    pub name: String,
    pub url_address: String,
}
