//! CuePlayer - キューの再生を管理するプレイヤー
//!
//! GStreamerパイプラインを構築・制御し、複数出力への同期再生を実現

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

use crate::error::{AppError, AppResult};
use crate::output::native_handle::NativeHandle;
use crate::pipeline::media_handler;
use crate::pipeline::NdiSender;
use crate::types::*;

/// 出力先とモニター情報、ネイティブハンドルを組み合わせた構造体
#[derive(Debug, Clone)]
pub struct OutputWithMonitor {
    pub output: OutputTarget,
    pub monitor: Option<MonitorInfo>,
    pub native_handle: Option<NativeHandle>,
}

/// キュープレイヤー
///
/// 単一のGStreamerパイプラインで複数の出力先への同期再生を管理
pub struct CuePlayer {
    pipeline: gst::Pipeline,
    video_balances: HashMap<String, gst::Element>,
    volume_elements: HashMap<String, gst::Element>,
    /// NDI出力用のNdiSender (output_id -> NdiSender)
    ndi_senders: HashMap<String, Arc<NdiSender>>,
    /// NDI出力用のappsink (output_id -> AppSink)
    ndi_appsinks: HashMap<String, gst_app::AppSink>,
    master_brightness: f64,
    master_volume: f64,
    output_brightness: HashMap<String, Option<f64>>,
}

impl CuePlayer {
    pub fn new() -> Result<Self, gst::glib::Error> {
        let pipeline = gst::Pipeline::new();

        Ok(Self {
            pipeline,
            video_balances: HashMap::new(),
            volume_elements: HashMap::new(),
            ndi_senders: HashMap::new(),
            ndi_appsinks: HashMap::new(),
            master_brightness: 100.0,
            master_volume: 100.0,
            output_brightness: HashMap::new(),
        })
    }

    /// Cueを読み込んでパイプラインを構築
    pub fn load_cue(
        &mut self,
        cue: &Cue,
        outputs: &[OutputTarget],
        monitors: &[MonitorInfo],
        native_handles: &HashMap<String, NativeHandle>,
    ) -> AppResult<()> {
        self.reset_pipeline()?;

        let outputs_with_monitors =
            self.build_outputs_with_monitors(outputs, monitors, native_handles);

        // NDI出力用のNdiSenderを作成
        for owm in &outputs_with_monitors {
            if owm.output.output_type == OutputType::Ndi {
                self.setup_ndi_sender(owm)?;
            }
        }

        // 各メディアアイテムを追加
        for item in &cue.items {
            let owm = outputs_with_monitors
                .iter()
                .find(|o| o.output.id == item.output_id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("Output not found: {}", item.output_id))
                })?;

            let brightness = self.get_effective_brightness(&owm.output.id);
            let appsink_weak = if owm.output.output_type == OutputType::Ndi {
                self.ndi_appsinks.get(&owm.output.id).map(|a| a.downgrade())
            } else {
                None
            };

