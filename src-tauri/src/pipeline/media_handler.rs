//! メディアアイテムのパイプラインへの追加処理
//!
//! ビデオ・オーディオの動的パッド処理とシンク構築を担当

use gst::glib;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use tracing::{debug, error};

use crate::audio::sink::create_audio_sink;
use crate::error::{AppError, AppResult};
use crate::output::native_handle::{create_fallback_sink, create_video_sink_with_handle};
use crate::pipeline::OutputWithMonitor;
use crate::types::*;

/// メディアアイテムをパイプラインに追加
///
/// filesrc → decodebin を追加し、動的パッドのコールバックを設定
pub fn add_media_item(
    pipeline: &gst::Pipeline,
    item: &MediaItem,
    owm: &OutputWithMonitor,
    effective_brightness: f64,
    appsink_weak: Option<glib::WeakRef<gst_app::AppSink>>,
) -> AppResult<()> {
    // ソースエレメント
    let src = gst::ElementFactory::make("filesrc")
        .property("location", &item.path)
        .build()
        .map_err(|e| AppError::GStreamer(format!("Failed to create filesrc: {:?}", e)))?;

    let decode = gst::ElementFactory::make("decodebin")
        .build()
        .map_err(|e| AppError::GStreamer(format!("Failed to create decodebin: {:?}", e)))?;

    pipeline
        .add_many([&src, &decode])
        .map_err(|e| AppError::Pipeline(format!("Failed to add elements: {:?}", e)))?;

    src.link(&decode)
        .map_err(|e| AppError::Pipeline(format!("Failed to link src to decode: {:?}", e)))?;

    // 動的パッドのためのクロージャ用変数
    let item_clone = item.clone();
    let owm_clone = owm.clone();
    let pipeline_weak = pipeline.downgrade();
    let brightness = effective_brightness;

    decode.connect_pad_added(move |_, src_pad| {
        let pipeline = match pipeline_weak.upgrade() {
            Some(p) => p,
            None => return,
        };

        let caps = match src_pad.current_caps() {
            Some(c) => c,
            None => return,
        };

        let Some(structure) = caps.structure(0) else {
            return;
        };
        let name = structure.name();

        if name.starts_with("video/") && item_clone.media_type == MediaType::Video {
            handle_video_pad(
                &pipeline,
                src_pad,
                &item_clone,
                &owm_clone,
                brightness,
                appsink_weak.as_ref(),
            );
        } else if name.starts_with("audio/") && item_clone.media_type == MediaType::Video {
            // ビデオアイテムからのオーディオパッドは fakesink に捨てる
            handle_audio_pad_from_video(&pipeline, src_pad, &item_clone);
        } else if name.starts_with("audio/") && item_clone.media_type == MediaType::Audio {
            handle_audio_pad(&pipeline, src_pad, &owm_clone);
        }
    });

    Ok(())
}

/// ビデオパッドの処理
fn handle_video_pad(
    pipeline: &gst::Pipeline,
    src_pad: &gst::Pad,
    item: &MediaItem,
    owm: &OutputWithMonitor,
    brightness: f64,
    appsink_weak: Option<&glib::WeakRef<gst_app::AppSink>>,
) {
    debug!(
        "[CuePlayer] Video pad added for '{}' -> output '{}'",
        item.name, owm.output.name
    );

    // ビデオ処理チェーン
    let convert = match gst::ElementFactory::make("videoconvert").build() {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create videoconvert: {:?}", e);
            return;
        }
    };

    // brightness: 0.0 = normal, -1.0 = black, 1.0 = white
    // UI では 0-100 (100が通常) なので変換
    let gst_brightness = (brightness / 100.0) - 1.0;
    debug!(
        "[CuePlayer] Brightness for '{}': UI={} -> GStreamer={}",
        owm.output.name, brightness, gst_brightness
    );
    let balance = match gst::ElementFactory::make("videobalance")
        .property("brightness", gst_brightness)
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create videobalance: {:?}", e);
            return;
        }
    };

    // 出力タイプに応じたパイプライン構築
    match owm.output.output_type {
        OutputType::Ndi => {
            handle_ndi_video(pipeline, src_pad, owm, &convert, &balance, appsink_weak);
        }
        OutputType::Syphon => {
            #[cfg(target_os = "macos")]
            handle_syphon_video(pipeline, src_pad, owm, &convert, &balance, appsink_weak);
            #[cfg(not(target_os = "macos"))]
            error!("Syphon is only supported on macOS");
        }
        OutputType::Spout => {
            #[cfg(windows)]
            handle_spout_video(pipeline, src_pad, owm, &convert, &balance, appsink_weak);
            #[cfg(not(windows))]
            error!("Spout is only supported on Windows");
        }
        OutputType::Display => {
            handle_display_video(pipeline, src_pad, owm, &convert, &balance);
        }
        OutputType::Audio => {
            // Audio output doesn't have video, should not reach here
            error!("Audio output type received video pad");
        }
    }

    debug!(
        "[CuePlayer] Video pipeline linked successfully for '{}'",
        owm.output.name
    );
}

