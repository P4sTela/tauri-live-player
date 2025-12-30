//! 出力先関連の型定義

use serde::{Deserialize, Serialize};

/// 出力先
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

    // Syphon (macOS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syphon_name: Option<String>,

    // Spout (Windows)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spout_name: Option<String>,
}

/// 出力タイプ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Display,
    Ndi,
    Audio,
    /// macOS: Syphon GPU texture sharing
    Syphon,
    /// Windows: Spout GPU texture sharing
    Spout,
}

/// オーディオドライバ
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

/// モニター情報
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

/// NDIソース情報
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NdiSource {
    pub name: String,
    pub url_address: String,
}
