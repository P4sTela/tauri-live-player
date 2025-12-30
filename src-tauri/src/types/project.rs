//! プロジェクト関連の型定義

use serde::{Deserialize, Serialize};

use super::media::Cue;
use super::output::OutputTarget;

/// プロジェクト設定
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettings {
    pub default_brightness: f64,
    pub auto_save: bool,
    pub preview_quality: PreviewQuality,
}

/// プレビュー品質
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

/// プロジェクト
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub master_brightness: f64,
    #[serde(default = "default_volume")]
    pub master_volume: f64,
    pub outputs: Vec<OutputTarget>,
    pub cues: Vec<Cue>,
    #[serde(default)]
    pub settings: ProjectSettings,
}

fn default_volume() -> f64 {
    100.0
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Untitled Project".to_string(),
            master_brightness: 100.0,
            master_volume: 100.0,
            outputs: Vec::new(),
            cues: Vec::new(),
            settings: ProjectSettings::default(),
        }
    }
}
