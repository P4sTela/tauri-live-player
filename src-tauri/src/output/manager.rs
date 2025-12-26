use std::collections::HashMap;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::{AppError, AppResult};
use crate::types::*;

use super::native_handle::{get_native_handle, NativeHandle};

pub struct OutputManager {
    outputs: HashMap<String, OutputWindowState>,
}

/// State for an output window including native handle for GStreamer
pub struct OutputWindowState {
    pub id: String,
    pub output_type: OutputType,
    pub native_handle: Option<NativeHandle>,
    pub monitor_index: Option<usize>,
}

impl OutputManager {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    /// Create an output window and extract native handle for GStreamer
    ///
    /// # Arguments
    /// * `app` - Tauri AppHandle
    /// * `config` - Output configuration
    /// * `monitor` - Monitor info for fullscreen mode (None for windowed mode)
    pub fn create_output(
        &mut self,
        app: &AppHandle,
        config: &OutputTarget,
        monitor: Option<&MonitorInfo>,
    ) -> AppResult<Option<NativeHandle>> {
        match config.output_type {
            OutputType::Display => {
                let is_fullscreen = config.fullscreen.unwrap_or(true) && monitor.is_some();

                let window = if let Some(m) = monitor {
                    // Fullscreen mode: position on specific monitor
                    println!(
                        "[OutputManager] Creating fullscreen window on monitor {} ({}, {})",
                        m.index, m.width, m.height
                    );
                    WebviewWindowBuilder::new(
                        app,
                        format!("output_{}", config.id),
                        WebviewUrl::App("output.html".into()),
                    )
                    .title(&config.name)
                    .position(m.x as f64, m.y as f64)
                    .inner_size(m.width as f64, m.height as f64)
                    .fullscreen(true)
                    .decorations(false)
                    .always_on_top(true)
                    .build()
                    .map_err(|e| AppError::Output(format!("Failed to create window: {:?}", e)))?
                } else {
                    // Windowed mode: normal resizable window
                    println!("[OutputManager] Creating windowed output (not fullscreen)");
                    WebviewWindowBuilder::new(
                        app,
                        format!("output_{}", config.id),
                        WebviewUrl::App("output.html".into()),
                    )
                    .title(&config.name)
                    .inner_size(1280.0, 720.0)
                    .fullscreen(false)
                    .decorations(true)
                    .resizable(true)
                    .build()
                    .map_err(|e| AppError::Output(format!("Failed to create window: {:?}", e)))?
                };

                // Extract native handle for GStreamer
                let native_handle = get_native_handle(&window);

                let monitor_index = monitor.map(|m| m.index);

                if native_handle.is_some() {
                    println!(
                        "[OutputManager] Created output window '{}' (fullscreen={}, monitor={:?}) with native handle",
                        config.name, is_fullscreen, monitor_index
                    );
                } else {
                    println!(
                        "[OutputManager] Warning: Could not get native handle for output '{}'",
                        config.name
                    );
                }

                let handle_clone = native_handle.clone();

                self.outputs.insert(
                    config.id.clone(),
                    OutputWindowState {
                        id: config.id.clone(),
                        output_type: OutputType::Display,
                        native_handle,
                        monitor_index,
                    },
                );

                Ok(handle_clone)
            }
            OutputType::Ndi => {
                // NDI出力はパイプラインで処理、ウィンドウ不要
                self.outputs.insert(
                    config.id.clone(),
                    OutputWindowState {
                        id: config.id.clone(),
                        output_type: OutputType::Ndi,
                        native_handle: None,
                        monitor_index: None,
                    },
                );
                Ok(None)
            }
            OutputType::Audio => {
                // オーディオ出力もパイプラインで処理
                self.outputs.insert(
                    config.id.clone(),
                    OutputWindowState {
                        id: config.id.clone(),
                        output_type: OutputType::Audio,
                        native_handle: None,
                        monitor_index: None,
                    },
                );
                Ok(None)
            }
        }
    }

    /// Get the native handle for an output (for GStreamer sink creation)
    pub fn get_native_handle(&self, output_id: &str) -> Option<NativeHandle> {
        self.outputs.get(output_id)?.native_handle.clone()
    }

    /// Check if an output window exists
    pub fn has_output(&self, output_id: &str) -> bool {
        self.outputs.contains_key(output_id)
    }

    /// Get all open output IDs
    pub fn get_open_output_ids(&self) -> Vec<String> {
        self.outputs.keys().cloned().collect()
    }

    pub fn close_output(&mut self, app: &AppHandle, id: &str) {
        if let Some(output) = self.outputs.remove(id) {
            if output.output_type == OutputType::Display {
                if let Some(window) = app.get_webview_window(&format!("output_{}", id)) {
                    let _ = window.close();
                }
            }
        }
    }

    pub fn close_all(&mut self, app: &AppHandle) {
        for (id, output) in self.outputs.drain() {
            if output.output_type == OutputType::Display {
                if let Some(window) = app.get_webview_window(&format!("output_{}", id)) {
                    let _ = window.close();
                }
            }
        }
    }

    pub fn get_monitor_list(app: &AppHandle) -> AppResult<Vec<MonitorInfo>> {
        let primary = app
            .primary_monitor()
            .map_err(|e| AppError::Output(format!("Failed to get primary monitor: {:?}", e)))?;

        let monitors = app
            .available_monitors()
            .map_err(|e| AppError::Output(format!("Failed to get monitors: {:?}", e)))?;

        Ok(monitors
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let is_primary = primary
                    .as_ref()
                    .map(|p| p.name() == m.name())
                    .unwrap_or(false);

                MonitorInfo {
                    index: i,
                    name: m.name().map(|s| s.to_string()).unwrap_or_default(),
                    width: m.size().width,
                    height: m.size().height,
                    x: m.position().x,
                    y: m.position().y,
                    is_primary,
                }
            })
            .collect())
    }
}

impl Default for OutputManager {
    fn default() -> Self {
        Self::new()
    }
}
