use std::collections::HashMap;
use tauri::{AppHandle, State};

use crate::output::manager::OutputManager;
use crate::output::native_handle::NativeHandle;
use crate::state::AppState;
use crate::types::*;

/// テスト用: 単一のビデオファイルを直接再生
#[tauri::command]
pub async fn play_test_video(
    state: State<'_, AppState>,
    app: AppHandle,
    path: String,
) -> Result<(), String> {
    // モニター一覧を取得
    let monitors = OutputManager::get_monitor_list(&app).map_err(|e| e.to_string())?;

    let mut player_guard = state.player.lock();
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "Player not initialized".to_string())?;

    // テスト用のCueを作成
    let test_cue = Cue {
        id: "test".to_string(),
        name: "Test Video".to_string(),
        items: vec![MediaItem {
            id: "test-item".to_string(),
            name: "Test".to_string(),
            path: path.clone(),
            output_id: "display-1".to_string(),
            media_type: MediaType::Video,
            offset: None,
            trim_start: None,
            trim_end: None,
        }],
        duration: 0.0,
        loop_playback: false,
        auto_advance: false,
        color: None,
    };

    // テスト用の出力を作成
    let test_output = OutputTarget {
        id: "display-1".to_string(),
        name: "Test Display".to_string(),
        output_type: OutputType::Display,
        brightness: None,
        display_index: Some(0),
        fullscreen: Some(false),
        ndi_name: None,
        syphon_name: None,
        spout_name: None,
        audio_driver: None,
        audio_device: None,
        audio_channels: None,
    };

    // ネイティブハンドルを取得（テスト用は空のマップ）
    let native_handles: HashMap<String, NativeHandle> = HashMap::new();

    player
        .load_cue(&test_cue, &[test_output], &monitors, &native_handles)
        .map_err(|e| e.to_string())?;

    player.play().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn load_cue(
    state: State<'_, AppState>,
    app: AppHandle,
    cue_index: usize,
) -> Result<(), String> {
    // モニター一覧を取得
    let monitors = OutputManager::get_monitor_list(&app).map_err(|e| e.to_string())?;

    // OutputManagerからネイティブハンドルを収集
    let native_handles: HashMap<String, NativeHandle> = {
        let output_manager = state.output_manager.lock();
        let open_ids = output_manager.get_open_output_ids();
        tracing::debug!(
            "[load_cue] Open output IDs in OutputManager: {:?}",
            open_ids
        );

        let handles: HashMap<String, NativeHandle> = open_ids
            .iter()
            .filter_map(|id| {
                output_manager
                    .get_native_handle(id)
                    .map(|handle| (id.clone(), handle))
            })
            .collect();

        tracing::debug!(
            "[load_cue] Native handles collected: {:?}",
            handles.keys().collect::<Vec<_>>()
        );
        handles
    };

    let mut player_guard = state.player.lock();
    let project_guard = state.project.lock();

    let project = project_guard
        .as_ref()
        .ok_or_else(|| "No project loaded".to_string())?;

    let cue = project
        .cues
        .get(cue_index)
        .ok_or_else(|| "Cue not found".to_string())?;

    let player = player_guard
        .as_mut()
        .ok_or_else(|| "Player not initialized".to_string())?;

    player
        .load_cue(cue, &project.outputs, &monitors, &native_handles)
        .map_err(|e| e.to_string())?;

    *state.current_cue_index.lock() = cue_index as i32;

    Ok(())
}

#[tauri::command]
pub async fn play(state: State<'_, AppState>) -> Result<(), String> {
    let player_guard = state.player.lock();
    let player = player_guard
        .as_ref()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.play().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>) -> Result<(), String> {
    let player_guard = state.player.lock();
    let player = player_guard
        .as_ref()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.pause().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop(state: State<'_, AppState>) -> Result<(), String> {
    let player_guard = state.player.lock();
    let player = player_guard
        .as_ref()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.stop().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    let player_guard = state.player.lock();
    let player = player_guard
        .as_ref()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.seek(position).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_master_brightness(state: State<'_, AppState>, value: f64) -> Result<(), String> {
    let mut player_guard = state.player.lock();
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.set_master_brightness(value);

    // プロジェクトの値も更新
    if let Some(project) = state.project.lock().as_mut() {
        project.master_brightness = value;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_output_brightness(
    state: State<'_, AppState>,
    output_id: String,
    value: Option<f64>,
) -> Result<(), String> {
    let mut player_guard = state.player.lock();
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.set_output_brightness(&output_id, value);

    // プロジェクトの値も更新
    if let Some(project) = state.project.lock().as_mut() {
        if let Some(output) = project.outputs.iter_mut().find(|o| o.id == output_id) {
            output.brightness = value;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn set_master_volume(state: State<'_, AppState>, value: f64) -> Result<(), String> {
    let mut player_guard = state.player.lock();
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "Player not initialized".to_string())?;
    player.set_master_volume(value);

    // プロジェクトの値も更新
    if let Some(project) = state.project.lock().as_mut() {
        project.master_volume = value;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_player_state(state: State<'_, AppState>) -> Result<PlayerState, String> {
    let player_guard = state.player.lock();

    let (status, current_time, duration) = match player_guard.as_ref() {
        Some(player) => {
            let status = match player.state() {
                gstreamer::State::Null => PlayerStatus::Idle,
                gstreamer::State::Ready => PlayerStatus::Ready,
                gstreamer::State::Paused => PlayerStatus::Paused,
                gstreamer::State::Playing => PlayerStatus::Playing,
                _ => PlayerStatus::Idle,
            };
            let pos = player.position().unwrap_or(0.0);
            let dur = player.duration().unwrap_or(0.0);
            // デバッグ: 最初の数回だけログ出力（高頻度ポーリングなので）
            static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 20 || count % 100 == 0 {
                tracing::debug!(
                    "get_player_state: status={:?}, position={:.3}s, duration={:.3}s",
                    status,
                    pos,
                    dur
                );
            }
            (status, pos, dur)
        }
        None => (PlayerStatus::Idle, 0.0, 0.0),
    };

    let current_cue_index = *state.current_cue_index.lock();

    Ok(PlayerState {
        status,
        current_cue_index,
        current_time,
        duration,
        error: None,
    })
}
