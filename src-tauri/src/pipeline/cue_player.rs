use gstreamer as gst;
use gstreamer::prelude::*;
use std::collections::HashMap;

use crate::audio::sink::create_audio_sink;
use crate::error::{AppError, AppResult};
use crate::types::*;

pub struct CuePlayer {
    pipeline: gst::Pipeline,
    video_balances: HashMap<String, gst::Element>,
    master_brightness: f64,
    output_brightness: HashMap<String, Option<f64>>,
}

impl CuePlayer {
    pub fn new() -> Result<Self, gst::glib::Error> {
        let pipeline = gst::Pipeline::new();

        Ok(Self {
            pipeline,
            video_balances: HashMap::new(),
            master_brightness: 100.0,
            output_brightness: HashMap::new(),
        })
    }

    /// Cueを読み込んでパイプラインを構築
    pub fn load_cue(&mut self, cue: &Cue, outputs: &[OutputTarget]) -> AppResult<()> {
        // パイプラインをリセット
        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|e| AppError::Pipeline(format!("Failed to reset pipeline: {:?}", e)))?;

        // 既存のエレメントを削除
        let iter = self.pipeline.iterate_elements();
        for element in iter.into_iter().flatten() {
            let _ = self.pipeline.remove(&element);
        }
        self.video_balances.clear();

        // 出力ごとの明るさ設定を保存
        for output in outputs {
            self.output_brightness
                .insert(output.id.clone(), output.brightness);
        }

