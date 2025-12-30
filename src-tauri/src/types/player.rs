//! プレイヤー状態関連の型定義

use serde::{Deserialize, Serialize};

/// プレイヤーステータス
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

/// プレイヤー状態
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
