use tauri::{AppHandle, State};

use crate::audio::sink::{list_asio_devices, AsioDevice};
use crate::output::manager::OutputManager;
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn get_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    OutputManager::get_monitor_list(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_output_window(
    state: State<'_, AppState>,
    app: AppHandle,
    config: OutputTarget,
) -> Result<(), String> {
    let mut manager = state.output_manager.lock();
    manager
        .create_output(&app, &config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_output_window(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut manager = state.output_manager.lock();
    manager.close_output(&app, &id);
    Ok(())
}

#[tauri::command]
pub async fn close_all_outputs(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let mut manager = state.output_manager.lock();
    manager.close_all(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_asio_devices() -> Result<Vec<AsioDevice>, String> {
    Ok(list_asio_devices())
}
