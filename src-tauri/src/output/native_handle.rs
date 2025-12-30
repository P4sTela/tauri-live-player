//! Platform-specific native window handle extraction and GStreamer sink creation
//!
//! This module provides utilities to extract native window handles from Tauri windows
//! and create GStreamer video sinks that render directly to those windows.

use gstreamer as gst;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::WebviewWindow;
use tracing::{debug, warn};

/// Platform-agnostic native window handle
#[derive(Debug, Clone)]
pub enum NativeHandle {
    #[cfg(target_os = "macos")]
    /// NSView pointer on macOS
    NSView(u64),
    #[cfg(target_os = "windows")]
    /// HWND on Windows
    Hwnd(isize),
    #[cfg(target_os = "linux")]
    /// X11 window ID or Wayland surface (simplified as u64)
    X11Window(u64),
}

// Make NativeHandle Send + Sync safe
// The actual pointers are just numeric values used by GStreamer
unsafe impl Send for NativeHandle {}
unsafe impl Sync for NativeHandle {}

/// Extract native window handle from a Tauri WebviewWindow
pub fn get_native_handle(window: &WebviewWindow) -> Option<NativeHandle> {
    let handle = window.window_handle().ok()?;

    match handle.as_raw() {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(appkit) => {
            let ns_view = appkit.ns_view.as_ptr() as u64;
            debug!("[NativeHandle] macOS NSView handle: 0x{:x}", ns_view);
            Some(NativeHandle::NSView(ns_view))
        }

        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(win32) => {
            let hwnd = win32.hwnd.get() as isize;
            debug!("[NativeHandle] Windows HWND handle: 0x{:x}", hwnd);
            Some(NativeHandle::Hwnd(hwnd))
        }

        #[cfg(target_os = "linux")]
        RawWindowHandle::Xlib(xlib) => {
            let window_id = xlib.window;
            debug!("[NativeHandle] Linux X11 window ID: {}", window_id);
            Some(NativeHandle::X11Window(window_id as u64))
        }

        #[cfg(target_os = "linux")]
        RawWindowHandle::Xcb(xcb) => {
            let window_id = xcb.window.get();
            debug!("[NativeHandle] Linux XCB window ID: {}", window_id);
            Some(NativeHandle::X11Window(window_id as u64))
        }

        _ => {
            warn!("[NativeHandle] Unsupported window handle type");
            None
        }
    }
}

/// Create a platform-specific GStreamer video sink with the given native handle
pub fn create_video_sink_with_handle(
    handle: &NativeHandle,
) -> Result<gst::Element, gst::glib::BoolError> {
    match handle {
        #[cfg(target_os = "macos")]
        NativeHandle::NSView(ns_view) => create_macos_sink(*ns_view),

        #[cfg(target_os = "windows")]
        NativeHandle::Hwnd(hwnd) => create_windows_sink(*hwnd),

        #[cfg(target_os = "linux")]
        NativeHandle::X11Window(window_id) => create_linux_sink(*window_id),

        #[allow(unreachable_patterns)]
        _ => Err(gst::glib::bool_error!(
            "Unsupported platform for native handle sink"
        )),
    }
}

#[cfg(target_os = "macos")]
fn create_macos_sink(ns_view: u64) -> Result<gst::Element, gst::glib::BoolError> {
    // Try glimagesink first (OpenGL-based, better performance)
    if let Ok(sink) = gst::ElementFactory::make("glimagesink").build() {
        // Note: glimagesink on macOS expects the window-handle property
        // to be set via the GstVideoOverlay interface
        set_window_handle_overlay(&sink, ns_view);
        debug!(
            "[NativeHandle] Created glimagesink for macOS with handle 0x{:x}",
            ns_view
        );
        return Ok(sink);
    }

    // Fallback to osxvideosink
    if let Ok(sink) = gst::ElementFactory::make("osxvideosink").build() {
        set_window_handle_overlay(&sink, ns_view);
        debug!(
            "[NativeHandle] Created osxvideosink for macOS with handle 0x{:x}",
            ns_view
        );
        return Ok(sink);
    }

    Err(gst::glib::bool_error!(
        "No suitable video sink found for macOS (tried glimagesink, osxvideosink)"
    ))
}

