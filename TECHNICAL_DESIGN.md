# TauriLivePlayer 技術設計書

## 目次

1. [アーキテクチャ概要](#1-アーキテクチャ概要)
2. [プロジェクト構成](#2-プロジェクト構成)
3. [型定義](#3-型定義)
4. [GStreamer パイプライン設計](#4-gstreamer-パイプライン設計)
5. [Rust 実装](#5-rust-実装)
6. [フロントエンド実装](#6-フロントエンド実装)
7. [プラットフォーム別対応](#7-プラットフォーム別対応)
8. [ビルド・デプロイ](#8-ビルドデプロイ)

---

## 1. アーキテクチャ概要

### 全体構成図

```
┌─────────────────────────────────────────────────────────────────┐
│  Tauri Application                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  WebView (React + TypeScript)                             │ │
│  │                                                           │ │
│  │  ┌─────────────┐  ┌─────────────────────────────────────┐│ │
│  │  │  Preview    │  │  Cue List                           ││ │
│  │  │  Grid       │  │  - Opening         ▶ Playing       ││ │
│  │  │             │  │  - Song 01           Ready         ││ │
│  │  └─────────────┘  │  - MC Bridge         Pending       ││ │
│  │                   └─────────────────────────────────────┘│ │
│  │                                                           │ │
│  │  Brightness                                               │ │
│  │  Master: ════════●════ 80%                               │ │
│  │  main:   ════════●════ 80%   [Link]                      │ │
│  │  side:   ══════●══════ 60%   [Link]                      │ │
│  │                                                           │ │
│  │  [◀] [▶ PLAY] [■] [▶▶]                                   │ │
│  │  00:01:23 ════════●════════════════════════════ 04:12    │ │
│  └───────────────────────────────────────────────────────────┘ │
│                          │ Tauri Commands (IPC)                │
│                          ↓                                      │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Rust Backend                                             │ │
│  │                                                           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐ │ │
│  │  │ CuePlayer   │  │ OutputMgr   │  │ GStreamer         │ │ │
│  │  │             │  │             │  │ Pipeline          │ │ │
│  │  └─────────────┘  └─────────────┘  └───────────────────┘ │ │
│  └───────────────────────────────────────────────────────────┘ │
│                          │                                      │
└──────────────────────────┼──────────────────────────────────────┘
                           ↓
        ┌──────────────────┼──────────────────┐
        ↓                  ↓                  ↓
   ┌─────────┐      ┌─────────────┐     ┌──────────┐
   │Display  │      │ NDI Output  │     │ ASIO     │
   │Window   │      │             │     │ Audio    │
   └─────────┘      └─────────────┘     └──────────┘
```

### GStreamer マルチソース同期パイプライン

```
┌─────────────────────────────────────────────────────────────────┐
│  GStreamer Pipeline (単一パイプライン = 自動同期)               │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐│
│  │ filesrc(audio.wav) → decodebin → audioconvert → asiosink  ││
│  └────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐│
│  │ filesrc(main.mp4) → decodebin → videobalance[main] → sink ││
│  └────────────────────────────────────────────────────────────┘│
│                                      ↑                         │
│  ┌────────────────────────────────────────────────────────────┐│
│  │ filesrc(side.mp4) → decodebin → videobalance[side] → ndi  ││
│  └────────────────────────────────────────────────────────────┘│
│                                      ↑                         │
│                         各出力ごとに個別の明るさ(videobalance)  │
│                                                                 │
│  ← 同一 GStreamer クロックで全ソース同期再生 →                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. プロジェクト構成

```
tauri-live-player/
├── src/                              # フロントエンド (React)
│   ├── components/
│   │   ├── cue/
│   │   │   ├── CueList.tsx           # キュー一覧
│   │   │   ├── CueItem.tsx           # キュー行
│   │   │   ├── CueEditor.tsx         # キュー編集パネル
│   │   │   └── MediaItemList.tsx     # アイテム一覧
│   │   ├── output/
│   │   │   ├── OutputManager.tsx     # 出力先管理
│   │   │   ├── OutputBadge.tsx       # 出力先バッジ
│   │   │   └── OutputConfigDialog.tsx
│   │   ├── player/
│   │   │   ├── Controls.tsx          # 再生制御
│   │   │   ├── ProgressBar.tsx       # シークバー
│   │   │   ├── BrightnessSlider.tsx  # 明るさ (Master + 個別)
│   │   │   └── PreviewGrid.tsx       # マルチプレビュー
│   │   ├── project/
│   │   │   ├── ProjectManager.tsx
│   │   │   └── SettingsDialog.tsx
│   │   └── ui/                       # shadcn/ui コンポーネント
│   │       ├── button.tsx
│   │       ├── slider.tsx
│   │       ├── table.tsx
│   │       └── ...
│   ├── stores/
│   │   ├── projectStore.ts           # プロジェクト状態
│   │   └── playerStore.ts            # 再生状態
│   ├── types/
│   │   └── index.ts                  # 型定義
│   ├── hooks/
│   │   ├── useKeyboard.ts            # キーボードショートカット
│   │   └── usePlayer.ts              # プレイヤー操作
│   ├── lib/
│   │   └── utils.ts
│   ├── App.tsx
│   ├── main.tsx
│   └── output.html                   # 出力ウィンドウ用
│
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── player.rs             # 再生制御コマンド
│   │   │   ├── project.rs            # プロジェクト管理
│   │   │   └── output.rs             # 出力管理コマンド
│   │   ├── pipeline/
│   │   │   ├── mod.rs
│   │   │   ├── cue_player.rs         # Cue再生パイプライン
│   │   │   ├── ndi.rs                # NDI送受信
│   │   │   └── preview.rs            # プレビュー生成
│   │   ├── output/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs            # 出力ウィンドウ管理
│   │   │   └── window.rs             # ウィンドウ制御
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   └── sink.rs               # オーディオ出力 (ASIO等)
│   │   ├── types.rs                  # Rust型定義
│   │   ├── state.rs                  # アプリ状態
│   │   ├── error.rs                  # エラー型
│   │   └── main.rs                   # エントリポイント
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
├── .github/
│   └── workflows/
│       └── build.yml                 # CI/CD
│
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── README.md
```

---

## 3. 型定義

### 3.1 TypeScript 型定義

```typescript
// src/types/index.ts

// ========================================
// メディアアイテム（1つのファイル）
// ========================================
export interface MediaItem {
  id: string;
  type: 'video' | 'audio';
  name: string;
  path: string;
  outputId: string;
  offset?: number;        // 開始オフセット（秒）
  trimStart?: number;     // トリム開始位置
  trimEnd?: number;       // トリム終了位置
}

// ========================================
// キュー（同期再生するメディアのグループ）
// ========================================
export interface Cue {
  id: string;
  name: string;
  items: MediaItem[];
  duration: number;       // 最長アイテムの長さ
  loop: boolean;
  autoAdvance: boolean;   // 終了時に次のキューへ
  color?: string;         // UI表示用カラー
}

// ========================================
// 出力先の定義
// ========================================
export type OutputType = 'display' | 'ndi' | 'audio';
export type AudioDriver = 'auto' | 'asio' | 'wasapi' | 'coreaudio' | 'jack' | 'alsa';

export interface OutputTarget {
  id: string;
  name: string;
  type: OutputType;
  
  // 映像出力共通
  brightness?: number | null;  // null = Masterに連動、number = 個別値
  
  // Display用
  displayIndex?: number;
  fullscreen?: boolean;
  
  // NDI用
  ndiName?: string;
  
  // Audio用
  audioDriver?: AudioDriver;
  audioDevice?: string;
  audioChannels?: number[];
}

// ========================================
// プロジェクト
// ========================================
export interface ProjectSettings {
  defaultBrightness: number;
  autoSave: boolean;
  previewQuality: 'low' | 'medium' | 'high';
}

export interface Project {
  id: string;
  name: string;
  masterBrightness: number;
  outputs: OutputTarget[];
  cues: Cue[];
  settings: ProjectSettings;
}

// ========================================
// プレイヤー状態
// ========================================
export type PlayerStatus = 'idle' | 'loading' | 'ready' | 'playing' | 'paused' | 'error';

export interface PlayerState {
  status: PlayerStatus;
  currentCueIndex: number;
  currentTime: number;
  duration: number;
  error?: string;
}

// ========================================
// モニター情報
// ========================================
export interface MonitorInfo {
  index: number;
  name: string;
  width: number;
  height: number;
  x: number;
  y: number;
  isPrimary: boolean;
}

// ========================================
// NDIソース情報
// ========================================
export interface NdiSource {
  name: string;
  urlAddress: string;
}
```

### 3.2 Rust 型定義

```rust
// src-tauri/src/types.rs

use serde::{Deserialize, Serialize};

// ========================================
// メディアアイテム
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub name: String,
    pub path: String,
    pub output_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trim_end: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Audio,
}

// ========================================
// キュー
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    pub name: String,
    pub items: Vec<MediaItem>,
    pub duration: f64,
    #[serde(rename = "loop")]
    pub loop_playback: bool,
    pub auto_advance: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

// ========================================
// 出力先
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTarget {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: OutputType,
    
    // 明るさ (None = Master連動)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f64>,
    
    // Display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fullscreen: Option<bool>,
    
    // NDI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndi_name: Option<String>,
    
    // Audio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_driver: Option<AudioDriver>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Display,
    Ndi,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioDriver {
    Auto,
    Asio,
    Wasapi,
    #[serde(rename = "coreaudio")]
    CoreAudio,
    Jack,
    Alsa,
}

// ========================================
// プロジェクト
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub master_brightness: f64,
    pub outputs: Vec<OutputTarget>,
    pub cues: Vec<Cue>,
}

// ========================================
// プレイヤー状態
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerStatus {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: PlayerStatus,
    pub current_cue_index: i32,
    pub current_time: f64,
    pub duration: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ========================================
// モニター情報
// ========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}
```

---

## 4. GStreamer パイプライン設計

### 4.1 基本再生パイプライン

```
filesrc location=video.mp4
    ↓
decodebin  ─────────────────┐
    │                       │
    ↓ (video pad)           ↓ (audio pad)
videoconvert            audioconvert
    ↓                       ↓
videobalance            audioresample
(brightness)                ↓
    ↓                   autoaudiosink
autovideosink
```

**gst-launch-1.0 での確認:**
```bash
gst-launch-1.0 \
  filesrc location=test.mp4 ! \
  decodebin name=d \
  d. ! videoconvert ! videobalance brightness=0.0 ! autovideosink \
  d. ! audioconvert ! autoaudiosink
```

### 4.2 マルチソースパイプライン

```
Pipeline
├── filesrc(audio.wav) → decodebin → audioconvert → asiosink
├── filesrc(main.mp4)  → decodebin → videoconvert → videobalance[id=main] → sink[Display1]
├── filesrc(side.mp4)  → decodebin → videoconvert → videobalance[id=side] → ndisink
└── filesrc(floor.mp4) → decodebin → videoconvert → videobalance[id=floor] → sink[Display2]
```

### 4.3 NDI パイプライン

**送信:**
```
filesrc → decodebin → videoconvert → video/x-raw,format=UYVY → ndisink ndi-name="LivePlayer"
```

**受信:**
```
ndisrc ndi-name="SOURCE" → ndisrcdemux name=d
d.video → videoconvert → autovideosink
d.audio → audioconvert → autoaudiosink
```

### 4.4 マルチスクリーン合成

```
compositor name=c
  sink_0::xpos=0    sink_0::ypos=0
  sink_1::xpos=1920 sink_1::ypos=0
  sink_2::xpos=0    sink_2::ypos=1080
  sink_3::xpos=1920 sink_3::ypos=1080
    ↓
video/x-raw,width=3840,height=2160
    ↓
ndisink ndi-name="MULTI_SCREEN"
```

---

## 5. Rust 実装

### 5.1 Cargo.toml

```toml
[package]
name = "tauri-live-player"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"

serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
thiserror = "1"
uuid = { version = "1", features = ["v4"] }
parking_lot = "0.12"

# GStreamer
gstreamer = "0.23"
gstreamer-video = "0.23"
gstreamer-audio = "0.23"
gstreamer-app = "0.23"
gstreamer-pbutils = "0.23"

[target.'cfg(windows)'.dependencies]
# Windows固有

[target.'cfg(target_os = "macos")'.dependencies]
# macOS固有

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

### 5.2 CuePlayer 実装

```rust
// src-tauri/src/pipeline/cue_player.rs

use gstreamer as gst;
use gstreamer::prelude::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::*;
use crate::audio::sink::create_audio_sink;

pub struct CuePlayer {
    pipeline: gst::Pipeline,
    video_balances: HashMap<String, gst::Element>,
    master_brightness: f64,
    output_brightness: HashMap<String, Option<f64>>,
}

impl CuePlayer {
    pub fn new() -> Result<Self, gst::glib::Error> {
        gst::init()?;
        let pipeline = gst::Pipeline::new();
        
        Ok(Self {
            pipeline,
            video_balances: HashMap::new(),
            master_brightness: 1.0,
            output_brightness: HashMap::new(),
        })
    }
    
    /// Cueを読み込んでパイプラインを構築
    pub fn load_cue(
        &mut self,
        cue: &Cue,
        outputs: &[OutputTarget],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // パイプラインをリセット
        self.pipeline.set_state(gst::State::Null)?;
        
        // 既存のエレメントを削除
        for element in self.pipeline.iterate_elements() {
            if let Ok(el) = element {
                self.pipeline.remove(&el).ok();
            }
        }
        self.video_balances.clear();
        
        // 出力ごとの明るさ設定を保存
        for output in outputs {
            self.output_brightness.insert(output.id.clone(), output.brightness);
        }
        
        // 各メディアアイテムを追加
        for item in &cue.items {
            let output = outputs.iter()
                .find(|o| o.id == item.output_id)
                .ok_or_else(|| format!("Output not found: {}", item.output_id))?;
            
            self.add_media_item(item, output)?;
        }
        
        // PAUSED状態にしてプリロール
        self.pipeline.set_state(gst::State::Paused)?;
        
        // 状態変更を待機
        let bus = self.pipeline.bus().unwrap();
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => break,
                gst::MessageView::Error(err) => {
                    return Err(format!(
                        "Pipeline error: {} ({:?})",
                        err.error(),
                        err.debug()
                    ).into());
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    fn add_media_item(
        &mut self,
        item: &MediaItem,
        output: &OutputTarget,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // ソースエレメント
        let src = gst::ElementFactory::make("filesrc")
            .property("location", &item.path)
            .build()?;
        
        let decode = gst::ElementFactory::make("decodebin").build()?;
        
        self.pipeline.add_many([&src, &decode])?;
        src.link(&decode)?;
        
        // 動的パッドのためのクロージャ用変数
        let item_clone = item.clone();
        let output_clone = output.clone();
        let pipeline_weak = self.pipeline.downgrade();
        let brightness = self.get_effective_brightness(&output.id);
        let output_id = output.id.clone();
        
        // video_balances への参照を作るため、Arc<Mutex>を使用
        let video_balances = Arc::new(Mutex::new(HashMap::<String, gst::Element>::new()));
        let video_balances_clone = video_balances.clone();
        
        decode.connect_pad_added(move |_, src_pad| {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            
            let caps = match src_pad.current_caps() {
                Some(c) => c,
                None => return,
            };
            let structure = caps.structure(0).unwrap();
            let name = structure.name();
            
            if name.starts_with("video/") && item_clone.media_type == MediaType::Video {
                // ビデオ処理チェーン
                let convert = gst::ElementFactory::make("videoconvert")
                    .build()
                    .unwrap();
                
                let balance = gst::ElementFactory::make("videobalance")
                    .property("brightness", brightness - 1.0)
                    .build()
                    .unwrap();
                
                let sink = Self::create_video_sink(&output_clone).unwrap();
                
                pipeline.add_many([&convert, &balance, &sink]).unwrap();
                gst::Element::link_many([&convert, &balance, &sink]).unwrap();
                
                let sink_pad = convert.static_pad("sink").unwrap();
                src_pad.link(&sink_pad).unwrap();
                
                convert.sync_state_with_parent().unwrap();
                balance.sync_state_with_parent().unwrap();
                sink.sync_state_with_parent().unwrap();
                
                // video_balance を保存
                video_balances_clone.lock().insert(output_id.clone(), balance);
                
            } else if name.starts_with("audio/") && item_clone.media_type == MediaType::Audio {
                // オーディオ処理チェーン
                let convert = gst::ElementFactory::make("audioconvert")
                    .build()
                    .unwrap();
                
                let resample = gst::ElementFactory::make("audioresample")
                    .build()
                    .unwrap();
                
                let sink = create_audio_sink(&output_clone).unwrap();
                
                pipeline.add_many([&convert, &resample, &sink]).unwrap();
                gst::Element::link_many([&convert, &resample, &sink]).unwrap();
                
                let sink_pad = convert.static_pad("sink").unwrap();
                src_pad.link(&sink_pad).unwrap();
                
                convert.sync_state_with_parent().unwrap();
                resample.sync_state_with_parent().unwrap();
                sink.sync_state_with_parent().unwrap();
            }
        });
        
        // 外部で作成したHashMapをselfにマージ
        // (実際にはconnect_pad_addedのコールバック内で非同期に追加される)
        
        Ok(())
    }
    
    fn create_video_sink(output: &OutputTarget) -> Result<gst::Element, gst::glib::Error> {
        match output.output_type {
            OutputType::Display => {
                // TODO: 特定ディスプレイへの出力
                gst::ElementFactory::make("autovideosink").build()
            }
            OutputType::Ndi => {
                let sink = gst::ElementFactory::make("ndisink")
                    .property("ndi-name", output.ndi_name.as_ref().unwrap())
                    .build()?;
                Ok(sink)
            }
            OutputType::Audio => {
                Err(gst::glib::Error::new(
                    gst::CoreError::Failed,
                    "Audio output cannot be used as video sink",
                ))
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
    
    pub fn play(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }
    
    pub fn pause(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Paused)?;
        Ok(())
    }
    
    pub fn stop(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }
    
    pub fn seek(&self, position_secs: f64) -> Result<(), gst::glib::BoolError> {
        let position = gst::ClockTime::from_seconds_f64(position_secs);
        self.pipeline.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            position,
        )?;
        Ok(())
    }
    
    // ========================================
    // 明るさ調整
    // ========================================
    
    pub fn set_master_brightness(&mut self, value: f64) {
        self.master_brightness = value;
        
        // Master連動の出力を更新
        for (output_id, balance) in &self.video_balances {
            if self.output_brightness.get(output_id).map(|b| b.is_none()).unwrap_or(true) {
                balance.set_property("brightness", value - 1.0);
            }
        }
    }
    
    pub fn set_output_brightness(&mut self, output_id: &str, value: Option<f64>) {
        self.output_brightness.insert(output_id.to_string(), value);
        
        if let Some(balance) = self.video_balances.get(output_id) {
            let effective = value.unwrap_or(self.master_brightness);
            balance.set_property("brightness", effective - 1.0);
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
```

### 5.3 オーディオシンク (ASIO対応)

```rust
// src-tauri/src/audio/sink.rs

use gstreamer as gst;
use crate::types::{AudioDriver, OutputTarget};

pub fn create_audio_sink(config: &OutputTarget) -> Result<gst::Element, gst::glib::Error> {
    let driver = config.audio_driver.clone().unwrap_or(AudioDriver::Auto);
    
    match driver {
        #[cfg(target_os = "windows")]
        AudioDriver::Asio => create_asio_sink(config),
        
        #[cfg(target_os = "windows")]
        AudioDriver::Wasapi => create_wasapi_sink(),
        
        #[cfg(target_os = "windows")]
        AudioDriver::Auto => {
            // ASIO を優先、なければ WASAPI
            if gst::ElementFactory::find("asiosink").is_some() {
                create_asio_sink(config)
            } else {
                create_wasapi_sink()
            }
        }
        
        #[cfg(target_os = "macos")]
        AudioDriver::CoreAudio | AudioDriver::Auto => {
            gst::ElementFactory::make("osxaudiosink").build()
        }
        
        #[cfg(target_os = "linux")]
        AudioDriver::Jack => {
            gst::ElementFactory::make("jackaudiosink").build()
        }
        
        #[cfg(target_os = "linux")]
        AudioDriver::Alsa => {
            gst::ElementFactory::make("alsasink").build()
        }
        
        #[cfg(target_os = "linux")]
        AudioDriver::Auto => {
            // JACK を優先、なければ ALSA
            if gst::ElementFactory::find("jackaudiosink").is_some() {
                gst::ElementFactory::make("jackaudiosink").build()
            } else {
                gst::ElementFactory::make("alsasink").build()
            }
        }
        
        _ => gst::ElementFactory::make("autoaudiosink").build(),
    }
}

#[cfg(target_os = "windows")]
fn create_asio_sink(config: &OutputTarget) -> Result<gst::Element, gst::glib::Error> {
    let sink = gst::ElementFactory::make("asiosink").build()?;
    
    if let Some(device) = &config.audio_device {
        sink.set_property("device-clsid", device);
    }
    
    if let Some(channels) = &config.audio_channels {
        let ch_str = channels
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sink.set_property("output-channels", &ch_str);
    }
    
    Ok(sink)
}

#[cfg(target_os = "windows")]
fn create_wasapi_sink() -> Result<gst::Element, gst::glib::Error> {
    gst::ElementFactory::make("wasapisink")
        .property("low-latency", true)
        .build()
}

/// ASIOデバイス一覧を取得
#[cfg(target_os = "windows")]
pub fn list_asio_devices() -> Vec<AsioDevice> {
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);
    
    if monitor.start().is_err() {
        return Vec::new();
    }
    
    let devices = monitor.devices();
    monitor.stop();
    
    devices
        .iter()
        .filter_map(|d| {
            let props = d.properties()?;
            let api = props.get::<String>("device.api").ok()?;
            
            if api == "asio" {
                Some(AsioDevice {
                    name: d.display_name().to_string(),
                    clsid: props.get::<String>("device.clsid").ok()?,
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AsioDevice {
    pub name: String,
    pub clsid: String,
}
```

### 5.4 出力ウィンドウ管理

```rust
// src-tauri/src/output/manager.rs

use std::collections::HashMap;
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};
use crate::types::*;

pub struct OutputManager {
    outputs: HashMap<String, OutputWindow>,
}

struct OutputWindow {
    id: String,
    window: Option<WebviewWindow>,
    output_type: OutputType,
}

impl OutputManager {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }
    
    pub fn create_output(
        &mut self,
        app: &AppHandle,
        config: &OutputTarget,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match config.output_type {
            OutputType::Display => {
                let monitors: Vec<_> = app.available_monitors()?.collect();
                let monitor = monitors
                    .get(config.display_index.unwrap_or(0))
                    .ok_or("Monitor not found")?;
                
                let position = monitor.position();
                let size = monitor.size();
                
                let window = WebviewWindowBuilder::new(
                    app,
                    &format!("output_{}", config.id),
                    tauri::WebviewUrl::App("output.html".into()),
                )
                .title(&config.name)
                .position(position.x as f64, position.y as f64)
                .inner_size(size.width as f64, size.height as f64)
                .fullscreen(config.fullscreen.unwrap_or(true))
                .decorations(false)
                .always_on_top(true)
                .build()?;
                
                self.outputs.insert(
                    config.id.clone(),
                    OutputWindow {
                        id: config.id.clone(),
                        window: Some(window),
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
                        window: None,
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
                        window: None,
                        output_type: OutputType::Audio,
                    },
                );
            }
        }
        
        Ok(())
    }
    
    pub fn close_output(&mut self, id: &str) {
        if let Some(output) = self.outputs.remove(id) {
            if let Some(window) = output.window {
                let _ = window.close();
            }
        }
    }
    
    pub fn close_all(&mut self) {
        for (_, output) in self.outputs.drain() {
            if let Some(window) = output.window {
                let _ = window.close();
            }
        }
    }
    
    pub fn get_monitor_list(app: &AppHandle) -> Result<Vec<MonitorInfo>, Box<dyn std::error::Error>> {
        let primary = app.primary_monitor()?;
        let monitors: Vec<_> = app.available_monitors()?.collect();
        
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
                    name: m.name().unwrap_or_default().to_string(),
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
```

### 5.5 Tauri コマンド

```rust
// src-tauri/src/commands/player.rs

use tauri::State;
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn load_cue(
    state: State<'_, AppState>,
    cue_index: usize,
) -> Result<(), String> {
    let mut player = state.player.lock();
    let project = state.project.lock();
    
    let project = project.as_ref().ok_or("No project loaded")?;
    let cue = project.cues.get(cue_index).ok_or("Cue not found")?;
    
    player
        .load_cue(cue, &project.outputs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn play(state: State<'_, AppState>) -> Result<(), String> {
    let player = state.player.lock();
    player.play().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>) -> Result<(), String> {
    let player = state.player.lock();
    player.pause().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop(state: State<'_, AppState>) -> Result<(), String> {
    let player = state.player.lock();
    player.stop().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    let player = state.player.lock();
    player.seek(position).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_master_brightness(
    state: State<'_, AppState>,
    value: f64,
) -> Result<(), String> {
    let mut player = state.player.lock();
    player.set_master_brightness(value);
    Ok(())
}

#[tauri::command]
pub async fn set_output_brightness(
    state: State<'_, AppState>,
    output_id: String,
    value: Option<f64>,
) -> Result<(), String> {
    let mut player = state.player.lock();
    player.set_output_brightness(&output_id, value);
    Ok(())
}

#[tauri::command]
pub async fn get_player_state(state: State<'_, AppState>) -> Result<PlayerState, String> {
    let player = state.player.lock();
    
    let status = match player.state() {
        gstreamer::State::Null => PlayerStatus::Idle,
        gstreamer::State::Ready => PlayerStatus::Ready,
        gstreamer::State::Paused => PlayerStatus::Paused,
        gstreamer::State::Playing => PlayerStatus::Playing,
        _ => PlayerStatus::Idle,
    };
    
    Ok(PlayerState {
        status,
        current_cue_index: -1, // TODO: track current cue
        current_time: player.position().unwrap_or(0.0),
        duration: player.duration().unwrap_or(0.0),
        error: None,
    })
}
```

```rust
// src-tauri/src/commands/output.rs

use tauri::{AppHandle, State};
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn get_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    crate::output::manager::OutputManager::get_monitor_list(&app)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_output_window(
    state: State<'_, AppState>,
    app: AppHandle,
    config: OutputTarget,
) -> Result<(), String> {
    let mut manager = state.output_manager.lock();
    manager.create_output(&app, &config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_output_window(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut manager = state.output_manager.lock();
    manager.close_output(&id);
    Ok(())
}

#[tauri::command]
pub async fn close_all_outputs(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.output_manager.lock();
    manager.close_all();
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn list_asio_devices() -> Result<Vec<crate::audio::sink::AsioDevice>, String> {
    Ok(crate::audio::sink::list_asio_devices())
}
```

### 5.6 アプリ状態

```rust
// src-tauri/src/state.rs

use parking_lot::Mutex;
use crate::pipeline::cue_player::CuePlayer;
use crate::output::manager::OutputManager;
use crate::types::Project;

pub struct AppState {
    pub player: Mutex<CuePlayer>,
    pub output_manager: Mutex<OutputManager>,
    pub project: Mutex<Option<Project>>,
}

impl AppState {
    pub fn new() -> Result<Self, gstreamer::glib::Error> {
        Ok(Self {
            player: Mutex::new(CuePlayer::new()?),
            output_manager: Mutex::new(OutputManager::new()),
            project: Mutex::new(None),
        })
    }
}
```

### 5.7 main.rs

```rust
// src-tauri/src/main.rs

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod output;
mod pipeline;
mod state;
mod types;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new().expect("Failed to initialize app state"))
        .invoke_handler(tauri::generate_handler![
            // Player
            commands::player::load_cue,
            commands::player::play,
            commands::player::pause,
            commands::player::stop,
            commands::player::seek,
            commands::player::set_master_brightness,
            commands::player::set_output_brightness,
            commands::player::get_player_state,
            // Output
            commands::output::get_monitors,
            commands::output::open_output_window,
            commands::output::close_output_window,
            commands::output::close_all_outputs,
            #[cfg(target_os = "windows")]
            commands::output::list_asio_devices,
            // Project
            commands::project::load_project,
            commands::project::save_project,
            commands::project::new_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## 6. フロントエンド実装

### 6.1 Zustand Store

```typescript
// src/stores/projectStore.ts

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { Project, Cue, MediaItem, OutputTarget } from '../types';

interface ProjectStore {
  project: Project | null;
  isDirty: boolean;
  
  // Project actions
  loadProject: (path: string) => Promise<void>;
  saveProject: (path?: string) => Promise<void>;
  newProject: (name: string) => void;
  
  // Cue actions
  addCue: (cue: Cue) => void;
  updateCue: (id: string, updates: Partial<Cue>) => void;
  removeCue: (id: string) => void;
  reorderCues: (fromIndex: number, toIndex: number) => void;
  
  // Item actions
  addItemToCue: (cueId: string, item: MediaItem) => void;
  updateItem: (cueId: string, itemId: string, updates: Partial<MediaItem>) => void;
  removeItem: (cueId: string, itemId: string) => void;
  
  // Output actions
  addOutput: (output: OutputTarget) => void;
  updateOutput: (id: string, updates: Partial<OutputTarget>) => void;
  removeOutput: (id: string) => void;
  
  // Brightness
  setMasterBrightness: (value: number) => void;
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  project: null,
  isDirty: false,
  
  loadProject: async (path) => {
    const project = await invoke<Project>('load_project', { path });
    set({ project, isDirty: false });
  },
  
  saveProject: async (path) => {
    const { project } = get();
    if (!project) return;
    await invoke('save_project', { project, path });
    set({ isDirty: false });
  },
  
  newProject: (name) => {
    const project: Project = {
      id: crypto.randomUUID(),
      name,
      masterBrightness: 100,
      outputs: [],
      cues: [],
      settings: {
        defaultBrightness: 100,
        autoSave: true,
        previewQuality: 'medium',
      },
    };
    set({ project, isDirty: false });
  },
  
  addCue: (cue) => {
    set((state) => ({
      project: state.project
        ? { ...state.project, cues: [...state.project.cues, cue] }
        : null,
      isDirty: true,
    }));
  },
  
  updateCue: (id, updates) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === id ? { ...c, ...updates } : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },
  
  removeCue: (id) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.filter((c) => c.id !== id),
          }
        : null,
      isDirty: true,
    }));
  },
  
  reorderCues: (fromIndex, toIndex) => {
    set((state) => {
      if (!state.project) return state;
      const cues = [...state.project.cues];
      const [removed] = cues.splice(fromIndex, 1);
      cues.splice(toIndex, 0, removed);
      return {
        project: { ...state.project, cues },
        isDirty: true,
      };
    });
  },
  
  addItemToCue: (cueId, item) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === cueId ? { ...c, items: [...c.items, item] } : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },
  
  updateItem: (cueId, itemId, updates) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === cueId
                ? {
                    ...c,
                    items: c.items.map((i) =>
                      i.id === itemId ? { ...i, ...updates } : i
                    ),
                  }
                : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },
  
  removeItem: (cueId, itemId) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            cues: state.project.cues.map((c) =>
              c.id === cueId
                ? { ...c, items: c.items.filter((i) => i.id !== itemId) }
                : c
            ),
          }
        : null,
      isDirty: true,
    }));
  },
  
  addOutput: (output) => {
    set((state) => ({
      project: state.project
        ? { ...state.project, outputs: [...state.project.outputs, output] }
        : null,
      isDirty: true,
    }));
  },
  
  updateOutput: (id, updates) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            outputs: state.project.outputs.map((o) =>
              o.id === id ? { ...o, ...updates } : o
            ),
          }
        : null,
      isDirty: true,
    }));
  },
  
  removeOutput: (id) => {
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            outputs: state.project.outputs.filter((o) => o.id !== id),
          }
        : null,
      isDirty: true,
    }));
  },
  
  setMasterBrightness: (value) => {
    invoke('set_master_brightness', { value });
    set((state) => ({
      project: state.project
        ? { ...state.project, masterBrightness: value }
        : null,
    }));
  },
}));
```

```typescript
// src/stores/playerStore.ts

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { PlayerState, PlayerStatus } from '../types';

