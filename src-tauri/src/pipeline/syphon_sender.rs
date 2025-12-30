//! Syphon output sender for macOS
//!
//! Uses appsink + Syphon SDK via objc2 to share video frames with other applications
//! on the same machine via GPU texture sharing.

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Once};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2_foundation::{NSBundle, NSDictionary, NSPoint, NSRect, NSSize, NSString};
use parking_lot::Mutex;
use tracing::{debug, error, info};

use crate::error::AppResult;

// OpenGL types for legacy Syphon (OpenGL-based)
type CGLContextObj = *mut c_void;

#[link(name = "OpenGL", kind = "framework")]
extern "C" {
    fn CGLChoosePixelFormat(attribs: *const i32, pix: *mut *mut c_void, npix: *mut i32) -> i32;
    fn CGLCreateContext(pix: *mut c_void, share: CGLContextObj, ctx: *mut CGLContextObj) -> i32;
    fn CGLDestroyContext(ctx: CGLContextObj) -> i32;
    fn CGLDestroyPixelFormat(pix: *mut c_void) -> i32;
    fn CGLSetCurrentContext(ctx: CGLContextObj) -> i32;
    fn CGLLockContext(ctx: CGLContextObj) -> i32;
    fn CGLUnlockContext(ctx: CGLContextObj) -> i32;
}

// CGL pixel format attributes
const K_CGL_PFA_ACCELERATED: i32 = 73;
const K_CGL_PFA_ALLOW_OFFLINE_RENDERERS: i32 = 96;
const K_CGL_PFA_COLOR_SIZE: i32 = 8;
const K_CGL_PFA_DOUBLE_BUFFER: i32 = 5;
const K_CGL_PFA_OPENGL_PROFILE: i32 = 99;
const K_CGL_OPENGL_PROFILE_LEGACY: i32 = 0x1000;

// Framework loading state
static SYPHON_LOADED: AtomicBool = AtomicBool::new(false);
static SYPHON_INIT: Once = Once::new();

/// Load Syphon framework at runtime
fn ensure_syphon_loaded() -> bool {
    SYPHON_INIT.call_once(|| {
        let loaded = unsafe { load_syphon_framework() };
        SYPHON_LOADED.store(loaded, Ordering::SeqCst);
    });
    SYPHON_LOADED.load(Ordering::SeqCst)
}

unsafe fn load_syphon_framework() -> bool {
    let path = NSString::from_str("/Library/Frameworks/Syphon.framework");
    let bundle = NSBundle::bundleWithPath(&path);

    match bundle {
        Some(bundle) => {
            if bundle.isLoaded() {
                info!("[Syphon] Framework already loaded");
                return true;
            }

            if bundle.load() {
                info!("[Syphon] Successfully loaded Syphon.framework");
                true
            } else {
                error!("[Syphon] Failed to load Syphon.framework");
                false
            }
        }
        None => {
            error!("[Syphon] Syphon.framework not found at /Library/Frameworks/");
            false
        }
    }
}

/// OpenGL context wrapper for Syphon (OpenGL-based)
struct GLContext {
    context: CGLContextObj,
    pixel_format: *mut c_void,
}

unsafe impl Send for GLContext {}
unsafe impl Sync for GLContext {}

impl GLContext {
    fn new() -> Option<Self> {
        unsafe {
            let attributes: [i32; 8] = [
                K_CGL_PFA_ACCELERATED,
                K_CGL_PFA_ALLOW_OFFLINE_RENDERERS,
                K_CGL_PFA_DOUBLE_BUFFER,
                K_CGL_PFA_COLOR_SIZE,
                24,
                K_CGL_PFA_OPENGL_PROFILE,
                K_CGL_OPENGL_PROFILE_LEGACY,
                0, // terminator
            ];

            let mut pixel_format: *mut c_void = std::ptr::null_mut();
            let mut num_pixel_formats: i32 = 0;

            let err = CGLChoosePixelFormat(
                attributes.as_ptr(),
                &mut pixel_format,
                &mut num_pixel_formats,
            );

            if err != 0 || pixel_format.is_null() {
                error!("[GLContext] Failed to choose pixel format: error={}", err);
                return None;
            }

            let mut context: CGLContextObj = std::ptr::null_mut();
            let err = CGLCreateContext(pixel_format, std::ptr::null_mut(), &mut context);

            if err != 0 || context.is_null() {
                error!("[GLContext] Failed to create CGL context: error={}", err);
                CGLDestroyPixelFormat(pixel_format);
                return None;
            }

            info!("[GLContext] Created CGL context successfully");

            Some(Self {
                context,
                pixel_format,
            })
        }
    }

    fn lock(&self) -> bool {
        unsafe { CGLLockContext(self.context) == 0 }
    }

    fn unlock(&self) {
        unsafe {
            CGLUnlockContext(self.context);
        }
    }

    fn make_current(&self) -> bool {
        unsafe { CGLSetCurrentContext(self.context) == 0 }
    }

