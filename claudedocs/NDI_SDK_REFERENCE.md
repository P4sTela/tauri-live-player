# NDI SDK リファレンス

TauriLivePlayer における NDI 送受信の実装ガイド。

---

## 目次

1. [概要](#概要)
2. [アーキテクチャ](#アーキテクチャ)
3. [grafton-ndi クレート](#grafton-ndi-クレート)
4. [送信実装](#送信実装)
5. [受信実装](#受信実装)
6. [GStreamer 統合](#gstreamer-統合)
7. [パフォーマンス最適化](#パフォーマンス最適化)
8. [トラブルシューティング](#トラブルシューティング)

---

## 概要

### なぜ appsink + NDI SDK か

GStreamer の `ndisink` を使わず、`appsink` + NDI SDK 直接呼び出しを採用する理由：

| 問題 | ndisink | appsink + SDK |
|------|---------|---------------|
| ライブ/非ライブ混在 | position が不正確（13秒オフセット） | 問題なし |
| PTS 取得 | query_position() が不安定 | バッファから直接取得 |
| NDI|HX 対応 | 不可（Advanced SDK 必要） | 可能 |
| ゼロコピー送信 | 不可 | 可能 |

### 使用クレート

```toml
[dependencies]
grafton-ndi = "0.9"

# async 機能を使う場合
# grafton-ndi = { version = "0.9", features = ["tokio"] }
```

**grafton-ndi を選んだ理由:**
- NDI 6 SDK 対応（最新）
- ゼロコピー送受信サポート
- async 送信対応
- 活発なメンテナンス（2025年10月 v0.9）
- クロスプラットフォーム

---

## アーキテクチャ

### 全体構成

```
┌─────────────────────────────────────────────────────────────────┐
│  GStreamer Pipeline                                             │
│                                                                 │
│  filesrc → decodebin → videoconvert → videobalance             │
│                                            ↓                    │
│                                     capsfilter (UYVY)           │
│                                            ↓                    │
│                                     appsink (sync=true)         │
│                                            │                    │
└────────────────────────────────────────────┼────────────────────┘
                                             ↓
┌────────────────────────────────────────────┴────────────────────┐
│  Rust NdiSender                                                 │
│                                                                 │
│  ┌─────────────────┐                                           │
│  │ new-sample      │                                           │
│  │ callback        │                                           │
│  │                 │                                           │
│  │ 1. PTS 記録     │                                           │
│  │ 2. バッファ取得  │                                           │
│  │ 3. NDI 送信     │                                           │
│  └────────┬────────┘                                           │
│           ↓                                                     │
│  ┌─────────────────┐    ┌─────────────────┐                    │
│  │ Double Buffer   │ →  │ grafton-ndi     │ → NDI Network      │
│  │ [A] ←→ [B]      │    │ Sender          │                    │
│  └─────────────────┘    └─────────────────┘                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### データフロー

```
1. GStreamer がファイルをデコード
2. videobalance で明るさ調整
3. UYVY フォーマットに変換
4. appsink の new-sample コールバック発火
5. コールバック内で:
   a. バッファの PTS を記録（position 取得用）
   b. フレームデータを取得
   c. grafton-ndi で NDI 送信
```

---

## grafton-ndi クレート

### 基本構造

```rust
use grafton_ndi::{
    NDI,                    // ライブラリ初期化
    Sender, SenderOptions,  // 送信
    Receiver, ReceiverOptions, ReceiverBandwidth,  // 受信
    Finder, FinderOptions,  // ソース発見
    VideoFrame,             // 所有権ありフレーム
    BorrowedVideoFrame,     // ゼロコピーフレーム
    PixelFormat,            // UYVY, BGRA, etc.
    Error,                  // エラー型
};
```

### 初期化

```rust
use grafton_ndi::NDI;
use std::sync::Arc;

// NDI ライブラリ初期化（アプリケーション起動時に1回）
let ndi = Arc::new(NDI::new()?);

// Arc でラップして複数の Sender/Receiver で共有
```

### ピクセルフォーマット

| フォーマット | 説明 | 用途 |
|-------------|------|------|
| `PixelFormat::UYVY` | 4:2:2 YUV | **推奨**（NDI ネイティブ） |
| `PixelFormat::BGRA` | 8bit BGRA | アルファチャンネル必要時 |
| `PixelFormat::BGRX` | 8bit BGR (Xは無視) | アルファ不要時 |
| `PixelFormat::RGBA` | 8bit RGBA | アルファチャンネル必要時 |
| `PixelFormat::RGBX` | 8bit RGB (Xは無視) | アルファ不要時 |

**UYVY を推奨する理由:**
- NDI のネイティブフォーマット
- 帯域幅が BGRA の半分（1ピクセル2バイト vs 4バイト）
- NDI SDK 内部での変換が不要

---

## 送信実装

### レベル1: 基本実装（コピーあり）

最もシンプルな実装。まずはこれで動作確認。

```rust
use grafton_ndi::{NDI, Sender, SenderOptions, VideoFrame, PixelFormat};
use std::sync::Arc;

pub struct NdiSender {
    _ndi: Arc<NDI>,
    sender: Sender,
}

impl NdiSender {
    pub fn new(name: &str) -> Result<Self, grafton_ndi::Error> {
        let ndi = Arc::new(NDI::new()?);
        
        let options = SenderOptions::builder(name)
            .clock_video(false)  // GStreamer が同期管理
            .clock_audio(false)
            .build();
        
        let sender = Sender::new(&ndi, &options)?;
        
        Ok(Self {
            _ndi: ndi,
            sender,
        })
    }
    
    /// フレーム送信（コピーあり、同期）
    pub fn send_frame(
        &self,
        data: &[u8],
        width: i32,
        height: i32,
        frame_rate_n: i32,
        frame_rate_d: i32,
    ) {
        let frame = VideoFrame::builder()
            .resolution(width, height)
            .pixel_format(PixelFormat::UYVY)
            .frame_rate(frame_rate_n, frame_rate_d)
            .data(data.to_vec())  // ここでコピー発生
            .build();
        
        self.sender.send_video(&frame);  // ブロッキング送信
    }
}
```

**特徴:**
- シンプルで理解しやすい
- `data.to_vec()` でコピーが発生
- `send_video()` は送信完了までブロック
- 1080p60 で約 250MB/秒 の memcpy

---

### レベル2: ダブルバッファ + 非同期送信

ゼロコピー送信でパフォーマンス向上。

```rust
use grafton_ndi::{
    NDI, Sender, SenderOptions, BorrowedVideoFrame, PixelFormat
};
use std::sync::Arc;

pub struct NdiSenderOptimized {
    _ndi: Arc<NDI>,
    sender: Sender,
    
    // ダブルバッファ
    buffers: [Vec<u8>; 2],
    current_buffer: usize,
    
    // フレーム情報
    width: i32,
    height: i32,
    frame_rate_n: i32,
    frame_rate_d: i32,
}

impl NdiSenderOptimized {
    pub fn new(
        name: &str,
        width: i32,
        height: i32,
        frame_rate_n: i32,
        frame_rate_d: i32,
    ) -> Result<Self, grafton_ndi::Error> {
        let ndi = Arc::new(NDI::new()?);
        
        let options = SenderOptions::builder(name)
            .clock_video(false)
            .build();
        
        let sender = Sender::new(&ndi, &options)?;
        
        // UYVY: 1ピクセル = 2バイト
        let buffer_size = (width * height * 2) as usize;
        
        Ok(Self {
            _ndi: ndi,
            sender,
            buffers: [
                vec![0u8; buffer_size],
                vec![0u8; buffer_size],
            ],
            current_buffer: 0,
            width,
            height,
            frame_rate_n,
            frame_rate_d,
        })
    }
    
    /// フレーム送信（ゼロコピー、非同期）
    /// 
    /// # 動作
    /// 1. データを現在のバッファにコピー（1回のみ）
    /// 2. BorrowedVideoFrame でポインタだけ渡す
    /// 3. send_video_async() で即座にリターン
    /// 4. バッファを切り替え（次フレーム用）
    /// 
    /// # 重要
    /// - 前のフレームの送信完了を待ってからバッファを再利用
    /// - send_video_async() は前フレーム完了まで内部で待機
    pub fn send_frame(&mut self, data: &[u8]) {
        // 1. 現在のバッファにコピー
        let buf = &mut self.buffers[self.current_buffer];
        buf[..data.len()].copy_from_slice(data);
        
        // 2. ゼロコピーでフレーム作成（ポインタだけ）
        let frame = BorrowedVideoFrame::builder()
            .resolution(self.width, self.height)
            .pixel_format(PixelFormat::UYVY)
            .frame_rate(self.frame_rate_n, self.frame_rate_d)
            .line_stride(self.width * 2)  // UYVY: 2 bytes per pixel
            .data(&self.buffers[self.current_buffer])
            .build();
        
        // 3. 非同期送信（前フレーム完了まで待機後、即リターン）
        self.sender.send_video_async(&frame);
        
        // 4. バッファ切り替え
        self.current_buffer = 1 - self.current_buffer;
    }
}
```

**ダブルバッファの仕組み:**

```
Time →

Frame 1:  [Buffer A に書き込み] → [NDI送信開始] ─────────────────→ [送信完了]
Frame 2:                         [Buffer B に書き込み] → [NDI送信開始] ──→
Frame 3:  [Buffer A 再利用OK] ─────────────────────────→ [Buffer A に書き込み]
```

- Buffer A を送信中に Buffer B に次のフレームを書き込み
- `send_video_async()` は前フレーム完了を待ってから新フレームを送信開始
- 2つのバッファを交互に使うことで、常に安全

---

### レベル3: 完了コールバック（Advanced SDK）

最高のパフォーマンス。フレームプール管理が必要。

```rust
use grafton_ndi::{NDI, Sender, SenderOptions, BorrowedVideoFrame, PixelFormat};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// フレームバッファプール
struct FramePool {
    available: VecDeque<Vec<u8>>,
    in_flight: Vec<Vec<u8>>,
}

pub struct NdiSenderAdvanced {
    _ndi: Arc<NDI>,
    sender: Sender,
    pool: Arc<Mutex<FramePool>>,
    buffer_size: usize,
    width: i32,
    height: i32,
}

impl NdiSenderAdvanced {
    pub fn new(
        name: &str,
        width: i32,
        height: i32,
    ) -> Result<Self, grafton_ndi::Error> {
        let ndi = Arc::new(NDI::new()?);
        
        let options = SenderOptions::builder(name)
            .clock_video(false)
            .build();
        
        let mut sender = Sender::new(&ndi, &options)?;
        
        let buffer_size = (width * height * 2) as usize;
        
        // 初期バッファプール（3-4フレーム分）
        let pool = Arc::new(Mutex::new(FramePool {
            available: (0..4).map(|_| vec![0u8; buffer_size]).collect(),
            in_flight: Vec::new(),
        }));
        
        // 完了コールバック設定
        let pool_clone = pool.clone();
        sender.set_async_completion_handler(move |_frame_ptr| {
            // 送信完了したバッファをプールに戻す
            let mut pool = pool_clone.lock().unwrap();
            if let Some(buf) = pool.in_flight.pop() {
                pool.available.push_back(buf);
            }
        });
        
        Ok(Self {
            _ndi: ndi,
            sender,
            pool,
            buffer_size,
            width,
            height,
        })
    }
    
    /// フレーム送信（完全非同期）
    pub fn send_frame(&mut self, data: &[u8]) -> Result<(), &'static str> {
        let mut pool = self.pool.lock().unwrap();
        
        // 利用可能なバッファを取得
        let mut buf = pool.available.pop_front()
            .ok_or("No available buffer")?;
        
        // データをコピー
        buf[..data.len()].copy_from_slice(data);
        
        // フレーム作成
        let frame = BorrowedVideoFrame::builder()
            .resolution(self.width, self.height)
            .pixel_format(PixelFormat::UYVY)
            .data(&buf)
            .build();
        
        // 送信中リストに追加
        pool.in_flight.push(buf);
        
        drop(pool);  // ロック解放
        
        // 非同期送信（即リターン）
        self.sender.send_video_async(&frame);
        
        Ok(())
    }
}
```

**完了コールバックの仕組み:**

```
              ┌─────────────────────────────────────┐
              │         Frame Pool                  │
              │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐  │
              │  │Buf 0│ │Buf 1│ │Buf 2│ │Buf 3│  │
              │  └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘  │
              └─────┼───────┼───────┼───────┼─────┘
                    ↓       ↓       ↓       ↓
             [available]         [in_flight]
                    
send_frame():
  1. available.pop() → バッファ取得
  2. データコピー
  3. in_flight.push() → 送信中に移動
  4. send_video_async() → 即リターン
  
completion_callback():
  1. in_flight.pop() → 送信完了
  2. available.push() → 再利用可能に
```

---

## 受信実装

### 基本受信

```rust
use grafton_ndi::{
    NDI, Finder, FinderOptions, Receiver, ReceiverOptions, ReceiverBandwidth
};
use std::time::Duration;

pub struct NdiReceiver {
    _ndi: Arc<NDI>,
    receiver: Receiver,
}

impl NdiReceiver {
    pub fn new(ndi: Arc<NDI>, source_name: &str) -> Result<Self, grafton_ndi::Error> {
        // ソース検索
        let finder_options = FinderOptions::builder()
            .show_local_sources(true)
            .build();
        let finder = Finder::new(&ndi, &finder_options)?;
        
        // ソースが見つかるまで待機
        let sources = finder.find_sources(Duration::from_secs(5))?;
        
        let source = sources.iter()
            .find(|s| s.name().contains(source_name))
            .ok_or(grafton_ndi::Error::NotFound)?;
        
        // 受信設定
        let options = ReceiverOptions::builder(source.clone())
            .color(grafton_ndi::ReceiverColorFormat::UYVY_BGRA)
            .bandwidth(ReceiverBandwidth::Highest)
            .build();
        
        let receiver = Receiver::new(&ndi, &options)?;
        
        Ok(Self {
            _ndi: ndi,
            receiver,
        })
    }
    
    /// フレーム受信（所有権あり）
    pub fn receive_frame(&self) -> Result<grafton_ndi::VideoFrame, grafton_ndi::Error> {
        self.receiver.capture_video(Duration::from_secs(5))
    }
    
    /// フレーム受信（ゼロコピー）
    pub fn receive_frame_ref(&self) -> Result<grafton_ndi::VideoFrameRef<'_>, grafton_ndi::Error> {
        self.receiver.capture_video_ref(Duration::from_secs(5))
    }
}
```

### ゼロコピー受信

```rust
// ゼロコピー受信：NDI SDK のバッファを直接参照
let frame_ref = receiver.capture_video_ref(Duration::from_secs(1))?;

// データは NDI SDK のメモリを直接参照
let data: &[u8] = frame_ref.data();
let width = frame_ref.width();
let height = frame_ref.height();
let pts = frame_ref.timestamp();

// frame_ref がドロップされるまでデータは有効
// 注意: frame_ref のライフタイムは receiver に束縛される
process_frame(data, width, height);

// ここで frame_ref がドロップ → NDI SDK にバッファ返却
```

---

## GStreamer 統合

### 完全な実装例

```rust
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use grafton_ndi::{NDI, Sender, SenderOptions, BorrowedVideoFrame, PixelFormat};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

/// GStreamer + NDI 統合送信器
pub struct GstNdiSender {
    _ndi: Arc<NDI>,
    sender: Arc<Mutex<Sender>>,
    
    // ダブルバッファ
    buffers: Arc<Mutex<[Vec<u8>; 2]>>,
    current_buffer: Arc<Mutex<usize>>,
    
    // 最後のPTS（position取得用）
    last_pts: Arc<AtomicU64>,
    
    // フレーム情報
    width: i32,
    height: i32,
}

impl GstNdiSender {
    pub fn new(
        name: &str,
        width: i32,
        height: i32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ndi = Arc::new(NDI::new()?);
        
        let options = SenderOptions::builder(name)
            .clock_video(false)
            .build();
        
        let sender = Sender::new(&ndi, &options)?;
        
        let buffer_size = (width * height * 2) as usize;
        
        Ok(Self {
            _ndi: ndi,
            sender: Arc::new(Mutex::new(sender)),
            buffers: Arc::new(Mutex::new([
                vec![0u8; buffer_size],
                vec![0u8; buffer_size],
            ])),
            current_buffer: Arc::new(Mutex::new(0)),
            last_pts: Arc::new(AtomicU64::new(0)),
            width,
            height,
        })
    }
    
    /// appsink を作成して返す
    pub fn create_appsink(&self) -> gst::Element {
        let appsink = gst_app::AppSink::builder()
            .sync(true)           // GStreamer 同期有効
            .emit_signals(true)
            .max_buffers(1)
            .drop(true)           // 遅れたフレームはドロップ
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "UYVY")
                    .field("width", self.width)
                    .field("height", self.height)
                    .build(),
            )
            .build();
        
        // コールバック用にクローン
        let sender = self.sender.clone();
        let buffers = self.buffers.clone();
        let current_buffer = self.current_buffer.clone();
        let last_pts = self.last_pts.clone();
        let width = self.width;
        let height = self.height;
        
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    Self::handle_sample(
                        sink,
                        &sender,
                        &buffers,
                        &current_buffer,
                        &last_pts,
                        width,
                        height,
                    )
                })
                .build(),
        );
        
        appsink.upcast()
    }
    
    fn handle_sample(
        sink: &gst_app::AppSink,
        sender: &Arc<Mutex<Sender>>,
        buffers: &Arc<Mutex<[Vec<u8>; 2]>>,
        current_buffer: &Arc<Mutex<usize>>,
        last_pts: &Arc<AtomicU64>,
        width: i32,
        height: i32,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        // サンプル取得
        let sample = sink.pull_sample().map_err(|_| gst::FlowError::Error)?;
        let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
        
        // PTS 記録
        if let Some(pts) = buffer.pts() {
            last_pts.store(pts.nseconds(), Ordering::Relaxed);
        }
        
        // バッファデータ取得
        let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
        let data = map.as_slice();
        
        // 現在のバッファにコピー
        let mut bufs = buffers.lock().unwrap();
        let mut curr = current_buffer.lock().unwrap();
        bufs[*curr][..data.len()].copy_from_slice(data);
        
        // ゼロコピーでフレーム作成
        let frame = BorrowedVideoFrame::builder()
            .resolution(width, height)
            .pixel_format(PixelFormat::UYVY)
            .line_stride(width * 2)
            .data(&bufs[*curr])
            .build();
        
        // 非同期送信
        sender.lock().unwrap().send_video_async(&frame);
        
        // バッファ切り替え
        *curr = 1 - *curr;
        
        Ok(gst::FlowSuccess::Ok)
    }
    
    /// 最後のフレームの PTS（秒）を取得
    pub fn last_position(&self) -> f64 {
        let pts_ns = self.last_pts.load(Ordering::Relaxed);
        pts_ns as f64 / 1_000_000_000.0
    }
}
```

### パイプライン構築

```rust
fn build_ndi_pipeline(
    video_path: &str,
    ndi_name: &str,
) -> Result<(gst::Pipeline, GstNdiSender), Box<dyn std::error::Error>> {
    gst::init()?;
    
    let pipeline = gst::Pipeline::new();
    
    // エレメント作成
    let src = gst::ElementFactory::make("filesrc")
        .property("location", video_path)
        .build()?;
    
    let decode = gst::ElementFactory::make("decodebin").build()?;
    let convert = gst::ElementFactory::make("videoconvert").build()?;
    let balance = gst::ElementFactory::make("videobalance").build()?;
    
    // NDI Sender 作成
    let ndi_sender = GstNdiSender::new(ndi_name, 1920, 1080)?;
    let appsink = ndi_sender.create_appsink();
    
    // パイプラインに追加
    pipeline.add_many([&src, &decode, &convert, &balance, &appsink])?;
    
    // 静的リンク
    src.link(&decode)?;
    gst::Element::link_many([&convert, &balance, &appsink])?;
    
    // decodebin の動的パッド
    let convert_weak = convert.downgrade();
    decode.connect_pad_added(move |_, src_pad| {
        let convert = match convert_weak.upgrade() {
            Some(c) => c,
            None => return,
        };
        
        let caps = match src_pad.current_caps() {
            Some(c) => c,
            None => return,
        };
        
        let structure = caps.structure(0).unwrap();
        if structure.name().starts_with("video/") {
            let sink_pad = convert.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                src_pad.link(&sink_pad).unwrap();
            }
        }
    });
    
    Ok((pipeline, ndi_sender))
}
```

---

## パフォーマンス最適化

### 帯域幅の目安

| 解像度 | フォーマット | フレームサイズ | 60fps での帯域 |
|-------|-------------|---------------|---------------|
| 1080p | UYVY | 4.1 MB | 247 MB/秒 |
| 1080p | BGRA | 8.3 MB | 498 MB/秒 |
| 4K | UYVY | 16.6 MB | 995 MB/秒 |
| 4K | BGRA | 33.2 MB | 1.99 GB/秒 |

**→ UYVY を使う理由**

### 最適化チェックリスト

| 項目 | 推奨設定 | 理由 |
|------|---------|------|
| ピクセルフォーマット | UYVY | NDI ネイティブ、帯域半減 |
| バッファ方式 | ダブルバッファ | ゼロコピー送信可能 |
| 送信方式 | send_video_async | ブロッキングなし |
| clock_video | false | GStreamer が同期管理 |
| appsink.sync | true | GStreamer 同期有効 |
| appsink.drop | true | 遅延フレームはドロップ |
| appsink.max-buffers | 1 | メモリ節約 |

### CPU 使用率を下げるには

```rust
// 1. UYVY フォーマットを使う（NDI SDK 内部変換なし）
.pixel_format(PixelFormat::UYVY)

