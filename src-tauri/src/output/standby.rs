//! Standby pipeline for output windows
//!
//! Displays debug information using GStreamer's textoverlay when no video is playing.

use gstreamer as gst;
use gstreamer::prelude::*;
use tracing::{debug, info};

use super::native_handle::{create_video_sink_with_handle, NativeHandle};

/// Standby pipeline that shows debug info on an output window
pub struct StandbyPipeline {
    pipeline: gst::Pipeline,
    textoverlay: gst::Element,
    output_name: String,
    resolution: (u32, u32),
}

impl StandbyPipeline {
    /// Create a new standby pipeline for the given output
    pub fn new(
        output_id: &str,
        output_name: &str,
        handle: &NativeHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, gst::glib::BoolError> {
        let pipeline = gst::Pipeline::new();

        // videotestsrc with black pattern
        let src = gst::ElementFactory::make("videotestsrc")
            .property_from_str("pattern", "black")
            .property("is-live", true)
            .build()?;

        // capsfilter to set resolution
        let caps = gst::Caps::builder("video/x-raw")
            .field("width", width as i32)
            .field("height", height as i32)
            .field("framerate", gst::Fraction::new(30, 1))
            .build();

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property("caps", &caps)
            .build()?;

        // textoverlay for debug info
        let text = format!(
            "{}\n{}x{}\nWaiting for input...",
            output_name, width, height
        );

        let textoverlay = gst::ElementFactory::make("textoverlay")
            .property("text", &text)
            .property("font-desc", "Sans Bold 48")
            .property_from_str("valignment", "center")
            .property_from_str("halignment", "center")
            .property("shaded-background", true)
            .build()?;

        // Video sink with native handle
        let sink = create_video_sink_with_handle(handle)?;

        // Add elements to pipeline
        pipeline.add_many([&src, &capsfilter, &textoverlay, &sink])?;
        gst::Element::link_many([&src, &capsfilter, &textoverlay, &sink])?;

        debug!(
            "[StandbyPipeline] Created for '{}' at {}x{}",
            output_name, width, height
        );

        Ok(Self {
            pipeline,
            textoverlay,
            output_name: output_name.to_string(),
            resolution: (width, height),
        })
    }

    /// Start the standby pipeline
    pub fn start(&self) -> Result<(), gst::glib::BoolError> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| gst::glib::bool_error!("Failed to start standby pipeline"))?;
        info!("[StandbyPipeline] Started for '{}'", self.output_name);
        Ok(())
    }

    /// Stop the standby pipeline
    pub fn stop(&self) -> Result<(), gst::glib::BoolError> {
        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|_| gst::glib::bool_error!("Failed to stop standby pipeline"))?;
        info!("[StandbyPipeline] Stopped for '{}'", self.output_name);
        Ok(())
    }

    /// Update the displayed text
    pub fn set_text(&self, text: &str) {
        self.textoverlay.set_property("text", text);
    }

    /// Update debug info with current state
    pub fn update_debug_info(&self, state: &str, extra_info: Option<&str>) {
        let mut text = format!(
            "{}\n{}x{}\n{}",
            self.output_name, self.resolution.0, self.resolution.1, state
        );

        if let Some(info) = extra_info {
            text.push_str(&format!("\n{}", info));
        }

        self.set_text(&text);
    }
}

impl Drop for StandbyPipeline {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// Manager for standby pipelines across multiple outputs
pub struct StandbyManager {
    pipelines: std::collections::HashMap<String, StandbyPipeline>,
}

impl StandbyManager {
    pub fn new() -> Self {
        Self {
            pipelines: std::collections::HashMap::new(),
        }
    }

    /// Create and start a standby pipeline for an output
    pub fn create_standby(
        &mut self,
        output_id: &str,
        output_name: &str,
        handle: &NativeHandle,
        width: u32,
        height: u32,
    ) -> Result<(), gst::glib::BoolError> {
        // Stop existing pipeline if any
        if let Some(existing) = self.pipelines.remove(output_id) {
            let _ = existing.stop();
        }

        let pipeline = StandbyPipeline::new(output_id, output_name, handle, width, height)?;
        pipeline.start()?;
        self.pipelines.insert(output_id.to_string(), pipeline);

        Ok(())
    }

    /// Stop and remove a standby pipeline
    pub fn stop_standby(&mut self, output_id: &str) {
        if let Some(pipeline) = self.pipelines.remove(output_id) {
            let _ = pipeline.stop();
        }
    }

    /// Stop all standby pipelines
    pub fn stop_all(&mut self) {
        for (_, pipeline) in self.pipelines.drain() {
            let _ = pipeline.stop();
        }
    }

    /// Check if a standby pipeline exists for an output
    pub fn has_standby(&self, output_id: &str) -> bool {
        self.pipelines.contains_key(output_id)
    }

    /// Update debug info for an output
    pub fn update_info(&self, output_id: &str, state: &str, extra: Option<&str>) {
        if let Some(pipeline) = self.pipelines.get(output_id) {
            pipeline.update_debug_info(state, extra);
        }
    }
}

impl Default for StandbyManager {
    fn default() -> Self {
        Self::new()
    }
}
