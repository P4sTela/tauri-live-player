mod audio;
mod commands;
mod error;
mod output;
mod pipeline;
mod state;
mod types;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .setup(|app| {
            // GStreamer初期化
            let state = app.state::<AppState>();
            if let Err(e) = state.init_player() {
                eprintln!("Failed to initialize GStreamer: {:?}", e);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Player
            commands::player::load_cue,
            commands::player::play,
            commands::player::pause,
            commands::player::stop,
            commands::player::seek,
            commands::player::set_master_brightness,
            commands::player::set_output_brightness,
            commands::player::get_player_state,
            // Output
            commands::output::get_monitors,
            commands::output::open_output_window,
            commands::output::close_output_window,
            commands::output::close_all_outputs,
            commands::output::get_asio_devices,
            // Project
            commands::project::load_project,
            commands::project::save_project,
            commands::project::new_project,
            commands::project::get_project,
            commands::project::update_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