        // 各メディアアイテムを追加
        for item in &cue.items {
            let output = outputs
                .iter()
                .find(|o| o.id == item.output_id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("Output not found: {}", item.output_id))
                })?;

            self.add_media_item(item, output)?;
        }

        // PAUSED状態にしてプリロール
        self.pipeline
            .set_state(gst::State::Paused)
            .map_err(|e| AppError::Pipeline(format!("Failed to pause pipeline: {:?}", e)))?;

        // 状態変更を待機
        let bus = self
            .pipeline
            .bus()
            .ok_or_else(|| AppError::Pipeline("Failed to get bus".to_string()))?;

        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => break,
                gst::MessageView::Error(err) => {
                    return Err(AppError::Pipeline(format!(
                        "Pipeline error: {} ({:?})",
                        err.error(),
                        err.debug()
                    )));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn add_media_item(&mut self, item: &MediaItem, output: &OutputTarget) -> AppResult<()> {
        // ソースエレメント
        let src = gst::ElementFactory::make("filesrc")
            .property("location", &item.path)
            .build()
            .map_err(|e| AppError::GStreamer(format!("Failed to create filesrc: {:?}", e)))?;

        let decode = gst::ElementFactory::make("decodebin")
            .build()
            .map_err(|e| AppError::GStreamer(format!("Failed to create decodebin: {:?}", e)))?;

        self.pipeline
            .add_many([&src, &decode])
            .map_err(|e| AppError::Pipeline(format!("Failed to add elements: {:?}", e)))?;

        src.link(&decode)
            .map_err(|e| AppError::Pipeline(format!("Failed to link src to decode: {:?}", e)))?;

        // 動的パッドのためのクロージャ用変数
        let item_clone = item.clone();
        let output_clone = output.clone();
        let pipeline_weak = self.pipeline.downgrade();
        let brightness = self.get_effective_brightness(&output.id);

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
                // ビデオ処理チェーン
                let convert = match gst::ElementFactory::make("videoconvert").build() {
                    Ok(e) => e,
                    Err(_) => return,
                };

                // brightness: 0.0 = normal, -1.0 = black, 1.0 = white
                // UI では 0-100 (100が通常) なので変換
                let gst_brightness = (brightness / 100.0) - 1.0;
                let balance = match gst::ElementFactory::make("videobalance")
                    .property("brightness", gst_brightness)
                    .build()
                {
                    Ok(e) => e,
                    Err(_) => return,
                };

                let sink = match create_video_sink(&output_clone) {
                    Ok(s) => s,
                    Err(_) => return,
                };

                if pipeline.add_many([&convert, &balance, &sink]).is_err() {
                    return;
                }

                if gst::Element::link_many([&convert, &balance, &sink]).is_err() {
                    return;
                }

                let sink_pad = match convert.static_pad("sink") {
                    Some(p) => p,
                    None => return,
                };

                if src_pad.link(&sink_pad).is_err() {
                    return;
                }

                let _ = convert.sync_state_with_parent();
                let _ = balance.sync_state_with_parent();
                let _ = sink.sync_state_with_parent();
            } else if name.starts_with("audio/") && item_clone.media_type == MediaType::Audio {
                // オーディオ処理チェーン
                let convert = match gst::ElementFactory::make("audioconvert").build() {
                    Ok(e) => e,
                    Err(_) => return,
                };

                let resample = match gst::ElementFactory::make("audioresample").build() {
                    Ok(e) => e,
                    Err(_) => return,
                };

                let sink = match create_audio_sink(&output_clone) {
                    Ok(s) => s,
                    Err(_) => return,
                };

                if pipeline.add_many([&convert, &resample, &sink]).is_err() {
                    return;
                }

                if gst::Element::link_many([&convert, &resample, &sink]).is_err() {
                    return;
                }

                let sink_pad = match convert.static_pad("sink") {
                    Some(p) => p,
                    None => return,
                };

                if src_pad.link(&sink_pad).is_err() {
                    return;
                }

                let _ = convert.sync_state_with_parent();
                let _ = resample.sync_state_with_parent();
                let _ = sink.sync_state_with_parent();
            }
        });

        Ok(())
    }

    fn get_effective_brightness(&self, output_id: &str) -> f64 {
        self.output_brightness
            .get(output_id)
            .and_then(|b| *b)
            .unwrap_or(self.master_brightness)
    }

    // ========================================
    // 再生制御
    // ========================================

    pub fn play(&self) -> AppResult<()> {
        self.pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| AppError::Pipeline(format!("Failed to play: {:?}", e)))?;
        Ok(())
    }

    pub fn pause(&self) -> AppResult<()> {
        self.pipeline
            .set_state(gst::State::Paused)
            .map_err(|e| AppError::Pipeline(format!("Failed to pause: {:?}", e)))?;
        Ok(())
    }

    pub fn stop(&self) -> AppResult<()> {
        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|e| AppError::Pipeline(format!("Failed to stop: {:?}", e)))?;
        Ok(())
    }

    pub fn seek(&self, position_secs: f64) -> AppResult<()> {
        let position = gst::ClockTime::from_seconds_f64(position_secs);
        self.pipeline
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, position)
            .map_err(|e| AppError::Pipeline(format!("Failed to seek: {:?}", e)))?;
        Ok(())
    }

    // ========================================
    // 明るさ調整
    // ========================================

    pub fn set_master_brightness(&mut self, value: f64) {
        self.master_brightness = value;

        // Master連動の出力を更新
        for (output_id, balance) in &self.video_balances {
            if self
                .output_brightness
                .get(output_id)
                .map(|b| b.is_none())
                .unwrap_or(true)
            {
                let gst_brightness = (value / 100.0) - 1.0;
                balance.set_property("brightness", gst_brightness);
            }
        }
    }

    pub fn set_output_brightness(&mut self, output_id: &str, value: Option<f64>) {
        self.output_brightness.insert(output_id.to_string(), value);

        if let Some(balance) = self.video_balances.get(output_id) {
            let effective = value.unwrap_or(self.master_brightness);
            let gst_brightness = (effective / 100.0) - 1.0;
            balance.set_property("brightness", gst_brightness);
        }
    }

    // ========================================
    // 状態取得
    // ========================================

    pub fn position(&self) -> Option<f64> {
        self.pipeline
            .query_position::<gst::ClockTime>()
            .map(|p| p.seconds_f64())
    }

    pub fn duration(&self) -> Option<f64> {
        self.pipeline
            .query_duration::<gst::ClockTime>()
            .map(|d| d.seconds_f64())
    }

    pub fn state(&self) -> gst::State {
        self.pipeline.current_state()
    }
}

impl Drop for CuePlayer {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

fn create_video_sink(output: &OutputTarget) -> Result<gst::Element, gst::glib::BoolError> {
    match output.output_type {
        OutputType::Display => {
            // TODO: 特定ディスプレイへの出力
            gst::ElementFactory::make("autovideosink").build()
        }
        OutputType::Ndi => {
            // NDI送信
            gst::ElementFactory::make("ndisink")
                .property(
                    "ndi-name",
                    output.ndi_name.as_deref().unwrap_or("TauriLivePlayer"),
                )
                .build()
        }
        OutputType::Audio => Err(gst::glib::bool_error!(
            "Audio output cannot be used as video sink"
        )),
    }
}