#[cfg(target_os = "windows")]
fn create_windows_sink(hwnd: isize) -> Result<gst::Element, gst::glib::BoolError> {
    // Try d3d11videosink first (Direct3D 11, best performance on Windows)
    if let Ok(sink) = gst::ElementFactory::make("d3d11videosink").build() {
        set_window_handle_overlay(&sink, hwnd as u64);
        debug!(
            "[NativeHandle] Created d3d11videosink for Windows with HWND 0x{:x}",
            hwnd
        );
        return Ok(sink);
    }

    // Fallback to d3dvideosink (Direct3D 9)
    if let Ok(sink) = gst::ElementFactory::make("d3dvideosink").build() {
        set_window_handle_overlay(&sink, hwnd as u64);
        debug!(
            "[NativeHandle] Created d3dvideosink for Windows with HWND 0x{:x}",
            hwnd
        );
        return Ok(sink);
    }

    // Fallback to glimagesink
    if let Ok(sink) = gst::ElementFactory::make("glimagesink").build() {
        set_window_handle_overlay(&sink, hwnd as u64);
        debug!(
            "[NativeHandle] Created glimagesink for Windows with HWND 0x{:x}",
            hwnd
        );
        return Ok(sink);
    }

    Err(gst::glib::bool_error!(
        "No suitable video sink found for Windows (tried d3d11videosink, d3dvideosink, glimagesink)"
    ))
}

#[cfg(target_os = "linux")]
fn create_linux_sink(window_id: u64) -> Result<gst::Element, gst::glib::BoolError> {
    // Try glimagesink first
    if let Ok(sink) = gst::ElementFactory::make("glimagesink").build() {
        set_window_handle_overlay(&sink, window_id);
        debug!(
            "[NativeHandle] Created glimagesink for Linux with window ID {}",
            window_id
        );
        return Ok(sink);
    }

    // Fallback to xvimagesink
    if let Ok(sink) = gst::ElementFactory::make("xvimagesink").build() {
        set_window_handle_overlay(&sink, window_id);
        debug!(
            "[NativeHandle] Created xvimagesink for Linux with window ID {}",
            window_id
        );
        return Ok(sink);
    }

    // Fallback to ximagesink
    if let Ok(sink) = gst::ElementFactory::make("ximagesink").build() {
        set_window_handle_overlay(&sink, window_id);
        debug!(
            "[NativeHandle] Created ximagesink for Linux with window ID {}",
            window_id
        );
        return Ok(sink);
    }

    Err(gst::glib::bool_error!(
        "No suitable video sink found for Linux (tried glimagesink, xvimagesink, ximagesink)"
    ))
}

/// Set the window handle on a GStreamer element using the GstVideoOverlay interface
fn set_window_handle_overlay(element: &gst::Element, handle: u64) {
    use gstreamer_video::prelude::*;

    // Try to get the VideoOverlay interface
    if let Some(overlay) = element.dynamic_cast_ref::<gstreamer_video::VideoOverlay>() {
        unsafe {
            overlay.set_window_handle(handle as usize);
        }
        debug!(
            "[NativeHandle] Set window handle via VideoOverlay: 0x{:x}",
            handle
        );
    } else {
        // Some sinks might use a property instead
        if element.has_property("window-handle", None) {
            element.set_property("window-handle", handle);
            debug!(
                "[NativeHandle] Set window handle via property: 0x{:x}",
                handle
            );
        } else {
            warn!("[NativeHandle] Could not set window handle on element");
        }
    }
}

/// Create a fallback video sink (autovideosink) when native handle is not available
pub fn create_fallback_sink() -> Result<gst::Element, gst::glib::BoolError> {
    debug!("[NativeHandle] Creating fallback autovideosink");
    gst::ElementFactory::make("autovideosink").build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_handle_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NativeHandle>();
    }
}