interface PlayerStore {
  status: PlayerStatus;
  currentCueIndex: number;
  currentTime: number;
  duration: number;
  error: string | null;
  
  // Actions
  loadCue: (index: number) => Promise<void>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (time: number) => Promise<void>;
  next: () => Promise<void>;
  prev: () => Promise<void>;
  
  // State sync
  syncState: () => Promise<void>;
}

export const usePlayerStore = create<PlayerStore>((set, get) => ({
  status: 'idle',
  currentCueIndex: -1,
  currentTime: 0,
  duration: 0,
  error: null,
  
  loadCue: async (index) => {
    try {
      set({ status: 'loading', error: null });
      await invoke('load_cue', { cueIndex: index });
      set({ status: 'ready', currentCueIndex: index });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },
  
  play: async () => {
    try {
      await invoke('play');
      set({ status: 'playing' });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },
  
  pause: async () => {
    try {
      await invoke('pause');
      set({ status: 'paused' });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },
  
  stop: async () => {
    try {
      await invoke('stop');
      set({ status: 'idle', currentTime: 0 });
    } catch (e) {
      set({ status: 'error', error: String(e) });
    }
  },
  
  seek: async (time) => {
    try {
      await invoke('seek', { position: time });
      set({ currentTime: time });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  
  next: async () => {
    const { currentCueIndex, loadCue } = get();
    await loadCue(currentCueIndex + 1);
  },
  
  prev: async () => {
    const { currentCueIndex, loadCue } = get();
    if (currentCueIndex > 0) {
      await loadCue(currentCueIndex - 1);
    }
  },
  
  syncState: async () => {
    try {
      const state = await invoke<PlayerState>('get_player_state');
      set({
        status: state.status,
        currentTime: state.currentTime,
        duration: state.duration,
        error: state.error ?? null,
      });
    } catch (e) {
      // ignore sync errors
    }
  },
}));
```

### 6.2 キーボードショートカット

```typescript
// src/hooks/useKeyboard.ts

import { useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useProjectStore } from '../stores/projectStore';

export function useKeyboard() {
  const { play, pause, stop, seek, next, prev, status, currentTime } = usePlayerStore();
  const { saveProject } = useProjectStore();
  
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 入力フィールドでは無効
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }
      
      switch (e.code) {
        case 'Space':
          e.preventDefault();
          if (status === 'playing') {
            pause();
          } else {
            play();
          }
          break;
          
        case 'Escape':
          stop();
          break;
          
        case 'ArrowLeft':
          seek(Math.max(0, currentTime - 5));
          break;
          
        case 'ArrowRight':
          seek(currentTime + 5);
          break;
          
        case 'ArrowUp':
          e.preventDefault();
          // Cue選択を上に
          break;
          
        case 'ArrowDown':
          e.preventDefault();
          // Cue選択を下に
          break;
          
        case 'PageUp':
          prev();
          break;
          
        case 'PageDown':
          next();
          break;
          
        case 'Home':
          seek(0);
          break;
          
        case 'KeyS':
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            saveProject();
          }
          break;
      }
    };
    
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [play, pause, stop, seek, next, prev, status, currentTime, saveProject]);
}
```

### 6.3 明るさスライダーコンポーネント

```typescript
// src/components/player/BrightnessSlider.tsx

import React from 'react';
import { Slider } from '../ui/slider';
import { Button } from '../ui/button';
import { Link2, Link2Off } from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { invoke } from '@tauri-apps/api/core';

interface BrightnessSliderProps {
  outputId?: string;  // undefined = Master
  label: string;
}

export function BrightnessSlider({ outputId, label }: BrightnessSliderProps) {
  const { project, setMasterBrightness, updateOutput } = useProjectStore();
  
  if (!project) return null;
  
  const isMaster = !outputId;
  const output = outputId ? project.outputs.find(o => o.id === outputId) : null;
  
  const value = isMaster
    ? project.masterBrightness
    : output?.brightness ?? project.masterBrightness;
  
  const isLinked = output?.brightness == null;
  
  const handleChange = async (newValue: number[]) => {
    const val = newValue[0];
    
    if (isMaster) {
      setMasterBrightness(val);
    } else if (outputId) {
      await invoke('set_output_brightness', { outputId, value: val });
      updateOutput(outputId, { brightness: val });
    }
  };
  
  const toggleLink = () => {
    if (!outputId) return;
    
    if (isLinked) {
      // Unlink: 現在のMaster値を個別値として設定
      updateOutput(outputId, { brightness: project.masterBrightness });
    } else {
      // Link: Master連動に戻す
      invoke('set_output_brightness', { outputId, value: null });
      updateOutput(outputId, { brightness: null });
    }
  };
  
  return (
    <div className="flex items-center gap-4">
      <span className="w-16 text-sm text-muted-foreground">{label}:</span>
      <Slider
        value={[value]}
        onValueChange={handleChange}
        min={0}
        max={100}
        step={1}
        className="flex-1"
        disabled={!isMaster && isLinked}
      />
      <span className="w-12 text-sm text-right">{value}%</span>
      {!isMaster && (
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleLink}
          title={isLinked ? 'Unlink from Master' : 'Link to Master'}
        >
          {isLinked ? <Link2 className="h-4 w-4" /> : <Link2Off className="h-4 w-4" />}
        </Button>
      )}
    </div>
  );
}
```

---

## 7. プラットフォーム別対応

### 7.1 オーディオドライバ対応表

| OS | ドライバ | GStreamer Element | レイテンシ | 備考 |
|----|---------|-------------------|-----------|------|
| **Windows** | **ASIO** | `asiosink` | **~3ms** | **必須要件** |
| Windows | WASAPI | `wasapisink` | ~10ms | フォールバック |
| macOS | Core Audio | `osxaudiosink` | ~10ms | 標準 |
| Linux | JACK | `jackaudiosink` | ~5ms | プロ向け |
| Linux | ALSA | `alsasink` | ~20ms | 標準 |
| Linux | PulseAudio | `pulsesink` | ~50ms | デスクトップ |

### 7.2 HWエンコーダ対応表

| OS | エンコーダ | GStreamer Element | 備考 |
|----|-----------|-------------------|------|
| Windows | NVIDIA NVENC | `nvh264enc` | |
| Windows | Intel QSV | `qsvh264enc` | |
| Windows | AMD AMF | `amfh264enc` | |
| macOS | VideoToolbox | `vtenc_h264` | |
| Linux | VA-API | `vaapih264enc` | Intel/AMD |
| Linux | NVIDIA | `nvh264enc` | |
| 全OS | CPU | `x264enc` | フォールバック |

### 7.3 HWエンコーダ自動検出

```rust
// src-tauri/src/pipeline/encoder.rs