// 2. 送信側でクロックを使わない（GStreamer に任せる）
SenderOptions::builder(name)
    .clock_video(false)
    .clock_audio(false)

// 3. 解像度が変わらないなら事前にバッファ確保
let buffer_size = width * height * 2;  // UYVY
let buffers = [vec![0u8; buffer_size], vec![0u8; buffer_size]];
```

---

## トラブルシューティング

### NDI SDK が見つからない

**エラー:**
```
error: failed to run custom build command for `grafton-ndi`
```

**解決策:**

Windows:
```powershell
# NDI SDK をインストール後
$env:NDI_SDK_DIR = "C:\Program Files\NDI\NDI 6 SDK"
# または PATH に追加
$env:PATH += ";C:\Program Files\NDI\NDI 6 SDK\Bin\x64"
```

macOS:
```bash
# NDI SDK をインストール後
export NDI_SDK_DIR="/Library/NDI SDK for Apple"
export DYLD_LIBRARY_PATH="$NDI_SDK_DIR/lib/macOS:$DYLD_LIBRARY_PATH"
```

Linux:
```bash
export NDI_SDK_DIR="/usr/share/NDI SDK for Linux"
export LD_LIBRARY_PATH="$NDI_SDK_DIR/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH"
```

### 送信しても受信側に表示されない

**チェック項目:**

1. **ネットワーク**: 同一サブネット内か確認
2. **ファイアウォール**: NDI ポート（TCP 5960-5969, UDP 5960+）を開放
3. **NDI ソース名**: 特殊文字を避ける
4. **フレームレート**: 妥当な値か（例: 30/1, 60/1）

```rust
// デバッグ: 接続数を確認
let connections = sender.connections();
println!("Connected receivers: {}", connections);
```

### フレーム落ちが多い

**原因と対策:**

| 原因 | 対策 |
|------|------|
| CPU 負荷 | UYVY フォーマット使用 |
| 帯域不足 | 解像度を下げる |
| 同期送信 | send_video_async() 使用 |
| GStreamer ボトルネック | queue エレメント追加 |

```rust
// queue を追加してバッファリング
let queue = gst::ElementFactory::make("queue")
    .property("max-size-time", 500_000_000u64)  // 500ms
    .property("max-size-buffers", 0u32)
    .property("max-size-bytes", 0u32)
    .build()?;