    fn raw(&self) -> CGLContextObj {
        self.context
    }
}

impl Drop for GLContext {
    fn drop(&mut self) {
        unsafe {
            CGLDestroyContext(self.context);
            CGLDestroyPixelFormat(self.pixel_format);
            info!("[GLContext] Destroyed CGL context");
        }
    }
}

/// Wrapper for SyphonServer (OpenGL-based)
struct SyphonOpenGLServer {
    server: Retained<AnyObject>,
}

unsafe impl Send for SyphonOpenGLServer {}
unsafe impl Sync for SyphonOpenGLServer {}

impl SyphonOpenGLServer {
    fn new(name: &str, cgl_context: CGLContextObj) -> Option<Self> {
        if !ensure_syphon_loaded() {
            error!("[SyphonOpenGLServer] Syphon framework not loaded");
            return None;
        }

        unsafe {
            // Get SyphonServer class
            let class = AnyClass::get(c"SyphonServer")?;

            // Allocate
            let obj: *mut AnyObject = msg_send![class, alloc];
            if obj.is_null() {
                error!("[SyphonOpenGLServer] Failed to allocate");
                return None;
            }

            // Create NSString for name
            let ns_name = NSString::from_str(name);

            // initWithName:context:options:
            let server: *mut AnyObject = msg_send![
                obj,
                initWithName: &*ns_name,
                context: cgl_context,
                options: std::ptr::null::<NSDictionary<NSString, AnyObject>>()
            ];

            if server.is_null() {
                error!("[SyphonOpenGLServer] initWithName failed");
                return None;
            }

            // Convert to Retained
            let retained = Retained::from_raw(server)?;
            info!("[SyphonOpenGLServer] Created server: {}", name);
            Some(Self { server: retained })
        }
    }

    fn publish_frame_texture(
        &self,
        texture_id: u32,
        target: u32,
        width: i32,
        height: i32,
        flipped: bool,
    ) {
        unsafe {
            // Use NSRect/NSSize from objc2_foundation
            let region = NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: width as f64,
                    height: height as f64,
                },
            };

            let dimensions = NSSize {
                width: width as f64,
                height: height as f64,
            };

            // publishFrameTexture:textureTarget:imageRegion:textureDimensions:flipped:
            let flipped_bool: objc2::runtime::Bool = if flipped {
                objc2::runtime::Bool::YES
            } else {
                objc2::runtime::Bool::NO
            };

            let _: () = msg_send![
                &*self.server,
                publishFrameTexture: texture_id,
                textureTarget: target,
                imageRegion: region,
                textureDimensions: dimensions,
                flipped: flipped_bool
            ];
        }
    }

    fn stop(&self) {
        unsafe {
            let _: () = msg_send![&*self.server, stop];
        }
    }
}

impl Drop for SyphonOpenGLServer {
    fn drop(&mut self) {
        self.stop();
        info!("[SyphonOpenGLServer] Stopped and released");
    }
}

/// Internal state for Syphon rendering (OpenGL path)
struct SyphonState {
    gl_context: GLContext,
    server: SyphonOpenGLServer,
    texture_id: u32,
    last_width: i32,
    last_height: i32,
}

// OpenGL constants
const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_RGBA: u32 = 0x1908;
const GL_RGBA8: i32 = 0x8058;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_LINEAR: i32 = 0x2601;

