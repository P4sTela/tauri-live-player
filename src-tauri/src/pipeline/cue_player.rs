use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

use crate::audio::sink::create_audio_sink;
use crate::error::{AppError, AppResult};
use crate::output::native_handle::{
    create_fallback_sink, create_video_sink_with_handle, NativeHandle,
};
use crate::pipeline::NdiSender;
use crate::types::*;

/// 出力先とモニター情報、ネイティブハンドルを組み合わせた構造体
#[derive(Debug, Clone)]
pub struct OutputWithMonitor {
    pub output: OutputTarget,
    pub monitor: Option<MonitorInfo>,
    pub native_handle: Option<NativeHandle>,
}

pub struct CuePlayer {
    pipeline: gst::Pipeline,
    video_balances: HashMap<String, gst::Element>,
    volume_elements: HashMap<String, gst::Element>,
    /// NDI出力用のNdiSender (output_id -> NdiSender)
    /// appsink + NDI SDK 直接呼び出し方式
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
    ///
    /// # Arguments
    /// * `cue` - 再生するキュー
    /// * `outputs` - 出力先の一覧
    /// * `monitors` - モニター情報の一覧
    /// * `native_handles` - 出力IDとネイティブハンドルのマッピング（OutputManagerから取得）
    pub fn load_cue(
        &mut self,
        cue: &Cue,
        outputs: &[OutputTarget],
        monitors: &[MonitorInfo],
        native_handles: &HashMap<String, NativeHandle>,
    ) -> AppResult<()> {
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
        self.volume_elements.clear();
        self.ndi_senders.clear();
        self.ndi_appsinks.clear();

        // 出力とモニター情報、ネイティブハンドルを組み合わせ
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
                OutputWithMonitor {
                    output: o.clone(),
                    monitor,
                    native_handle,
                }
            })
            .collect();

        // 出力ごとの明るさ設定を保存
        for owm in &outputs_with_monitors {
            self.output_brightness
                .insert(owm.output.id.clone(), owm.output.brightness);
        }

        // NDI出力用のNdiSenderを作成
        // appsink + NDI SDK 直接呼び出し方式
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

            self.add_media_item(item, owm)?;
        }

        // ライブ出力（NDI等）がある場合はパイプラインのlatencyを設定
        // これにより async=true を維持しながらライブ動作が可能になる
        let has_live_output = outputs_with_monitors
            .iter()
            .any(|owm| matches!(owm.output.output_type, OutputType::Ndi));

        if has_live_output {
            // ライブモード: 100msの基準遅延を設定
            let latency = gst::ClockTime::from_mseconds(100);
            self.pipeline.set_latency(latency);
            debug!(
                "[CuePlayer] Live mode enabled: pipeline latency set to {:?}",
                latency
            );
        }

        // PAUSED状態にしてプリロール
        debug!(" Setting pipeline to PAUSED...");
        self.pipeline
            .set_state(gst::State::Paused)
            .map_err(|e| AppError::Pipeline(format!("Failed to pause pipeline: {:?}", e)))?;

        // 状態変更を待機
        let bus = self
            .pipeline
            .bus()
            .ok_or_else(|| AppError::Pipeline("Failed to get bus".to_string()))?;

        debug!(" Waiting for pipeline to preroll...");
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => {
                    debug!(" Pipeline preroll complete (AsyncDone)");
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

        // プリロール後のpositionを確認
        let pos_after_preroll = self.pipeline.query_position::<gst::ClockTime>();
        let dur = self.pipeline.query_duration::<gst::ClockTime>();
        debug!(
            "[CuePlayer] After preroll: position={:?}, duration={:?}",
            pos_after_preroll, dur
        );

        // 動画ファイルのPTSが0から始まっていない場合に対応するため、
        // プリロール完了後に明示的に位置0へシークする
        // これにより再生開始時のposition queryが正しい値を返すようになる
        if let Some(pos) = pos_after_preroll {
            if pos.seconds() > 0 {
                debug!(
                    "[CuePlayer] Non-zero initial position detected ({:?}), seeking to 0",
                    pos
                );
                // FLUSH + ACCURATE で正確に位置0へシーク
                if let Err(e) = self.pipeline.seek_simple(
                    gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                    gst::ClockTime::ZERO,
                ) {
                    warn!("[CuePlayer] Failed to seek to 0: {:?}", e);
                } else {
                    // シーク完了を待機
                    for msg in bus.iter_timed(gst::ClockTime::from_seconds(2)) {
                        if let gst::MessageView::AsyncDone(_) = msg.view() {
                            debug!("[CuePlayer] Seek to 0 complete");
                            break;
                        }
                    }
                }
            }
        }

        // 現在のmaster_volumeを適用（volume要素は初期値1.0で作成されるため）
        self.apply_master_volume();

        Ok(())
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

    /// NDI出力用のNdiSenderとappsinkを作成
    ///
    /// 構造:
    /// [filesrc] → decodebin → videoconvert → videobalance → capsfilter(UYVY) → appsink
    ///                                                                              ↓
    ///                                                                      NdiSender (NDI SDK)
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

    fn add_media_item(&mut self, item: &MediaItem, owm: &OutputWithMonitor) -> AppResult<()> {
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
        let owm_clone = owm.clone();
        let pipeline_weak = self.pipeline.downgrade();
        let brightness = self.get_effective_brightness(&owm.output.id);

        // NDI出力の場合、appsinkへの弱参照を取得
        let appsink_weak = if owm.output.output_type == OutputType::Ndi {
            self.ndi_appsinks.get(&owm.output.id).map(|a| a.downgrade())
        } else {
            None
        };

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
                debug!(
                    "[CuePlayer] Video pad added for '{}' -> output '{}'",
                    item_clone.name, owm_clone.output.name
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
                    owm_clone.output.name, brightness, gst_brightness
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

                // NDI出力の場合
                let is_ndi = owm_clone.output.output_type == OutputType::Ndi;

                if is_ndi {
                    // appsinkベースのNDIパイプライン:
                    // [ソース動画] → convert → balance → capsfilter(UYVY) → appsink → NdiSender

                    let appsink = match appsink_weak.as_ref().and_then(|w| w.upgrade()) {
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

                    if let Err(e) = pipeline.add_many([&convert, &balance, &capsfilter]) {
                        error!("Failed to add video elements to pipeline: {:?}", e);
                        return;
                    }

                    // convert → balance → capsfilter をリンク
                    if let Err(e) = gst::Element::link_many([&convert, &balance, &capsfilter]) {
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
                        owm_clone.output.name
                    );
                } else {
                    // 通常の出力（Display等）
                    let sink = match create_video_sink(&owm_clone) {
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

                    if let Err(e) = pipeline.add_many([&convert, &balance, &sink]) {
                        error!("Failed to add elements to pipeline: {:?}", e);
                        return;
                    }

                    if let Err(e) = gst::Element::link_many([&convert, &balance, &sink]) {
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

                debug!(
                    "[CuePlayer] Video pipeline linked successfully for '{}'",
                    owm_clone.output.name
                );
            } else if name.starts_with("audio/") && item_clone.media_type == MediaType::Video {
                // ビデオアイテムからのオーディオパッド（動画ファイル内蔵オーディオ）は fakesink に捨てる
                // これがないと not-negotiated エラーでパイプラインが停止する
                debug!(
                    "[CuePlayer] Discarding audio pad from video item '{}' with fakesink",
                    item_clone.name
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

                // Volume element with unique name for later access
                let volume_name = format!("volume_{}", owm_clone.output.id);
                let volume = match gst::ElementFactory::make("volume")
                    .name(&volume_name)
                    .property("volume", 1.0_f64)
                    .build()
                {
                    Ok(e) => e,
                    Err(_) => return,
                };

                let sink = match create_audio_sink(&owm_clone.output) {
                    Ok(s) => s,
                    Err(_) => return,
                };

                if pipeline
                    .add_many([&convert, &resample, &volume, &sink])
                    .is_err()
                {
                    return;
                }

                if gst::Element::link_many([&convert, &resample, &volume, &sink]).is_err() {
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
                let _ = volume.sync_state_with_parent();
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
        debug!(
            "play() called, current state: {:?}",
            self.pipeline.current_state()
        );

        // パイプラインのlatencyを確認
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

        // 再生前の position を確認
        let pos_before = self.pipeline.query_position::<gst::ClockTime>();
        debug!("play() position before: {:?}", pos_before);

        let result = self.pipeline.set_state(gst::State::Playing);
        debug!("play() set_state result: {:?}", result);

        // Wait for state change to complete (up to 100ms)
        let (success, state, pending) = self.pipeline.state(gst::ClockTime::from_mseconds(100));
        debug!(
            "play() -> state: {:?}, pending: {:?}, success: {:?}",
            state, pending, success
        );

        // 再生後の position を確認
        let pos_after = self.pipeline.query_position::<gst::ClockTime>();
        debug!("play() position after: {:?}", pos_after);

        // base_timeとstart_timeを確認
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

        // Wait for state change to complete (up to 2 seconds - NDI can be slow)
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

        // Wait for state change to complete (up to 100ms)
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
        // ACCURATE: キーフレームではなく正確な位置にシーク（複数動画の同期のため）
        // KEY_UNITだと動画ごとにキーフレーム位置が異なり同期がずれる
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
    // 音量調整
    // ========================================

    /// Set master volume (0-100)
    pub fn set_master_volume(&mut self, value: f64) {
        self.master_volume = value;
        self.apply_master_volume();
    }

    /// Get current master volume (0-100)
    pub fn master_volume(&self) -> f64 {
        self.master_volume
    }

    // ========================================
    // 状態取得
    // ========================================

    pub fn position(&self) -> Option<f64> {
        // NDI出力がある場合は、NdiSenderのPTS（appsinkで受け取ったフレームのタイムスタンプ）を使用
        // これにより、ndisinkのライブモードによるオフセット問題を回避
        if let Some(ndi_sender) = self.ndi_senders.values().next() {
            let pos = ndi_sender.last_position();
            if pos > 0.0 {
                return Some(pos);
            }
        }

        // NDI出力がない場合、または position が取得できない場合はパイプラインクエリを使用
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

fn create_video_sink(owm: &OutputWithMonitor) -> Result<gst::Element, gst::glib::BoolError> {
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
    }
}