// decode → queue → convert → ...
```

### メモリリーク

**注意点:**

```rust
// NG: コールバック内で毎回 Vec を作成
.new_sample(move |sink| {
    let data = sink.pull_sample()?.buffer()?.map_readable()?.to_vec();
    // data は毎回アロケート → メモリ増加
})

// OK: 事前確保したバッファを再利用
let buffers = Arc::new(Mutex::new([vec![0u8; size], vec![0u8; size]]));
.new_sample(move |sink| {
    let map = sink.pull_sample()?.buffer()?.map_readable()?;
    buffers.lock().unwrap()[current].copy_from_slice(map.as_slice());
})
```

---

## 実装時の注意点（grafton-ndi 0.9）

### SenderOptions の制約

```rust
// NG: clock_video と clock_audio を両方 false にできない
let options = SenderOptions::builder(name)
    .clock_video(false)
    .clock_audio(false)  // InvalidConfiguration エラー
    .build();

// OK: 少なくとも一方は true にする
let options = SenderOptions::builder(name)
    .clock_video(true)   // GStreamer の appsink.sync=true で同期するので問題なし
    .clock_audio(false)
    .build();
```

### VideoFrame の作成

```rust
// grafton-ndi 0.9 では VideoFrame::try_from_uncompressed() が存在しない
// VideoFrame::builder() を使用してフレームを作成し、データをコピーする