#[link(name = "OpenGL", kind = "framework")]
extern "C" {
    fn glGenTextures(n: i32, textures: *mut u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glTexImage2D(
        target: u32,
        level: i32,
        internalformat: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        type_: u32,
        pixels: *const c_void,
    );
    fn glTexParameteri(target: u32, pname: u32, param: i32);
    fn glDeleteTextures(n: i32, textures: *const u32);
    fn glFlush();
}

impl SyphonState {
    fn new(name: &str) -> Option<Self> {
        // Create GL context
        let gl_context = GLContext::new()?;

        // Lock and make current
        if !gl_context.lock() {
            error!("[SyphonState] Failed to lock GL context");
            return None;
        }

        if !gl_context.make_current() {
            error!("[SyphonState] Failed to make GL context current");
            gl_context.unlock();
            return None;
        }

        // Create Syphon server
        let server = match SyphonOpenGLServer::new(name, gl_context.raw()) {
            Some(s) => s,
            None => {
                gl_context.unlock();
                return None;
            }
        };

        // Create texture
        let mut texture_id: u32 = 0;
        unsafe {
            glGenTextures(1, &mut texture_id);
        }

        gl_context.unlock();

        info!(
            "[SyphonState] Created state: name={}, texture={}",
            name, texture_id
        );

        Some(Self {
            gl_context,
            server,
            texture_id,
            last_width: 0,
            last_height: 0,
        })
    }

    fn publish_frame(&mut self, data: &[u8], width: i32, height: i32) {
        if !self.gl_context.lock() {
            return;
        }

        if !self.gl_context.make_current() {
            self.gl_context.unlock();
            return;
        }

        unsafe {
            glBindTexture(GL_TEXTURE_2D, self.texture_id);

            if width != self.last_width || height != self.last_height {
                glTexImage2D(
                    GL_TEXTURE_2D,
                    0,
                    GL_RGBA8,
                    width,
                    height,
                    0,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    data.as_ptr() as *const _,
                );
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
                glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
                self.last_width = width;
                self.last_height = height;
                debug!("[SyphonState] Texture resized: {}x{}", width, height);
            } else {
                glTexImage2D(
                    GL_TEXTURE_2D,
                    0,
                    GL_RGBA8,
                    width,
                    height,
                    0,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    data.as_ptr() as *const _,
                );
            }

            glFlush();

            self.server.publish_frame_texture(
                self.texture_id,
                GL_TEXTURE_2D,
                width,
                height,
                true, // flipped for correct orientation
            );
        }

        self.gl_context.unlock();
    }
}

impl Drop for SyphonState {
    fn drop(&mut self) {
        if self.gl_context.lock() {
            if self.gl_context.make_current() {
                unsafe {
                    glDeleteTextures(1, &self.texture_id);
                }
            }
            self.gl_context.unlock();
        }
    }
}

/// Syphon sender that wraps appsink + Syphon SDK
pub struct SyphonSender {
    name: String,
    state: Arc<Mutex<Option<SyphonState>>>,
    last_pts_ns: Arc<AtomicU64>,
    #[allow(dead_code)]
    appsink: Option<gst_app::AppSink>,
}

impl SyphonSender {
    /// Create a new Syphon sender
    pub fn new(name: &str) -> AppResult<Self> {
        info!("[SyphonSender] Creating sender: {}", name);

        // Pre-create state
        let state = SyphonState::new(name);
        if state.is_none() {
            info!("[SyphonSender] Will retry state creation on first frame");
        }

        Ok(Self {
            name: name.to_string(),
            state: Arc::new(Mutex::new(state)),
            last_pts_ns: Arc::new(AtomicU64::new(0)),
            appsink: None,
        })
    }

    /// Create and configure an appsink for Syphon output
    pub fn create_appsink(&mut self) -> AppResult<gst::Element> {
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .build();

        let appsink = gst_app::AppSink::builder()
            .sync(true)
            .max_buffers(1)
            .drop(true)
            .caps(&caps)
            .build();

        let name = self.name.clone();
        let last_pts_ns = Arc::clone(&self.last_pts_ns);
        let state = Arc::clone(&self.state);

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| Self::handle_new_sample(sink, &last_pts_ns, &state, &name))
                .build(),
        );

        debug!(
            "[SyphonSender] Created appsink for '{}' with RGBA caps",
            self.name
        );

        self.appsink = Some(appsink.clone());

        Ok(appsink.upcast())
    }

    fn handle_new_sample(
        sink: &gst_app::AppSink,
        last_pts_ns: &Arc<AtomicU64>,
        state: &Arc<Mutex<Option<SyphonState>>>,
        name: &str,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;

        let buffer = sample.buffer().ok_or_else(|| {
            error!("[SyphonSender] No buffer in sample");
            gst::FlowError::Error
        })?;

        if let Some(pts) = buffer.pts() {
            last_pts_ns.store(pts.nseconds(), Ordering::Relaxed);
        }

        let caps = sample.caps().ok_or_else(|| {
            error!("[SyphonSender] No caps in sample");
            gst::FlowError::Error
        })?;

        let video_info = gst_video::VideoInfo::from_caps(caps).map_err(|e| {
            error!("[SyphonSender] Failed to get video info: {:?}", e);
            gst::FlowError::Error
        })?;

        let width = video_info.width() as i32;
        let height = video_info.height() as i32;

        let map = buffer.map_readable().map_err(|e| {
            error!("[SyphonSender] Failed to map buffer: {:?}", e);
            gst::FlowError::Error
        })?;

        let data = map.as_slice();

        // Get or create state
        let mut state_guard = state.lock();
        if state_guard.is_none() {
            info!("[SyphonSender] Lazy-creating Syphon state");
            *state_guard = SyphonState::new(name);
            if state_guard.is_none() {
                error!("[SyphonSender] Failed to create Syphon state");
                return Err(gst::FlowError::Error);
            }
        }

        if let Some(ref mut syphon_state) = *state_guard {
            syphon_state.publish_frame(data, width, height);
        }

        Ok(gst::FlowSuccess::Ok)
    }

    pub fn last_position(&self) -> f64 {
        let ns = self.last_pts_ns.load(Ordering::Relaxed);
        ns as f64 / 1_000_000_000.0
    }

    pub fn has_clients(&self) -> bool {
        false
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for SyphonSender {
    fn drop(&mut self) {
        info!("[SyphonSender] Dropping sender: {}", self.name);
    }
}
