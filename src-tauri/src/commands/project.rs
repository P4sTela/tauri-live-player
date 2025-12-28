use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn load_project(state: State<'_, AppState>, path: String) -> Result<Project, String> {
    let path = PathBuf::from(path);

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let project: Project = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    *state.project.lock() = Some(project.clone());

    Ok(project)
}

#[tauri::command]
pub async fn save_project(state: State<'_, AppState>, path: Option<String>) -> Result<(), String> {
    let project_guard = state.project.lock();
    let project = project_guard
        .as_ref()
        .ok_or_else(|| "No project to save".to_string())?;

    let path = path.ok_or_else(|| "No path specified".to_string())?;

    let content = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;

    std::fs::write(path, content).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn new_project(state: State<'_, AppState>, name: String) -> Result<Project, String> {
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        master_brightness: 100.0,
        master_volume: 100.0,
        outputs: Vec::new(),
        cues: Vec::new(),
        settings: ProjectSettings::default(),
    };

    *state.project.lock() = Some(project.clone());

    Ok(project)
}

#[tauri::command]
pub async fn get_project(state: State<'_, AppState>) -> Result<Option<Project>, String> {
    Ok(state.project.lock().clone())
}

#[tauri::command]
pub async fn update_project(state: State<'_, AppState>, project: Project) -> Result<(), String> {
    *state.project.lock() = Some(project);
    Ok(())
}