let mut frame = VideoFrame::builder()
    .resolution(width, height)
    .pixel_format(PixelFormat::UYVY)
    .frame_rate(fps_n, fps_d)
    .build()?;

// データをコピー
let copy_len = data.len().min(frame.data.len());
frame.data[..copy_len].copy_from_slice(&data[..copy_len]);

sender.send_video(&frame);  // 同期送信
```

### 非同期送信の注意

```rust
// send_video_async() は AsyncVideoToken を返す
// タイムアウトが発生する場合がある（特に初回フレーム）
// Warning: AsyncVideoToken dropped after timeout waiting for NDI completion callback

// 安定性を優先する場合は同期送信 send_video() を使用
sender.send_video(&frame);  // ブロッキング、タイムアウトなし
```

### NDI SDK パスの設定

`.cargo/config.toml` で環境変数を設定：

```toml
# macOS
[target.aarch64-apple-darwin.env]
NDI_SDK_DIR = "/Library/NDI SDK for Apple"

# Windows
[target.x86_64-pc-windows-msvc.env]
NDI_SDK_DIR = "C:\\Program Files\\NDI\\NDI 6 SDK"

# Linux
[target.x86_64-unknown-linux-gnu.env]
NDI_SDK_DIR = "/usr/share/NDI SDK for Linux"
```

---

## 参考リンク

- [grafton-ndi GitHub](https://github.com/GrantSparks/grafton-ndi)
- [grafton-ndi docs.rs](https://docs.rs/grafton-ndi)
- [NDI SDK Documentation](https://docs.ndi.video/)
- [NDI SDK Download](https://ndi.video/for-developers/ndi-sdk/)
- [GStreamer gst-plugin-ndi](https://github.com/teltek/gst-plugin-ndi) (参考用)

---

*最終更新: 2025-12-29*
