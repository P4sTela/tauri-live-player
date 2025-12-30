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
    monitor: Option<MonitorInfo>,
) -> Result<(), String> {
    // Create the output window
    let native_handle = {
        let mut manager = state.output_manager.lock();
        manager
            .create_output(&app, &config, monitor.as_ref())
            .map_err(|e| e.to_string())?
    };

    // Start standby pipeline if we have a native handle
    if let Some(handle) = native_handle {
        let (width, height) = if let Some(ref m) = monitor {
            (m.width, m.height)
        } else {
            (1280, 720) // Default windowed size
        };

        let mut standby = state.standby_manager.lock();
        if let Err(e) = standby.create_standby(&config.id, &config.name, &handle, width, height) {
            tracing::warn!("[Output] Failed to create standby pipeline: {:?}", e);
            // Non-fatal, continue without standby
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn close_output_window(
    state: State<'_, AppState>,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    // Stop standby pipeline first
    {
        let mut standby = state.standby_manager.lock();
        standby.stop_standby(&id);
    }

    // Then close the window
    let mut manager = state.output_manager.lock();
    manager.close_output(&app, &id);
    Ok(())
}

#[tauri::command]
pub async fn close_all_outputs(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    // Stop all standby pipelines
    {
        let mut standby = state.standby_manager.lock();
        standby.stop_all();
    }

    // Close all windows
    let mut manager = state.output_manager.lock();
    manager.close_all(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_asio_devices() -> Result<Vec<AsioDevice>, String> {
    Ok(list_asio_devices())
}