/// NDI出力用のビデオパイプライン構築
fn handle_ndi_video(
    pipeline: &gst::Pipeline,
    src_pad: &gst::Pad,
    owm: &OutputWithMonitor,
    convert: &gst::Element,
    balance: &gst::Element,
    appsink_weak: Option<&glib::WeakRef<gst_app::AppSink>>,
) {
    // appsinkベースのNDIパイプライン:
    // [ソース動画] → convert → balance → capsfilter(UYVY) → appsink → NdiSender

    let appsink = match appsink_weak.and_then(|w| w.upgrade()) {
        Some(a) => a,
        None => {
            error!("Failed to get appsink for NDI output");
            return;
        }
    };

    // NDI用にUYVYフォーマットに変換するcapsfilter
    let capsfilter = match gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("format", "UYVY")
                .build(),
        )
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create capsfilter: {:?}", e);
            return;
        }
    };

    if let Err(e) = pipeline.add_many([convert, balance, &capsfilter]) {
        error!("Failed to add video elements to pipeline: {:?}", e);
        return;
    }

    // convert → balance → capsfilter をリンク
    if let Err(e) = gst::Element::link_many([convert, balance, &capsfilter]) {
        error!("Failed to link convert to balance to capsfilter: {:?}", e);
        return;
    }

    // capsfilter → appsink をリンク
    let capsfilter_src = match capsfilter.static_pad("src") {
        Some(p) => p,
        None => {
            error!("Failed to get src pad from capsfilter");
            return;
        }
    };

    let appsink_sink = match appsink.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from appsink");
            return;
        }
    };

    if let Err(e) = capsfilter_src.link(&appsink_sink) {
        error!("Failed to link capsfilter to appsink: {:?}", e);
        return;
    }

    // デコーダからconvertへリンク
    let sink_pad = match convert.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from videoconvert");
            return;
        }
    };

    if let Err(e) = src_pad.link(&sink_pad) {
        error!("Failed to link src pad to sink pad: {:?}", e);
        return;
    }

    // 状態同期
    let _ = convert.sync_state_with_parent();
    let _ = balance.sync_state_with_parent();
    let _ = capsfilter.sync_state_with_parent();

    debug!(
        "[CuePlayer] NDI source linked to appsink for '{}'",
        owm.output.name
    );
}