            media_handler::add_media_item(&self.pipeline, item, owm, brightness, appsink_weak)?;
        }

        self.configure_live_mode(&outputs_with_monitors);
        self.preroll_pipeline()?;
        self.apply_master_volume();

        Ok(())
    }

    /// パイプラインをリセット
    fn reset_pipeline(&mut self) -> AppResult<()> {
        self.pipeline
            .set_state(gst::State::Null)
            .map_err(|e| AppError::Pipeline(format!("Failed to reset pipeline: {:?}", e)))?;

        // 既存のエレメントを削除
        let iter = self.pipeline.iterate_elements();
        for element in iter.into_iter().flatten() {
            let _ = self.pipeline.remove(&element);
        }
        self.video_balances.clear();
        self.volume_elements.clear();
        self.ndi_senders.clear();
        self.ndi_appsinks.clear();

        Ok(())
    }

    /// 出力とモニター情報、ネイティブハンドルを組み合わせ
    fn build_outputs_with_monitors(
        &mut self,
        outputs: &[OutputTarget],
        monitors: &[MonitorInfo],
        native_handles: &HashMap<String, NativeHandle>,
    ) -> Vec<OutputWithMonitor> {
        debug!(
            "[CuePlayer] Project output IDs: {:?}",
            outputs.iter().map(|o| &o.id).collect::<Vec<_>>()
        );
        debug!(
            "[CuePlayer] Native handle keys: {:?}",
            native_handles.keys().collect::<Vec<_>>()
        );

        let outputs_with_monitors: Vec<OutputWithMonitor> = outputs
            .iter()
            .map(|o| {
                let monitor = if o.output_type == OutputType::Display {
                    monitors.get(o.display_index.unwrap_or(0)).cloned()
                } else {
                    None
                };
                let native_handle = native_handles.get(&o.id).cloned();
                debug!(
                    "[CuePlayer] Output '{}' -> native_handle present: {}",
                    o.id,
                    native_handle.is_some()
                );

                // 出力ごとの明るさ設定を保存
                self.output_brightness.insert(o.id.clone(), o.brightness);

                OutputWithMonitor {
                    output: o.clone(),
                    monitor,
                    native_handle,
                }
            })
            .collect();

        outputs_with_monitors
    }

    /// NDI出力用のNdiSenderとappsinkを作成
    fn setup_ndi_sender(&mut self, owm: &OutputWithMonitor) -> AppResult<()> {
        let output_id = &owm.output.id;
        let ndi_name = owm.output.ndi_name.as_deref().unwrap_or("TauriLivePlayer");

        debug!(
            "[CuePlayer] Setting up NDI sender for '{}' (ndi-name='{}')",
            owm.output.name, ndi_name
        );

        // NdiSenderを作成
        let mut ndi_sender = NdiSender::new(ndi_name)?;

        // appsinkを作成（NdiSenderが内部でコールバックを設定）
        let appsink_element = ndi_sender.create_appsink()?;
        let appsink = appsink_element
            .downcast::<gst_app::AppSink>()
            .map_err(|_| AppError::Pipeline("Failed to downcast to AppSink".to_string()))?;

        // appsinkをパイプラインに追加
        self.pipeline.add(&appsink).map_err(|e| {
            AppError::Pipeline(format!("Failed to add appsink to pipeline: {:?}", e))
        })?;

        // NdiSenderとappsinkを保存
        self.ndi_senders
            .insert(output_id.clone(), Arc::new(ndi_sender));
        self.ndi_appsinks.insert(output_id.clone(), appsink);

        debug!(
            "[CuePlayer] NDI sender created for '{}' (appsink方式)",
            owm.output.name
        );

        Ok(())
    }

    /// ライブ出力がある場合のパイプライン設定
    fn configure_live_mode(&self, outputs_with_monitors: &[OutputWithMonitor]) {
        let has_live_output = outputs_with_monitors
            .iter()
            .any(|owm| matches!(owm.output.output_type, OutputType::Ndi));

        if has_live_output {
            let latency = gst::ClockTime::from_mseconds(100);
            self.pipeline.set_latency(latency);
            debug!(
                "[CuePlayer] Live mode enabled: pipeline latency set to {:?}",
                latency
            );
        }
    }

    /// パイプラインをプリロール
    fn preroll_pipeline(&self) -> AppResult<()> {
        debug!("[CuePlayer] Setting pipeline to PAUSED...");
        self.pipeline
            .set_state(gst::State::Paused)
            .map_err(|e| AppError::Pipeline(format!("Failed to pause pipeline: {:?}", e)))?;

        let bus = self
            .pipeline
            .bus()
            .ok_or_else(|| AppError::Pipeline("Failed to get bus".to_string()))?;

        debug!("[CuePlayer] Waiting for pipeline to preroll...");
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => {
                    debug!("[CuePlayer] Pipeline preroll complete (AsyncDone)");
                    break;
                }
                gst::MessageView::Error(err) => {
                    error!(
                        "[CuePlayer] Pipeline error: {} ({:?})",
                        err.error(),
                        err.debug()
                    );
                    return Err(AppError::Pipeline(format!(
                        "Pipeline error: {} ({:?})",
                        err.error(),
                        err.debug()
                    )));
                }
                gst::MessageView::Warning(warn) => {
                    error!(
                        "[CuePlayer] Pipeline warning: {} ({:?})",
                        warn.error(),
                        warn.debug()
                    );
                }
                gst::MessageView::StateChanged(state) => {
                    if let Some(src) = state.src() {
                        if src.type_() == gst::Pipeline::static_type() {
                            debug!(
                                "[CuePlayer] Pipeline state: {:?} -> {:?}",
                                state.old(),
                                state.current()
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        debug!(
            "[CuePlayer] Pipeline current state: {:?}",
            self.pipeline.current_state()
        );

        // プリロール後の位置調整
        self.adjust_initial_position(&bus);

        Ok(())
    }

    /// 動画ファイルのPTSが0から始まっていない場合の調整
    fn adjust_initial_position(&self, bus: &gst::Bus) {
        let pos_after_preroll = self.pipeline.query_position::<gst::ClockTime>();
        let dur = self.pipeline.query_duration::<gst::ClockTime>();
        debug!(
            "[CuePlayer] After preroll: position={:?}, duration={:?}",
            pos_after_preroll, dur
        );

        if let Some(pos) = pos_after_preroll {
            if pos.seconds() > 0 {
                debug!(
                    "[CuePlayer] Non-zero initial position detected ({:?}), seeking to 0",
                    pos
                );
                if let Err(e) = self.pipeline.seek_simple(
                    gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                    gst::ClockTime::ZERO,
                ) {
                    warn!("[CuePlayer] Failed to seek to 0: {:?}", e);
                } else {
                    for msg in bus.iter_timed(gst::ClockTime::from_seconds(2)) {
                        if let gst::MessageView::AsyncDone(_) = msg.view() {
                            debug!("[CuePlayer] Seek to 0 complete");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// 現在のmaster_volumeを全てのvolume要素に適用
    fn apply_master_volume(&self) {
        let gst_volume = self.master_volume / 100.0;
        for element in self.pipeline.iterate_elements().into_iter().flatten() {
            if element.name().starts_with("volume_") {
                element.set_property("volume", gst_volume);
            }
        }
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
        debug!(
            "play() called, current state: {:?}",
            self.pipeline.current_state()
        );

        let latency = self.pipeline.latency();
        debug!("play() pipeline latency: {:?}", latency);

        // 各シンクのlatencyをクエリ
        let iter = self.pipeline.iterate_sinks();
        for sink in iter.into_iter().flatten() {
            let mut query = gst::query::Latency::new();
            if sink.query(&mut query) {
                let (live, min, max) = query.result();
                debug!(
                    "play() sink '{}' latency: live={}, min={:?}, max={:?}",
                    sink.name(),
                    live,
                    min,
                    max
                );
            }
        }

        let pos_before = self.pipeline.query_position::<gst::ClockTime>();
        debug!("play() position before: {:?}", pos_before);

        let result = self.pipeline.set_state(gst::State::Playing);
        debug!("play() set_state result: {:?}", result);

        let (success, state, pending) = self.pipeline.state(gst::ClockTime::from_mseconds(100));
        debug!(
            "play() -> state: {:?}, pending: {:?}, success: {:?}",
            state, pending, success
        );

        let pos_after = self.pipeline.query_position::<gst::ClockTime>();
        debug!("play() position after: {:?}", pos_after);

        debug!(
            "play() base_time: {:?}, start_time: {:?}",
            self.pipeline.base_time(),
            self.pipeline.start_time()
        );

        result.map_err(|e| AppError::Pipeline(format!("Failed to play: {:?}", e)))?;
        Ok(())
    }

    pub fn pause(&self) -> AppResult<()> {
        debug!(
            "pause() called, current state: {:?}",
            self.pipeline.current_state()
        );
        let result = self.pipeline.set_state(gst::State::Paused);
        debug!("pause() set_state result: {:?}", result);

        let (success, state, pending) = self.pipeline.state(gst::ClockTime::from_seconds(2));
        debug!(
            "pause() -> state: {:?}, pending: {:?}, success: {:?}",
            state, pending, success
        );

        if state != gst::State::Paused {
            warn!(
                "pause() state change incomplete: {:?} (pending: {:?})",
                state, pending
            );
        }

        result.map_err(|e| AppError::Pipeline(format!("Failed to pause: {:?}", e)))?;
        Ok(())
    }

    pub fn stop(&self) -> AppResult<()> {
        debug!(
            "stop() called, current state: {:?}",
            self.pipeline.current_state()
        );
        let result = self.pipeline.set_state(gst::State::Null);
        debug!("stop() set_state result: {:?}", result);

        let (success, state, pending) = self.pipeline.state(gst::ClockTime::from_mseconds(100));
        debug!(
            "stop() -> state: {:?}, pending: {:?}, success: {:?}",
            state, pending, success
        );

        result.map_err(|e| AppError::Pipeline(format!("Failed to stop: {:?}", e)))?;
        Ok(())
    }

    pub fn seek(&self, position_secs: f64) -> AppResult<()> {
        let position = gst::ClockTime::from_seconds_f64(position_secs);
        self.pipeline
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE, position)
            .map_err(|e| AppError::Pipeline(format!("Failed to seek: {:?}", e)))?;
        Ok(())
    }

    // ========================================
    // 明るさ調整
    // ========================================

    pub fn set_master_brightness(&mut self, value: f64) {
        self.master_brightness = value;

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
    // 音量調整
    // ========================================

    pub fn set_master_volume(&mut self, value: f64) {
        self.master_volume = value;
        self.apply_master_volume();
    }

    pub fn master_volume(&self) -> f64 {
        self.master_volume
    }

    // ========================================
    // 状態取得
    // ========================================

    pub fn position(&self) -> Option<f64> {
        // NDI出力がある場合は、NdiSenderのPTSを使用
        if let Some(ndi_sender) = self.ndi_senders.values().next() {
            let pos = ndi_sender.last_position();
            if pos > 0.0 {
                return Some(pos);
            }
        }

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
