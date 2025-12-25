use tauri::State;

use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn load_cue(state: State<'_, AppState>, cue_index: usize) -> Result<(), String> {
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
        .load_cue(cue, &project.outputs)
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
            (
                status,
                player.position().unwrap_or(0.0),
                player.duration().unwrap_or(0.0),
            )
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
