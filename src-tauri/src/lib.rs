// 開発中は未使用の警告を抑制
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod audio;
mod commands;
mod error;
mod output;
mod pipeline;
mod state;
mod types;

use state::AppState;
use tauri::Manager;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize tracing/logging
fn init_logging() {
    // RUST_LOG env controls log level: error, warn, info, debug, trace
    // Example: RUST_LOG=tauri_live_player=debug,gstreamer=warn
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}

/// Set up NDI SDK library path for macOS
#[cfg(target_os = "macos")]
fn setup_ndi_library_path() {
    use std::env;

    // NDI SDK library is typically installed at /usr/local/lib
    // GStreamer NDI plugin uses NDI_RUNTIME_DIR_V6 to find the SDK
    let ndi_lib_path = "/usr/local/lib";

    // Set NDI_RUNTIME_DIR_V6 for GStreamer NDI plugin
    if env::var("NDI_RUNTIME_DIR_V6").is_err() {
        env::set_var("NDI_RUNTIME_DIR_V6", ndi_lib_path);
        info!(path = %ndi_lib_path, "Set NDI_RUNTIME_DIR_V6 for GStreamer NDI plugin");
    }
}

#[cfg(not(target_os = "macos"))]
fn setup_ndi_library_path() {
    // On Windows/Linux, NDI SDK is typically in system path or needs different handling
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging first
    init_logging();

    // Set up NDI library path before GStreamer initialization
    setup_ndi_library_path();

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
            } else {
                println!("GStreamer initialized successfully");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Player
            commands::player::play_test_video,
            commands::player::load_cue,
            commands::player::play,
            commands::player::pause,
            commands::player::stop,
            commands::player::seek,
            commands::player::set_master_brightness,
            commands::player::set_master_volume,
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