/// Syphon出力用のビデオパイプライン構築 (macOS)
#[cfg(target_os = "macos")]
fn handle_syphon_video(
    pipeline: &gst::Pipeline,
    src_pad: &gst::Pad,
    owm: &OutputWithMonitor,
    convert: &gst::Element,
    balance: &gst::Element,
    appsink_weak: Option<&glib::WeakRef<gst_app::AppSink>>,
) {
    // appsinkベースのSyphonパイプライン:
    // [ソース動画] → convert → balance → capsfilter(RGBA) → appsink → SyphonSender

    let appsink = match appsink_weak.and_then(|w| w.upgrade()) {
        Some(a) => a,
        None => {
            error!("Failed to get appsink for Syphon output");
            return;
        }
    };

    // Syphon用にRGBAフォーマットに変換するcapsfilter
    let capsfilter = match gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("format", "RGBA")
                .build(),
        )
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create capsfilter for Syphon: {:?}", e);
            return;
        }
    };

    if let Err(e) = pipeline.add_many([convert, balance, &capsfilter]) {
        error!("Failed to add video elements to pipeline: {:?}", e);
        return;
    }

    // convert → balance → capsfilter をリンク
    if let Err(e) = gst::Element::link_many([convert, balance, &capsfilter]) {
        error!("Failed to link convert to balance to capsfilter: {:?}", e);
        return;
    }

    // capsfilter → appsink をリンク
    let capsfilter_src = match capsfilter.static_pad("src") {
        Some(p) => p,
        None => {
            error!("Failed to get src pad from capsfilter");
            return;
        }
    };

    let appsink_sink = match appsink.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from appsink");
            return;
        }
    };

    if let Err(e) = capsfilter_src.link(&appsink_sink) {
        error!("Failed to link capsfilter to appsink: {:?}", e);
        return;
    }

    // デコーダからconvertへリンク
    let sink_pad = match convert.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from videoconvert");
            return;
        }
    };

    if let Err(e) = src_pad.link(&sink_pad) {
        error!("Failed to link src pad to sink pad: {:?}", e);
        return;
    }

    // 状態同期
    let _ = convert.sync_state_with_parent();
    let _ = balance.sync_state_with_parent();
    let _ = capsfilter.sync_state_with_parent();

    debug!(
        "[CuePlayer] Syphon source linked to appsink for '{}'",
        owm.output.name
    );
}

/// Display出力用のビデオパイプライン構築
fn handle_display_video(
    pipeline: &gst::Pipeline,
    src_pad: &gst::Pad,
    owm: &OutputWithMonitor,
    convert: &gst::Element,
    balance: &gst::Element,
) {
    let sink = match create_video_sink(owm) {
        Ok(s) => {
            debug!(
                "[CuePlayer] Created video sink: {}",
                s.factory()
                    .map(|f| f.name().to_string())
                    .unwrap_or_default()
            );
            s
        }
        Err(e) => {
            error!("Failed to create video sink: {:?}", e);
            return;
        }
    };

    if let Err(e) = pipeline.add_many([convert, balance, &sink]) {
        error!("Failed to add elements to pipeline: {:?}", e);
        return;
    }

    if let Err(e) = gst::Element::link_many([convert, balance, &sink]) {
        error!("Failed to link video elements: {:?}", e);
        return;
    }

    let sink_pad = match convert.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from videoconvert");
            return;
        }
    };

    if let Err(e) = src_pad.link(&sink_pad) {
        error!("Failed to link src pad to sink pad: {:?}", e);
        return;
    }

    let _ = convert.sync_state_with_parent();
    let _ = balance.sync_state_with_parent();
    if let Err(e) = sink.sync_state_with_parent() {
        error!("Failed to sync sink state: {:?}", e);
    }
}

/// ビデオアイテムからのオーディオパッドを処理（fakesinkに破棄）
fn handle_audio_pad_from_video(pipeline: &gst::Pipeline, src_pad: &gst::Pad, item: &MediaItem) {
    debug!(
        "[CuePlayer] Discarding audio pad from video item '{}' with fakesink",
        item.name
    );
    let fakesink = match gst::ElementFactory::make("fakesink")
        .property("async", false)
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create fakesink for audio: {:?}", e);
            return;
        }
    };

    if let Err(e) = pipeline.add(&fakesink) {
        error!("Failed to add fakesink to pipeline: {:?}", e);
        return;
    }

    let sink_pad = match fakesink.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from fakesink");
            return;
        }
    };

    if let Err(e) = src_pad.link(&sink_pad) {
        error!("Failed to link audio pad to fakesink: {:?}", e);
        return;
    }

    let _ = fakesink.sync_state_with_parent();
}

