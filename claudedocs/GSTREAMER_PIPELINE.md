# GStreamer パイプライン設計リファレンス

TauriLivePlayer で使用する GStreamer パイプラインの設計ドキュメント。

---

## 目次

1. [設計方針](#設計方針)
2. [出力タイプ別パイプライン](#出力タイプ別パイプライン)
3. [エレメント一覧](#エレメント一覧)
4. [gst-launch-1.0 コマンド例](#gst-launch-10-コマンド例)
5. [Rust 実装パターン](#rust-実装パターン)
6. [トラブルシューティング](#トラブルシューティング)

---

## 設計方針

### 基本原則

1. **単一パイプライン**
   - 同期再生が必要なメディアは必ず同じパイプラインに配置
   - GStreamer のクロック機構が自動的に同期を保証

2. **NDI 出力は appsink + SDK**
   - `ndisink` は使用しない（ライブシンク問題を回避）
   - `appsink` でフレームを取得し、NDI SDK で直接送信

3. **出力タイプ別シンク選択**

| 出力タイプ | シンク | 備考 |
|-----------|--------|------|
| Display | `autovideosink` / platform specific | GStreamer管理 |
| Audio | `asiosink` / `osxaudiosink` / etc | GStreamer管理 |
| NDI | `appsink` → NDI SDK | Rust側で送信 |

---

## 出力タイプ別パイプライン

### Display + Audio（基本形）

```
Pipeline
│
├── [Audio Branch]
│   filesrc location=audio.wav
│       ↓
│   decodebin
│       ↓
│   audioconvert
│       ↓
│   audioresample
│       ↓
│   audiosink (asiosink / osxaudiosink / autoaudiosink)
│
└── [Video Branch]
    filesrc location=video.mp4
        ↓
    decodebin
        ↓
    videoconvert
        ↓
    videobalance brightness=0.0
        ↓
    autovideosink
```

### NDI 出力（appsink 方式）

```
Pipeline
│
├── [Audio Branch]
│   filesrc → decodebin → audioconvert → audiosink
│
└── [Video Branch - NDI]
    filesrc location=video.mp4
        ↓
    decodebin
        ↓
    videoconvert
        ↓
    videobalance brightness=0.0
        ↓
    capsfilter caps="video/x-raw,format=UYVY"
        ↓
    appsink sync=true emit-signals=true max-buffers=1 drop=true
        │
        ↓ [Rust callback: new-sample]
    ┌─────────────────────────────────┐
    │  NdiSender                      │
    │  - buffer.pts() で位置取得      │
    │  - NDI SDK で送信               │
    └─────────────────────────────────┘
```

### マルチ出力（Display + NDI + Audio）

```
Pipeline
│
├── [Audio]
│   filesrc(audio.wav) → decodebin → audioconvert → asiosink
│
├── [Video - Display]
│   filesrc(main.mp4) → decodebin → videoconvert → videobalance → autovideosink
│
└── [Video - NDI]
    filesrc(side.mp4) → decodebin → videoconvert → videobalance → capsfilter → appsink
                                                                                   ↓
                                                                            NdiSender
```

### Syphon / Spout 出力（同一PC内フレーム共有）

ローカルマシン内で他のVJソフト（TouchDesigner, Resolume, VDMX等）と連携する場合に使用。

#### Syphon (macOS)

```
Pipeline
│
└── [Video Branch - Syphon]
    filesrc location=video.mp4
        ↓
    decodebin
        ↓
    videoconvert
        ↓
    videobalance
        ↓
    glupload
        ↓
    glcolorconvert
        ↓
    appsink (GLメモリ)
        │
        ↓ [Rust callback]
    ┌─────────────────────────────────┐
    │  SyphonServer                   │
    │  - IOSurface 経由で共有         │
    │  - GPU直接、ゼロコピー          │
    └─────────────────────────────────┘
```

#### Spout (Windows)

```
Pipeline
│
└── [Video Branch - Spout]
    filesrc location=video.mp4
        ↓
    decodebin
        ↓
    d3d11upload
        ↓
    d3d11convert
        ↓
    appsink (D3D11メモリ)
        │
        ↓ [Rust callback]
    ┌─────────────────────────────────┐
    │  SpoutSender                    │
    │  - DirectX テクスチャ共有       │
    │  - GPU直接、ゼロコピー          │
    └─────────────────────────────────┘
```

#### NDI vs Syphon/Spout

| | NDI | Syphon/Spout |
|---|-----|--------------|
| 範囲 | ネットワーク越し | 同一PC内のみ |
| 転送 | CPU経由 | GPU直接（ゼロコピー） |
| レイテンシ | 数フレーム | ほぼ0 |
| 用途 | 別PC間、配信 | 同一PC内VJ連携 |

---

### NDI 受信

```
Pipeline
│
└── ndisrc ndi-name="SOURCE_NAME"
        ↓
    ndisrcdemux name=demux
        │
        ├── demux.video
        │       ↓
        │   videoconvert
        │       ↓
        │   autovideosink
        │
        └── demux.audio
                ↓
            audioconvert
                ↓
            autoaudiosink
```

### マルチスクリーン合成（compositor）

```
Pipeline
│
├── [Source 1]
│   filesrc(a.mp4) → decodebin → videoconvert ─┐
│                                               ↓
├── [Source 2]                              compositor
│   filesrc(b.mp4) → decodebin → videoconvert ─┤  sink_0::xpos=0    sink_0::ypos=0
│                                               │  sink_1::xpos=1920 sink_1::ypos=0
├── [Source 3]                                  │  sink_2::xpos=0    sink_2::ypos=1080
│   filesrc(c.mp4) → decodebin → videoconvert ─┤  sink_3::xpos=1920 sink_3::ypos=1080
│                                               │
└── [Source 4]                                  ↓
    filesrc(d.mp4) → decodebin → videoconvert ─┘
                                                ↓
                                         capsfilter (3840x2160, UYVY)
                                                ↓
                                         appsink → NdiSender
```

---

## エレメント一覧

### ソース

| エレメント | 用途 | プロパティ |
|-----------|------|-----------|
| `filesrc` | ファイル読み込み | `location` |
| `ndisrc` | NDI受信 | `ndi-name`, `timeout` |
| `videotestsrc` | テストパターン | `pattern`, `is-live` |

### デコード

| エレメント | 用途 | 備考 |
|-----------|------|------|
| `decodebin` | 自動デコード | 動的パッド |
| `uridecodebin` | URI自動デコード | `uri` プロパティ |

### 変換

| エレメント | 用途 | プロパティ |
|-----------|------|-----------|
| `videoconvert` | ピクセルフォーマット変換 | - |
| `videoscale` | 解像度変換 | - |
| `videobalance` | 明るさ/コントラスト | `brightness` (-1.0〜1.0), `contrast`, `saturation` |
| `audioconvert` | オーディオフォーマット変換 | - |
| `audioresample` | サンプルレート変換 | - |
| `volume` | 音量調整 | `volume` (0.0〜10.0), `mute` |

### GPU アップロード / 変換（Syphon/Spout用）

| エレメント | OS | 用途 | 備考 |
|-----------|-----|------|------|
| `glupload` | 全OS | システムメモリ → GL テクスチャ | OpenGL |
| `gldownload` | 全OS | GL テクスチャ → システムメモリ | OpenGL |
| `glcolorconvert` | 全OS | GL上でフォーマット変換 | OpenGL |
| `d3d11upload` | Windows | システムメモリ → D3D11 | DirectX 11 |
| `d3d11download` | Windows | D3D11 → システムメモリ | DirectX 11 |
| `d3d11convert` | Windows | D3D11上でフォーマット変換 | DirectX 11 |

### Caps フィルタ

| エレメント | 用途 | 例 |
|-----------|------|-----|
| `capsfilter` | フォーマット指定 | `caps="video/x-raw,format=UYVY,width=1920,height=1080"` |

### 合成

| エレメント | 用途 | プロパティ |
|-----------|------|-----------|
| `compositor` | 映像合成 | `sink_N::xpos`, `sink_N::ypos`, `sink_N::zorder` |
| `audiomixer` | 音声ミックス | - |

### シンク

| エレメント | 用途 | プロパティ |
|-----------|------|-----------|
| `autovideosink` | 自動ビデオ出力 | - |
| `autoaudiosink` | 自動オーディオ出力 | - |
| `appsink` | アプリへ出力 | `sync`, `emit-signals`, `max-buffers`, `drop`, `caps` |
| `fakesink` | 破棄 | `sync`, `async` |

#### プラットフォーム別オーディオシンク

| OS | エレメント | レイテンシ |
|----|-----------|-----------|
| Windows | `asiosink` | ~3ms |
| Windows | `wasapisink` | ~10ms |
| macOS | `osxaudiosink` | ~10ms |
| Linux | `jackaudiosink` | ~5ms |
| Linux | `alsasink` | ~20ms |

### キュー

| エレメント | 用途 | プロパティ |
|-----------|------|-----------|
| `queue` | バッファリング | `max-size-time`, `max-size-buffers`, `max-size-bytes`, `leaky` |
| `queue2` | ファイルバッファ対応 | `use-buffering` |
| `tee` | ストリーム分岐 | - |

---

## gst-launch-1.0 コマンド例

### 基本再生

```bash
# 映像+音声再生
gst-launch-1.0 \
  filesrc location=test.mp4 ! \
  decodebin name=d \
  d. ! videoconvert ! autovideosink \
  d. ! audioconvert ! autoaudiosink
```

### 明るさ調整

```bash
# brightness: -1.0 (暗) 〜 0.0 (標準) 〜 1.0 (明)
gst-launch-1.0 \
  filesrc location=test.mp4 ! \
  decodebin ! \
  videoconvert ! \
  videobalance brightness=-0.2 ! \
  autovideosink
```

### UYVY フォーマット確認

```bash
gst-launch-1.0 \
  filesrc location=test.mp4 ! \
  decodebin ! \
  videoconvert ! \
  "video/x-raw,format=UYVY" ! \
  fakesink
```

### NDI 受信

```bash
gst-launch-1.0 \
  ndisrc ndi-name="MY_SOURCE" ! \
  ndisrcdemux name=d \
  d.video ! videoconvert ! autovideosink \
  d.audio ! audioconvert ! autoaudiosink
```

### 複数ファイル同時再生（同期確認）

```bash
gst-launch-1.0 \
  filesrc location=audio.wav ! decodebin ! audioconvert ! autoaudiosink \
  filesrc location=video.mp4 ! decodebin ! videoconvert ! autovideosink
```

### マルチスクリーン合成

```bash
gst-launch-1.0 \
  compositor name=c \
    sink_0::xpos=0 sink_0::ypos=0 \
    sink_1::xpos=960 sink_1::ypos=0 ! \
  videoconvert ! autovideosink \
  videotestsrc pattern=ball ! video/x-raw,width=960,height=540 ! c.sink_0 \
  videotestsrc pattern=snow ! video/x-raw,width=960,height=540 ! c.sink_1
```

### ASIO 出力（Windows）

```bash
# デバイス一覧
gst-device-monitor-1.0 Audio/Sink

# ASIO 出力
gst-launch-1.0 \
  filesrc location=audio.wav ! decodebin ! audioconvert ! \
  asiosink device-clsid="{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"
```

---

## Rust 実装パターン

### パイプライン作成

```rust
use gstreamer as gst;
use gstreamer::prelude::*;

fn create_pipeline() -> Result<gst::Pipeline, gst::glib::Error> {
    gst::init()?;
    let pipeline = gst::Pipeline::new();
    Ok(pipeline)
}
```

### エレメント作成とリンク

```rust
// エレメント作成
let src = gst::ElementFactory::make("filesrc")
    .property("location", "/path/to/video.mp4")
    .build()?;

let decode = gst::ElementFactory::make("decodebin").build()?;
let convert = gst::ElementFactory::make("videoconvert").build()?;
let sink = gst::ElementFactory::make("autovideosink").build()?;

// パイプラインに追加
pipeline.add_many([&src, &decode, &convert, &sink])?;

// 静的リンク
src.link(&decode)?;
convert.link(&sink)?;

// decodebin の動的パッド処理
decode.connect_pad_added(move |_, src_pad| {
    let sink_pad = convert.static_pad("sink").unwrap();
    if !sink_pad.is_linked() {
        src_pad.link(&sink_pad).unwrap();
    }
});
```

### appsink コールバック

```rust
use gstreamer_app as gst_app;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

let last_pts = Arc::new(AtomicU64::new(0));
let last_pts_clone = last_pts.clone();

let appsink = gst_app::AppSink::builder()
    .sync(true)
    .emit_signals(true)
    .max_buffers(1)
    .drop(true)
    .caps(
        &gst::Caps::builder("video/x-raw")
            .field("format", "UYVY")
            .build(),
    )
    .build();

appsink.set_callbacks(
    gst_app::AppSinkCallbacks::builder()
        .new_sample(move |sink| {
            let sample = sink.pull_sample().map_err(|_| gst::FlowError::Error)?;
            let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
            
            // PTS を記録
            if let Some(pts) = buffer.pts() {
                last_pts_clone.store(pts.nseconds(), Ordering::Relaxed);
            }
            
            // バッファデータを取得
            let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
            let data: &[u8] = map.as_slice();
            
            // NDI SDK で送信（省略）
            
            Ok(gst::FlowSuccess::Ok)
        })
        .build(),
);
```

### videobalance で明るさ調整

```rust
let balance = gst::ElementFactory::make("videobalance")
    .property("brightness", 0.0f64)  // -1.0 〜 1.0
    .build()?;

// 実行時に変更
balance.set_property("brightness", -0.2f64);
```

### 再生制御

```rust
// 再生
pipeline.set_state(gst::State::Playing)?;

// 一時停止
pipeline.set_state(gst::State::Paused)?;

// 停止
pipeline.set_state(gst::State::Null)?;

// シーク
let position = gst::ClockTime::from_seconds(30);
pipeline.seek_simple(
    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
    position,
)?;
```

### 位置・長さ取得

```rust
// 現在位置（秒）
let position: Option<f64> = pipeline
    .query_position::<gst::ClockTime>()
    .map(|p| p.seconds_f64());

// 長さ（秒）
let duration: Option<f64> = pipeline
    .query_duration::<gst::ClockTime>()
    .map(|d| d.seconds_f64());
```

### バスメッセージ処理

```rust
let bus = pipeline.bus().unwrap();

// 非同期でメッセージ処理
std::thread::spawn(move || {
    for msg in bus.iter() {
        match msg.view() {
            gst::MessageView::Eos(_) => {
                println!("End of stream");
                break;
            }
            gst::MessageView::Error(err) => {
                eprintln!("Error: {} ({:?})", err.error(), err.debug());
                break;
            }
            gst::MessageView::StateChanged(state) => {
                if state.src().map(|s| s == pipeline).unwrap_or(false) {
                    println!("State: {:?} -> {:?}", state.old(), state.current());
                }
            }
            _ => {}
        }
    }
});
```

---

## トラブルシューティング

### ndisink ライブシンク問題

**症状**: `pipeline.query_position()` が不正確な値を返す（例: 13秒オフセット）

**原因**: `ndisink` はライブシンクとして動作し、非ライブソース（filesrc）と混在するとクロック管理が複雑化する

**解決策**: `appsink` + NDI SDK 直接呼び出しを使用

```rust
// NG: ndisink
let sink = gst::ElementFactory::make("ndisink").build()?;

// OK: appsink + NdiSender
let appsink = gst_app::AppSink::builder()
    .sync(true)  // 同期を有効化
    // ...
    .build();
// コールバック内で NDI SDK を呼び出す
```

### decodebin の動的パッドでリンクできない

**症状**: `pad_added` コールバック内でリンクエラー

**原因**: シンクパッドが既にリンク済み、またはエレメントがパイプラインに追加されていない

**解決策**:

```rust
decode.connect_pad_added(move |_, src_pad| {
    let caps = match src_pad.current_caps() {
        Some(c) => c,
        None => return,  // caps がまだない場合はスキップ
    };
    
    let sink_pad = convert.static_pad("sink").unwrap();
    if sink_pad.is_linked() {
        return;  // 既にリンク済み
    }
    
    src_pad.link(&sink_pad).unwrap();
});
```

### 同期再生でズレが発生

**症状**: 複数ファイルの再生タイミングがズレる

**原因**: 別パイプラインで再生している、または queue 設定が不適切

**解決策**:

1. 同じパイプラインに配置
2. queue でバッファリング

```rust
let queue = gst::ElementFactory::make("queue")
    .property("max-size-time", 500_000_000u64)  // 500ms
    .property("max-size-buffers", 0u32)
    .property("max-size-bytes", 0u32)
    .build()?;
```

### ASIO デバイスが見つからない

**症状**: Windows で `asiosink` がエラー

**原因**: ASIO SDK がビルドに含まれていない

**解決策**:

1. Steinberg から ASIO SDK をダウンロード
2. GStreamer を自前ビルド、または vcpkg 使用
3. フォールバックとして `wasapisink` を使用

```rust
#[cfg(target_os = "windows")]
fn create_audio_sink() -> gst::Element {
    if gst::ElementFactory::find("asiosink").is_some() {
        gst::ElementFactory::make("asiosink").build().unwrap()
    } else {
        gst::ElementFactory::make("wasapisink")
            .property("low-latency", true)
            .build()
            .unwrap()
    }
}
```

---

## 参考リンク

- [GStreamer Documentation](https://gstreamer.freedesktop.org/documentation/)
- [gstreamer-rs Tutorials](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/tree/main/tutorials)
- [GStreamer Plugin Reference](https://gstreamer.freedesktop.org/documentation/plugins_doc.html)
- [NDI SDK Documentation](https://ndi.video/for-developers/ndi-sdk/)

---

*最終更新: 2025-12-28*
