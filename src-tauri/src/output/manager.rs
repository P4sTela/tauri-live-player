use std::collections::HashMap;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::{AppError, AppResult};
use crate::types::*;

pub struct OutputManager {
    outputs: HashMap<String, OutputWindow>,
}

struct OutputWindow {
    id: String,
    output_type: OutputType,
}

impl OutputManager {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    pub fn create_output(&mut self, app: &AppHandle, config: &OutputTarget) -> AppResult<()> {
        match config.output_type {
            OutputType::Display => {
                let monitors = app
                    .available_monitors()
                    .map_err(|e| AppError::Output(format!("Failed to get monitors: {:?}", e)))?;

                let monitor = monitors
                    .get(config.display_index.unwrap_or(0))
                    .ok_or_else(|| AppError::NotFound("Monitor not found".to_string()))?;

                let position = monitor.position();
                let size = monitor.size();

                let _window = WebviewWindowBuilder::new(
                    app,
                    format!("output_{}", config.id),
                    WebviewUrl::App("output.html".into()),
                )
                .title(&config.name)
                .position(position.x as f64, position.y as f64)
                .inner_size(size.width as f64, size.height as f64)
                .fullscreen(config.fullscreen.unwrap_or(true))
                .decorations(false)
                .always_on_top(true)
                .build()
                .map_err(|e| AppError::Output(format!("Failed to create window: {:?}", e)))?;

                self.outputs.insert(
                    config.id.clone(),
                    OutputWindow {
                        id: config.id.clone(),
                        output_type: OutputType::Display,
                    },
                );
            }
            OutputType::Ndi => {
                // NDI出力はパイプラインで処理、ウィンドウ不要
                self.outputs.insert(
                    config.id.clone(),
                    OutputWindow {
                        id: config.id.clone(),
                        output_type: OutputType::Ndi,
                    },
                );
            }
            OutputType::Audio => {
                // オーディオ出力もパイプラインで処理
                self.outputs.insert(
                    config.id.clone(),
                    OutputWindow {
                        id: config.id.clone(),
                        output_type: OutputType::Audio,
                    },
                );
            }
        }

        Ok(())
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