/// オーディオアイテムのオーディオパッドを処理
fn handle_audio_pad(pipeline: &gst::Pipeline, src_pad: &gst::Pad, owm: &OutputWithMonitor) {
    let convert = match gst::ElementFactory::make("audioconvert").build() {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create audioconvert: {:?}", e);
            return;
        }
    };

    let resample = match gst::ElementFactory::make("audioresample").build() {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create audioresample: {:?}", e);
            return;
        }
    };

    // Volume element with unique name for later access
    let volume_name = format!("volume_{}", owm.output.id);
    let volume = match gst::ElementFactory::make("volume")
        .name(&volume_name)
        .property("volume", 1.0_f64)
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to create volume element: {:?}", e);
            return;
        }
    };

    let sink = match create_audio_sink(&owm.output) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create audio sink: {:?}", e);
            return;
        }
    };

    if let Err(e) = pipeline.add_many([&convert, &resample, &volume, &sink]) {
        error!("Failed to add audio elements to pipeline: {:?}", e);
        return;
    }

    if let Err(e) = gst::Element::link_many([&convert, &resample, &volume, &sink]) {
        error!("Failed to link audio elements: {:?}", e);
        return;
    }

    let sink_pad = match convert.static_pad("sink") {
        Some(p) => p,
        None => {
            error!("Failed to get sink pad from audioconvert");
            return;
        }
    };

    if let Err(e) = src_pad.link(&sink_pad) {
        error!("Failed to link audio src pad to sink pad: {:?}", e);
        return;
    }

    let _ = convert.sync_state_with_parent();
    let _ = resample.sync_state_with_parent();
    let _ = volume.sync_state_with_parent();
    let _ = sink.sync_state_with_parent();
}

/// ビデオシンクの作成
pub fn create_video_sink(owm: &OutputWithMonitor) -> Result<gst::Element, gst::glib::BoolError> {
    match owm.output.output_type {
        OutputType::Display => {
            // モニター情報をログ出力（デバッグ用）
            if let Some(ref monitor) = owm.monitor {
                let fullscreen = owm.output.fullscreen.unwrap_or(true);
                debug!(
                    "[CuePlayer] Display output: {} -> Monitor {} at ({}, {}) {}x{} fullscreen={}",
                    owm.output.name,
                    monitor.index,
                    monitor.x,
                    monitor.y,
                    monitor.width,
                    monitor.height,
                    fullscreen
                );
            } else {
                debug!(
                    "[CuePlayer] Display output: {} -> Default monitor",
                    owm.output.name
                );
            }

            // ネイティブハンドルがあればプラットフォーム固有シンクを使用
            if let Some(ref handle) = owm.native_handle {
                debug!(
                    "[CuePlayer] Using platform-specific sink with native handle for '{}'",
                    owm.output.name
                );
                match create_video_sink_with_handle(handle) {
                    Ok(sink) => {
                        debug!(
                            "[CuePlayer] Successfully created platform-specific sink for '{}'",
                            owm.output.name
                        );
                        return Ok(sink);
                    }
                    Err(e) => {
                        debug!(
                            "[CuePlayer] Failed to create platform sink: {:?}, falling back to autovideosink",
                            e
                        );
                    }
                }
            } else {
                debug!(
                    "[CuePlayer] No native handle available for '{}', using fallback",
                    owm.output.name
                );
            }

            // フォールバック: autovideosink
            create_fallback_sink()
        }
        OutputType::Ndi => {
            // NDI送信
            let ndi_name = owm.output.ndi_name.as_deref().unwrap_or("TauriLivePlayer");
            debug!(
                "Creating NDI sink for '{}' with ndi-name='{}'",
                owm.output.name, ndi_name
            );
            // ライブパイプラインモード: async=true（デフォルト）を維持してクロック同期を保持
            // pipeline.set_latency() と組み合わせて使用
            let sink = gst::ElementFactory::make("ndisink")
                .property("ndi-name", ndi_name)
                .build()?;
            debug!("NDI sink created successfully");
            Ok(sink)
        }
        OutputType::Audio => Err(gst::glib::bool_error!(
            "Audio output cannot be used as video sink"
        )),
        OutputType::Syphon => Err(gst::glib::bool_error!(
            "Syphon output uses appsink, not video sink"
        )),
        OutputType::Spout => Err(gst::glib::bool_error!(
            "Spout output uses appsink, not video sink"
        )),
    }
}