pub fn detect_hw_encoder() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        return "vtenc_h264";
    }
    
    #[cfg(target_os = "windows")]
    {
        if gst::ElementFactory::find("nvh264enc").is_some() {
            return "nvh264enc";
        }
        if gst::ElementFactory::find("qsvh264enc").is_some() {
            return "qsvh264enc";
        }
        if gst::ElementFactory::find("amfh264enc").is_some() {
            return "amfh264enc";
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if gst::ElementFactory::find("vaapih264enc").is_some() {
            return "vaapih264enc";
        }
        if gst::ElementFactory::find("nvh264enc").is_some() {
            return "nvh264enc";
        }
    }
    
    // フォールバック
    "x264enc tune=zerolatency"
}
```

---

## 8. ビルド・デプロイ

### 8.1 GitHub Actions CI/CD

```yaml
# .github/workflows/build.yml

name: Build

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            name: Windows
          - os: macos-latest
            target: aarch64-apple-darwin
            name: macOS-ARM
          - os: macos-latest
            target: x86_64-apple-darwin
            name: macOS-Intel
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            name: Linux

    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      
      # GStreamer インストール
      - name: Install GStreamer (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          choco install gstreamer gstreamer-devel -y
          echo "GSTREAMER_1_0_ROOT_MSVC_X86_64=C:\gstreamer\1.0\msvc_x86_64" >> $env:GITHUB_ENV
          echo "C:\gstreamer\1.0\msvc_x86_64\bin" >> $env:GITHUB_PATH
      
      - name: Install GStreamer (macOS)
        if: matrix.os == 'macos-latest'
        run: |
          brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad
      
      - name: Install GStreamer (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libgstreamer1.0-dev \
            libgstreamer-plugins-base1.0-dev \
            gstreamer1.0-plugins-base \
            gstreamer1.0-plugins-good \
            gstreamer1.0-plugins-bad \
            libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev
      
      # Rust
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      
      # Node.js
      - uses: pnpm/action-setup@v2
        with:
          version: 9
      
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
      
      # ビルド
      - run: pnpm install
      - run: pnpm tauri build --target ${{ matrix.target }}
      
      # アーティファクト
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.name }}-build
          path: |
            src-tauri/target/${{ matrix.target }}/release/bundle/msi/*.msi
            src-tauri/target/${{ matrix.target }}/release/bundle/dmg/*.dmg
            src-tauri/target/${{ matrix.target }}/release/bundle/deb/*.deb
            src-tauri/target/${{ matrix.target }}/release/bundle/appimage/*.AppImage
  
  release:
    needs: build
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/download-artifact@v4
      
      - uses: softprops/action-gh-release@v1
        with:
          files: |
            **/*.msi
            **/*.dmg
            **/*.deb
            **/*.AppImage
```

### 8.2 tauri.conf.json

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "TauriLivePlayer",
  "version": "0.1.0",
  "identifier": "com.tauriliveplayer.app",
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "TauriLivePlayer",
        "width": 1400,
        "height": 900,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "wix": {
        "language": "ja-JP"
      }
    },
    "macOS": {
      "minimumSystemVersion": "10.15"
    },
    "linux": {
      "appimage": {
        "bundleMediaFramework": true
      }
    }
  }
}
```

---

## 9. マルチモニター出力設計（アプローチ2）

### 9.1 概要

GStreamerの映像を特定のモニターに出力するための設計。Tauriで出力ウィンドウを作成し、ネイティブウィンドウハンドルをGStreamerシンクに渡すことで、フレームコピーなしで直接レンダリングを実現する。

### 9.2 アプローチ比較

| 用途 | アプローチ | メリット | デメリット |
|------|-----------|---------|-----------|
| 本番出力 | **Approach 2**: Tauri Window + Native Handle | 低レイテンシ、GPU直接レンダリング | プラットフォーム固有コード必要 |
| プレビュー | **AppSink → WebView**: フレームをフロントエンドに送信 | 柔軟なUI統合 | オーバーヘッドあり |
| NDI出力 | **ndisink**: GStreamer NDIプラグイン | 標準的、安定 | NDI SDK依存 |

### 9.3 アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│  GStreamer Pipeline                                             │
│                                                                 │
│  filesrc → decodebin → videoconvert → videobalance              │
│                                            │                    │
│                                            ↓                    │
│                              ┌─────────────────────────────┐    │
│                              │  Platform-Specific Sink     │    │
│                              │  (glimagesink / d3d11sink)  │    │
│                              │         ↑                   │    │
│                              │   window-handle property    │    │
│                              └─────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                                       ↑
                                       │ Native Handle
                                       │
┌─────────────────────────────────────────────────────────────────┐
│  Tauri Application                                              │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Output Window (WebviewWindow)                          │   │
│  │  - 特定モニターに配置                                    │   │
│  │  - フルスクリーン                                        │   │
│  │  - 装飾なし                                              │   │
│  │                                                          │   │
│  │  window.window_handle() → NSView* (macOS)               │   │
│  │                        → HWND (Windows)                  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 9.4 プラットフォーム別実装

#### 9.4.1 macOS (NSView + glimagesink)

```rust
// src-tauri/src/output/native_handle.rs

#[cfg(target_os = "macos")]
pub mod macos {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tauri::WebviewWindow;
    use gstreamer as gst;
    
    pub fn get_native_handle(window: &WebviewWindow) -> Option<*mut std::ffi::c_void> {
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::AppKit(appkit) => {
                // NSView pointer
                Some(appkit.ns_view.as_ptr())
            }
            _ => None,
        }
    }
    
    pub fn create_video_sink_with_handle(
        handle: *mut std::ffi::c_void,
    ) -> Result<gst::Element, gst::glib::Error> {
        let sink = gst::ElementFactory::make("glimagesink")
            .build()?;
        
        // NSViewをシンクに設定
        unsafe {
            sink.set_property("window-handle", handle as u64);
        }
        
        Ok(sink)
    }
}
```

#### 9.4.2 Windows (HWND + d3d11videosink)

```rust
#[cfg(target_os = "windows")]
pub mod windows {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tauri::WebviewWindow;
    use gstreamer as gst;
    
    pub fn get_native_handle(window: &WebviewWindow) -> Option<isize> {
        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(win32) => {
                // HWND
                Some(win32.hwnd.get() as isize)
            }
            _ => None,
        }
    }
    
    pub fn create_video_sink_with_handle(
        hwnd: isize,
    ) -> Result<gst::Element, gst::glib::Error> {
        let sink = gst::ElementFactory::make("d3d11videosink")
            .build()?;
        
        // HWNDをシンクに設定
        sink.set_property("window-handle", hwnd as u64);
        
        Ok(sink)
    }
}
```

### 9.5 出力ウィンドウ管理（改訂版）

```rust
// src-tauri/src/output/manager.rs (改訂版)

use std::collections::HashMap;
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};
use crate::types::*;

pub struct OutputManager {
    outputs: HashMap<String, OutputWindowState>,
}

pub struct OutputWindowState {
    pub id: String,
    pub window: Option<WebviewWindow>,
    pub native_handle: Option<NativeHandle>,
    pub output_type: OutputType,
    pub monitor_index: usize,
}

#[derive(Debug, Clone)]
pub enum NativeHandle {
    #[cfg(target_os = "macos")]
    NSView(*mut std::ffi::c_void),
    #[cfg(target_os = "windows")]
    HWND(isize),
}

impl OutputManager {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }
    
    /// 出力ウィンドウを作成し、ネイティブハンドルを取得
    pub fn create_output_window(
        &mut self,
        app: &AppHandle,
        config: &OutputTarget,
        monitor: &MonitorInfo,
    ) -> Result<NativeHandle, Box<dyn std::error::Error>> {
        // 指定モニターにフルスクリーンウィンドウを作成
        let window = WebviewWindowBuilder::new(
            app,
            &format!("output_{}", config.id),
            tauri::WebviewUrl::App("output.html".into()),
        )
        .title(&config.name)
        .position(monitor.x as f64, monitor.y as f64)
        .inner_size(monitor.width as f64, monitor.height as f64)
        .fullscreen(config.fullscreen.unwrap_or(true))
        .decorations(false)
        .always_on_top(true)
        .transparent(true)  // 透明背景（GStreamerがレンダリング）
        .build()?;
        
        // ネイティブハンドルを取得
        let native_handle = self.extract_native_handle(&window)?;
        
        self.outputs.insert(
            config.id.clone(),
            OutputWindowState {
                id: config.id.clone(),
                window: Some(window),
                native_handle: Some(native_handle.clone()),
                output_type: OutputType::Display,
                monitor_index: monitor.index,
            },
        );
        
        Ok(native_handle)
    }
    
    #[cfg(target_os = "macos")]
    fn extract_native_handle(
        &self,
        window: &WebviewWindow,
    ) -> Result<NativeHandle, Box<dyn std::error::Error>> {
        use super::native_handle::macos;
        
        let handle = macos::get_native_handle(window)
            .ok_or("Failed to get NSView handle")?;
        
        Ok(NativeHandle::NSView(handle))
    }
    
    #[cfg(target_os = "windows")]
    fn extract_native_handle(
        &self,
        window: &WebviewWindow,
    ) -> Result<NativeHandle, Box<dyn std::error::Error>> {
        use super::native_handle::windows;
        
        let handle = windows::get_native_handle(window)
            .ok_or("Failed to get HWND handle")?;
        
        Ok(NativeHandle::HWND(handle))
    }
    
    /// ウィンドウのネイティブハンドルを取得
    pub fn get_native_handle(&self, output_id: &str) -> Option<NativeHandle> {
        self.outputs.get(output_id)?.native_handle.clone()
    }
    
    pub fn close_output(&mut self, id: &str) {
        if let Some(output) = self.outputs.remove(id) {
            if let Some(window) = output.window {
                let _ = window.close();
            }
        }
    }
    
    pub fn close_all(&mut self) {
        for (_, output) in self.outputs.drain() {
            if let Some(window) = output.window {
                let _ = window.close();
            }
        }
    }
}
```

### 9.6 CuePlayer 統合（改訂版）

```rust
// src-tauri/src/pipeline/cue_player.rs (create_video_sink 改訂版)

impl CuePlayer {
    fn create_video_sink(
        output: &OutputTarget,
        native_handle: Option<&NativeHandle>,
    ) -> Result<gst::Element, gst::glib::Error> {
        match output.output_type {
            OutputType::Display => {
                if let Some(handle) = native_handle {
                    // Approach 2: ネイティブハンドルを使用
                    Self::create_platform_sink(handle)
                } else {
                    // フォールバック: autovideosink
                    gst::ElementFactory::make("autovideosink").build()
                }
            }
            OutputType::Ndi => {
                gst::ElementFactory::make("ndisink")
                    .property("ndi-name", output.ndi_name.as_ref().unwrap())
                    .build()
            }
            OutputType::Audio => {
                Err(gst::glib::Error::new(
                    gst::CoreError::Failed,
                    "Audio output cannot be used as video sink",
                ))
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    fn create_platform_sink(handle: &NativeHandle) -> Result<gst::Element, gst::glib::Error> {
        use super::native_handle::macos;
        
        if let NativeHandle::NSView(ptr) = handle {
            macos::create_video_sink_with_handle(*ptr)
        } else {
            Err(gst::glib::Error::new(
                gst::CoreError::Failed,
                "Invalid handle type for macOS",
            ))
        }
    }
    
    #[cfg(target_os = "windows")]
    fn create_platform_sink(handle: &NativeHandle) -> Result<gst::Element, gst::glib::Error> {
        use super::native_handle::windows;
        
        if let NativeHandle::HWND(hwnd) = handle {
            windows::create_video_sink_with_handle(*hwnd)
        } else {
            Err(gst::glib::Error::new(
                gst::CoreError::Failed,
                "Invalid handle type for Windows",
            ))
        }
    }
}
```

### 9.7 フロントエンド連携

```typescript
// src/stores/outputStore.ts (改訂版)

interface OutputStore {
  monitors: MonitorInfo[];
  isLoadingMonitors: boolean;
  openOutputs: Map<string, OpenOutputInfo>;
  
  fetchMonitors: () => Promise<void>;
  openOutput: (output: OutputTarget, monitor: MonitorInfo) => Promise<void>;
  closeOutput: (id: string) => Promise<void>;
  closeAllOutputs: () => Promise<void>;
  isOutputOpen: (id: string) => boolean;
  getOutputMonitor: (id: string) => MonitorInfo | undefined;
}

interface OpenOutputInfo {
  outputId: string;
  monitorIndex: number;
  isFullscreen: boolean;
}
```

### 9.8 実装フロー

```
1. ユーザーがモニターを選択してOpen Output
      ↓
2. フロントエンド: outputStore.openOutput(output, monitor)
      ↓
3. Rust: OutputManager.create_output_window()
   - Tauriウィンドウを指定モニターに作成
   - window.window_handle() でネイティブハンドル取得
   - ハンドルを保存
      ↓
4. Cue再生時: CuePlayer.load_cue()
   - OutputManager からハンドルを取得
   - create_video_sink() でプラットフォーム固有シンク作成
   - ハンドルをシンクに設定
      ↓
5. GStreamerが直接ウィンドウにレンダリング
   - フレームコピーなし
   - GPU直接描画
   - 低レイテンシ
```

### 9.9 依存関係

```toml
# Cargo.toml 追加

[dependencies]
raw-window-handle = "0.6"

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
objc2-app-kit = { version = "0.2", features = ["NSView", "NSWindow"] }

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging"] }
```

### 9.10 注意事項

1. **スレッドセーフティ**: ネイティブハンドルはスレッド間で安全に共有できるが、ウィンドウ操作はメインスレッドで行う必要がある

2. **ライフサイクル管理**: 
   - ウィンドウが閉じられたらシンクからハンドルを解除
   - パイプライン停止時にウィンドウを適切にクリーンアップ

3. **フォールバック戦略**:
   - ハンドル取得失敗時は `autovideosink` にフォールバック
   - プラットフォーム固有シンクが利用不可の場合も同様

4. **デバッグ**: 
   - `GST_DEBUG=glimagesink:5` で詳細ログ出力
   - ウィンドウハンドルの値をログ出力して確認

---

## 参考資料

- [GStreamer Documentation](https://gstreamer.freedesktop.org/documentation/)
- [gstreamer-rs](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs)
- [gstreamer-rs tutorials](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/tree/main/tutorials)
- [gst-plugin-ndi](https://github.com/teltek/gst-plugin-ndi)
- [NDI SDK](https://ndi.video/for-developers/ndi-sdk/)
- [Tauri v2 Docs](https://v2.tauri.app/)
- [ASIO SDK](https://www.steinberg.net/developers/)

---

*最終更新: 2025-12-25*  
*バージョン: v1.0*
