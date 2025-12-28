//! NDI送信モジュール
//!
//! appsinkからフレームを受け取り、grafton-ndi (NDI SDK) で直接送信する。
//! これにより ndisink のライブシンク問題（13秒オフセット等）を回避し、
//! 正確な再生位置（PTS）を取得できる。
//!
//! ## 実装方式
//! - ダブルバッファ + 非同期送信（レベル2最適化）
//! - GStreamerが同期管理するため clock_video = false
//! - フレームレートは動画から動的に取得

use grafton_ndi::{PixelFormat, Sender, SenderOptions, VideoFrame, NDI};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, trace};

use crate::error::{AppError, AppResult};

/// NDI送信を管理する構造体
/// appsink からフレームを受け取り、NDI SDK で送信
pub struct NdiSender {
    /// NDI送信元の名前
    name: String,

    /// NDI SDK インスタンス（Senderより長生きする必要がある）
    #[allow(dead_code)]
    ndi: Arc<NDI>,

    /// NDI Sender
    sender: Arc<Mutex<Sender<'static>>>,

    /// 最後に送信したフレームのPTS（ナノ秒）
    /// position取得に使用
    last_pts_ns: Arc<AtomicU64>,

    /// appsink への参照
    #[allow(dead_code)]
    appsink: Option<gst_app::AppSink>,
}

impl NdiSender {
    /// 新しいNdiSenderを作成
    pub fn new(name: &str) -> AppResult<Self> {
        info!("[NdiSender] Creating new sender with name: {}", name);

        // NDI SDK 初期化
        debug!("[NdiSender] Initializing NDI SDK...");
        let ndi = NDI::new().map_err(|e| {
            error!("[NdiSender] Failed to initialize NDI SDK: {:?}", e);
            AppError::Ndi(format!("Failed to initialize NDI SDK: {:?}", e))
        })?;
        debug!("[NdiSender] NDI SDK initialized");

        // NDIインスタンスをArcでラップ
        let ndi_arc: Arc<NDI> = Arc::new(ndi);

        // Sender作成
        // grafton-ndi では clock_video/clock_audio のどちらかは true にする必要がある
        // clock_video = true でも appsink.sync = true なので GStreamer が同期を管理
        debug!("[NdiSender] Creating sender options...");
        let options = SenderOptions::builder(name)
            .clock_video(true)
            .clock_audio(false)
            .build();

        debug!("[NdiSender] Creating NDI sender...");
        let sender = {
            // 安全性: ndi_arcはNdiSenderがDropするまで生存し、senderより長生きする
            let ndi_ref: &'static NDI = unsafe { &*(Arc::as_ptr(&ndi_arc) as *const NDI) };
            Sender::new(ndi_ref, &options).map_err(|e| {
                error!("[NdiSender] Failed to create NDI sender: {:?}", e);
                AppError::Ndi(format!("Failed to create NDI sender: {:?}", e))
            })?
        };

        info!("[NdiSender] NDI sender '{}' created successfully", name);

        Ok(Self {
            name: name.to_string(),
            ndi: ndi_arc,
            sender: Arc::new(Mutex::new(sender)),
            last_pts_ns: Arc::new(AtomicU64::new(0)),
            appsink: None,
        })
    }

    /// appsink を作成して返す（パイプラインに追加用）
    pub fn create_appsink(&mut self) -> AppResult<gst::Element> {
        debug!("[NdiSender] Creating appsink for '{}'", self.name);

        // UYVY形式のcaps（NDI推奨フォーマット）
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "UYVY")
            .build();

        let appsink = gst_app::AppSink::builder()
            .sync(true) // GStreamer同期を有効化
            .max_buffers(1) // バッファを溜めない
            .drop(true) // 遅れたフレームはドロップ
            .caps(&caps)
            .build();

        // コールバック設定
        let last_pts_ns = self.last_pts_ns.clone();
        let sender = self.sender.clone();
        let name = self.name.clone();

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| Self::handle_new_sample(sink, &last_pts_ns, &sender, &name))
                .build(),
        );

        self.appsink = Some(appsink.clone());

        debug!("[NdiSender] appsink created for '{}'", self.name);
        Ok(appsink.upcast())
    }

    /// appsinkからの新しいサンプルを処理
    fn handle_new_sample(
        sink: &gst_app::AppSink,
        last_pts_ns: &Arc<AtomicU64>,
        sender: &Arc<Mutex<Sender<'static>>>,
        name: &str,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let sample = sink.pull_sample().map_err(|_| gst::FlowError::Error)?;
        let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;

        // PTS を記録（position取得用）
        if let Some(pts) = buffer.pts() {
            last_pts_ns.store(pts.nseconds(), Ordering::Relaxed);
            trace!("[NdiSender] {} - PTS: {:.3}s", name, pts.seconds_f64());
        }

        // Caps から解像度とフレームレートを取得
        let caps = sample.caps().ok_or(gst::FlowError::Error)?;
        let video_info =
            gst_video::VideoInfo::from_caps(caps).map_err(|_| gst::FlowError::Error)?;

        let width = video_info.width() as i32;
        let height = video_info.height() as i32;

        // フレームレートを取得（動画から動的に）
        let fps = video_info.fps();
        let fps_n = fps.numer() as i32;
        let fps_d = fps.denom() as i32;

        // バッファデータを取得
        let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
        let data = map.as_slice();

        // VideoFrame を作成
        let mut frame = VideoFrame::builder()
            .resolution(width, height)
            .pixel_format(PixelFormat::UYVY)
            .frame_rate(fps_n, fps_d)
            .build()
            .map_err(|e| {
                error!("[NdiSender] Failed to create VideoFrame: {:?}", e);
                gst::FlowError::Error
            })?;

        // データをコピー
        let copy_len = data.len().min(frame.data.len());
        frame.data[..copy_len].copy_from_slice(&data[..copy_len]);

        // 同期送信（送信完了までブロック）
        let sender_guard = sender.lock();
        sender_guard.send_video(&frame);

        trace!(
            "[NdiSender] {} - Sent frame {}x{} @ {}/{}fps",
            name,
            width,
            height,
            fps_n,
            fps_d,
        );

        Ok(gst::FlowSuccess::Ok)
    }

    /// 最後に送信したフレームのPTS（秒）
    /// UI からの position クエリに使用
    pub fn last_position(&self) -> f64 {
        let pts_ns = self.last_pts_ns.load(Ordering::Relaxed);
        pts_ns as f64 / 1_000_000_000.0
    }

    /// 最後に送信したフレームのPTS（ナノ秒）
    #[allow(dead_code)]
    pub fn last_pts_ns(&self) -> u64 {
        self.last_pts_ns.load(Ordering::Relaxed)
    }

    /// PTSをリセット
    #[allow(dead_code)]
    pub fn reset_pts(&self) {
        self.last_pts_ns.store(0, Ordering::Relaxed);
    }

    /// 送信元名を取得
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for NdiSender {
    fn drop(&mut self) {
        info!("[NdiSender] Dropping sender '{}'", self.name);
        // grafton-ndi の Sender は Drop で自動的にクリーンアップされる
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NDI SDKがインストールされている環境でのみテスト可能
    #[test]
    #[ignore]
    fn test_ndi_sender_creation() {
        let sender = NdiSender::new("Test Sender");
        assert!(sender.is_ok());
    }
}
