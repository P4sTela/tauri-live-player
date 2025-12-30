//! メディア関連の型定義

use serde::{Deserialize, Serialize};

/// メディアアイテム
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

/// メディアタイプ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Audio,
}

/// キュー
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
