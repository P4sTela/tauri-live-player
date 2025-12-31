# TauriLivePlayer 開発計画書

## 目次

1. [プロジェクト概要](#プロジェクト概要)
2. [背景・動機](#背景動機)
3. [要件](#要件)
4. [技術選定](#技術選定)
5. [システムアーキテクチャ](#システムアーキテクチャ)
6. [データ構造](#データ構造)
7. [**進捗状況**](#進捗状況) ⭐
8. [開発フェーズ](#開発フェーズ)
9. [マイルストーン](#マイルストーン)
10. [リスクと対策](#リスクと対策)
11. [付録: 技術詳細](#付録-技術詳細)

---

## プロジェクト概要

| 項目 | 内容 |
|------|------|
| プロジェクト名 | TauriLivePlayer |
| 目的 | ライブイベント向けマルチPC同期映像再生システム |
| 技術スタック | Tauri 2.0 + React + TypeScript + GStreamer |
| 対象OS | Windows / macOS / Linux |
| 開発期間 | 約18週間（4.5ヶ月） |

---

## 背景・動機

### なぜ作るのか

1. **TouchDesignerのコスト問題**
   - Pro ライセンス: $2,200 USD
   - 使う機能は全体のごく一部（動画再生、TC同期、出力程度）

2. **既存ツールの不足**
   - NDI + タイムコードチェイス機能を持つフリーソフトが存在しない

3. **Rustを書きたい**
   - 長時間稼働に耐える信頼性
   - パフォーマンスとメモリ安全性の両立

---

## 要件

### 機能要件

| 優先度 | 要件 | 説明 |
|--------|------|------|
| **必須** | Cueベース再生 | 1つのCueで複数メディア（音声×1 + 映像×4など）を同期再生 |
| **必須** | マルチ出力 | Display / NDI / Audio を個別に出力先指定 |
| **必須** | 明るさ調整 | 全出力一括 + 個別調整 |
| **必須** | Windows ASIO | ライブ現場向け低レイテンシ音声出力 |
| **高** | **マルチPC同期** | **マスター/スレーブ構成で複数PCを同期再生** |
| 高 | NDI送受信 | High Bandwidth対応 |
| 高 | トラックリストUI | キーボード操作、D&D対応 |
| 中 | NDI|HX | H.264/H.265圧縮送信（低帯域環境用） |
| 中 | マルチスクリーン | 複数映像を1ストリームに合成 / 1ストリームを分割 |
| 低 | Syphon/Spout | 同一PC内の他アプリとフレーム共有 |

### 非機能要件

| 項目 | 要件 |
|------|------|
| 信頼性 | 長時間稼働（8時間+）で安定動作 |
| パフォーマンス | 4K60fps再生、複数ストリーム同時処理 |
| **同期精度** | **60fps（16.7ms）以内のフレーム同期** |
| UI | モダンで綺麗なUI（React + shadcn/ui） |
| プラットフォーム | Windows（現場） + macOS（開発） + Linux |

---

## 技術選定

### 採用技術

| カテゴリ | 技術 | 選定理由 |
|---------|------|----------|
| フレームワーク | Tauri 2.0 | 軽量、Rust、WebView UI |
| フロントエンド | React + TypeScript | 開発効率、エコシステム |
| UIライブラリ | shadcn/ui | 綺麗、カスタマイズ性 |
| 映像処理 | GStreamer | パイプライン、NDI対応、実績 |
| Rustバインディング | gstreamer-rs | 公式サポート |
| 状態管理 | Zustand | シンプル、TypeScript |

### プラットフォーム別対応

| コンポーネント | Windows | macOS | Linux |
|---------------|---------|-------|-------|
| Tauri | ◎ WebView2 | ◎ WebKit | ◎ WebKitGTK |
| GStreamer | ◎ | ◎ | ◎ |
| NDIプラグイン | ○ | ○ | ◎ |
| ASIO | ◎ 必須 | - | - |
| HWエンコード | NVENC/QSV/AMF | VideoToolbox | VA-API/NVENC |

---

## システムアーキテクチャ

### 全体構成

```
┌─────────────────────────────────────────────────────────────────┐
│  Tauri App                                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Frontend (React + TypeScript)                          │   │
│  │  ├─ PlayView (Cueリスト、トランスポート)                 │   │
│  │  ├─ EditView (Cue編集)                                  │   │
│  │  └─ Settings (出力設定、同期設定)                        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │ IPC                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Backend (Rust)                                         │   │
│  │  ├─ commands/     (Tauri コマンド)                      │   │
│  │  ├─ pipeline/     (GStreamer)                           │   │
│  │  │   ├─ cue_player.rs                                   │   │
│  │  │   ├─ ndi_sender.rs                                   │   │
│  │  │   └─ syphon_sender.rs                                │   │
│  │  ├─ output/       (Display/Audio出力)                   │   │
│  │  ├─ sync/         (マルチPC同期) ※Phase 6              │   │
│  │  └─ types/        (共通型)                              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ↓                     ↓                     ↓
   ┌─────────┐          ┌─────────┐          ┌─────────┐
   │ Display │          │   NDI   │          │  ASIO   │
   │ Window  │          │ Stream  │          │  Audio  │
   └─────────┘          └─────────┘          └─────────┘
```

### NDI 出力アーキテクチャ

**採用方式: appsink + NDI SDK 直接呼び出し**

```
GStreamer Pipeline
┌─────────────────────────────────────────────────────────────┐
│ filesrc → decodebin → videoconvert → videobalance          │
│                                           ↓                 │
│                                    capsfilter (UYVY)        │
│                                           ↓                 │
│                                    appsink (sync=true)      │
└───────────────────────────────────────────┼─────────────────┘
                                            │ new-sample callback
                                            ↓
┌───────────────────────────────────────────────────────────────┐
│  NdiSender (Rust)                                             │
│  ├─ PTS を last_pts に記録                                    │
│  ├─ フレームデータ取得                                         │
│  └─ grafton-ndi で NDI 送信                                   │
└───────────────────────────────────────────────────────────────┘
```

**理由:** ndisink を使うと `query_position()` が不正確（13秒オフセット問題）

---

## データ構造

### 概念モデル

```
Project
├── name: string
├── masterBrightness: number
├── outputs: OutputTarget[]     # 出力先定義
├── cues: Cue[]                 # キューリスト
├── sync: SyncConfig            # 同期設定
└── items: MediaItem[]          # 各キュー内のメディア
```

### TypeScript型定義

```typescript
interface Project {
  id: string;
  name: string;
  outputs: OutputTarget[];
  cues: Cue[];
  sync?: SyncConfig;
}

interface OutputTarget {
  id: string;
  name: string;
  type: 'display' | 'ndi' | 'audio';
  
  // 映像出力共通
  brightness?: number;       // 個別明るさ (0-100, 省略時はマスター値を使用)
  
  // Display
  displayIndex?: number;
  fullscreen?: boolean;
  
  // NDI
  ndiName?: string;
  
  // Audio
  audioDriver?: 'auto' | 'asio' | 'wasapi' | 'coreaudio' | 'jack';
  audioDevice?: string;
  audioChannels?: number[];
}

interface Cue {
  id: string;
  name: string;
  items: MediaItem[];
  duration: number;
  loop: boolean;
  autoAdvance: boolean;
}

interface MediaItem {
  id: string;
  type: 'video' | 'audio';
  name: string;
  path: string;          // 相対パス（media_rootから解決）
  outputId: string;
  offset?: number;
}

// 同期設定
interface SyncConfig {
  mode: 'master' | 'slave' | 'off';
  
  // 送信設定（マスター時）
  transport: SyncTransport;
  sendIntervalHz: number;  // 60
  
  // 受信設定（スレーブ時）
  listenPort: number;
  multicastGroup?: string;
  timeoutMs: number;       // 500
  
  // 共通
  toleranceFrames: number; // 許容誤差（フレーム数）
}

type SyncTransport = 
  | { type: 'broadcast'; port: number }
  | { type: 'multicast'; addr: string; port: number }
  | { type: 'unicast'; targets: string[] };  // "ip:port"

// PC固有のローカル設定（プロジェクトファイルとは別）
interface LocalConfig {
  mediaRoot: string;        // メディアファイルのルートフォルダ
  latencyOffsetMs: number;  // 遅延オフセット（ms）
}
```

### プロジェクトファイル例

```json
{
  "name": "Miku Live 2025",
  "masterBrightness": 80,
  "outputs": [
    { "id": "main", "type": "display", "displayIndex": 1, "brightness": null },
    { "id": "side", "type": "ndi", "ndiName": "LivePlayer_Side", "brightness": 60 },
    { "id": "audio", "type": "audio", "audioDriver": "asio", "audioDevice": "{...}" }
  ],
  "cues": [
    {
      "name": "M01_Opening",
      "items": [
        { "type": "audio", "path": "M01/audio.wav", "outputId": "audio" },
        { "type": "video", "path": "M01/main.mp4", "outputId": "main" },
        { "type": "video", "path": "M01/side.mp4", "outputId": "side" }
      ]
    }
  ],
  "sync": {
    "mode": "master",
    "transport": { "type": "broadcast", "port": 7000 },
    "sendIntervalHz": 60,
    "toleranceFrames": 1
  }
}
```

- `brightness: null` → Masterに連動
- `brightness: 60` → 個別値（Masterに連動しない）
- `path` は相対パス（各PCの `mediaRoot` から解決）

### ローカル設定ファイル例（各PC固有）

```json
{
  "mediaRoot": "/Users/vj/Videos/MikuLive2025",
  "latencyOffsetMs": -15
}
```

Windows の場合:
```json
{
  "mediaRoot": "D:\\LiveVideos\\MikuLive2025",
  "latencyOffsetMs": 0
}
```

---

## 進捗状況

*最終更新: 2025-12-31*

### 実装完了フェーズ

| Phase | 状態 | 完了時期 | 備考 |
|-------|------|---------|------|
| Phase 0: 環境構築 | ✅ 完了 | 2025-12 | macOS開発環境構築済み |
| Phase 1: 基本再生 | ✅ 完了 | 2025-12 | CuePlayer実装、GStreamerパイプライン動作 |
| Phase 2: UI実装 | ✅ 完了 | 2025-12 | PlayView/EditView実装、Cue編集可能 |
| Phase 3: マルチ出力 | ✅ 完了 | 2025-12 | Display/Audio出力実装 |
| Phase 4a: NDI出力 | ✅ 完了 | 2025-12-29 | appsink + grafton-ndi方式、position同期解決 |
| Phase 4c: Syphon出力 | ✅ 完了 | 2025-12-31 | macOS Syphon実装、コードレビュー修正完了 |

### 未着手/計画中フェーズ

| Phase | 状態 | 優先度 | 備考 |
|-------|------|--------|------|
| Phase 4b: NDI\|HX | ⬜ 未着手 | 中 | H.264/H.265圧縮送信、Advanced SDK必要時 |
| Phase 5: マルチスクリーン | ⬜ 未着手 | 中 | compositor使用、複数映像合成 |
| Phase 6: マルチPC同期 | ⬜ 未着手 | 高 | 次期重点機能 |

**アイコン凡例:** ✅完了 / 🔄進行中 / ⬜未着手

### 主要な技術的決定

| 項目 | 決定内容 | 理由 |
|------|----------|------|
| NDI実装方式 | appsink + grafton-ndi | ndisinkはpositionオフセット問題あり |
| Syphon実装方式 | appsink + objc2 FFI | 直接SDK呼び出し、OpenGL経由 |
| 出力同期 | appsink sync=true | GStreamerのクロック同期を活用 |
| エラーハンドリング | AppError enum | 統一的なエラー管理 |

### 技術負債・改善検討事項

- [ ] Windows ASIO対応（現在はmacOSのみ開発）
- [ ] NDI|HX実装（必要時）
- [ ] Spout実装（Windows、優先度低）
- [ ] テクスチャ更新最適化（Syphon: glTexSubImage2D検討）

---

## 開発フェーズ

### 概要

```
Phase 0: 環境構築                    [1週間]
    ↓
Phase 1: 基本再生 (GStreamer)        [2週間]
    ↓
Phase 2: UI実装（Cueベース）          [2週間]
    ↓
Phase 3: マルチ出力ウィンドウ         [2週間]
    ↓
Phase 4a: NDI (High Bandwidth)       [2週間]
    ↓
Phase 4b: NDI|HX (H.264/H.265)       [2週間] ※必要時
    ↓
Phase 4c: Syphon/Spout               [1週間] ※必要時、優先度低
    ↓
Phase 5: マルチスクリーン             [2週間]
    ↓
Phase 6a: マルチPC同期（基本）        [1.5週間]
    ↓
Phase 6b: マルチPC同期（堅牢化）      [1.5週間]
    ↓
Phase 7: リリース                    [1週間]

総期間: 約18週間（4.5ヶ月）
```

---

### Phase 0: 環境構築 ✅

**概要:** Tauri + GStreamer + Rust開発環境の構築。

**成果物:**
- macOS開発環境（Rust, Node.js, GStreamer, Tauri CLI）
- Tauriプロジェクト作成完了

**技術スタック:**
- Tauri 2.0, React, TypeScript, GStreamer 0.23, Zustand

**注意点:**
- Windows ASIO対応は未実装（macOSで開発中）
- GStreamer ASIOプラグインは自前ビルド必要

---

### Phase 1: 基本再生 ✅

**概要:** GStreamerパイプラインを使った動画/音声再生機能の実装。

**成果物:**
- `pipeline/cue_player.rs` - Cueベース再生制御
- `pipeline/media_handler.rs` - 映像/音声パイプライン構築
- `commands/player.rs` - Tauri IPC コマンド

**パイプライン構成:**
```
filesrc → decodebin → videoconvert → videobalance → sink
                   ↘ audioconvert → volume → audiosink
```

**実装機能:**
- play/pause/stop/seek操作
- 明るさ調整（videobalance）
- 音量調整（volume element）

---

### Phase 2: UI実装 ✅

**概要:** React + shadcn/uiによるCueリスト管理UI。

**成果物:**
- `components/views/PlayView.tsx` - Cueリスト + トランスポート
- `components/views/EditView.tsx` - Cue編集画面
- `stores/playerStore.ts`, `projectStore.ts` - Zustand状態管理

**実装機能:**
- Cueリスト表示・選択
- トランスポートコントロール（再生/一時停止/停止）
- Cue編集（MediaItem追加/削除）
- キーボードショートカット（Space, ↑/↓, Enter, Esc）

---

### Phase 3: マルチ出力 ✅

**概要:** Display/Audio出力を個別ウィンドウ・デバイスに振り分け。

**成果物:**
- `output/manager.rs` - 出力先管理
- `output/native_handle.rs` - プラットフォーム別ウィンドウハンドル
- `commands/output.rs` - モニター列挙、ウィンドウ制御

**実装機能:**
- 別ウィンドウへの映像出力（Tauri WebView）
- モニター選択・フルスクリーン切り替え
- 音声デバイス選択

**技術ポイント:**
- Tauriウィンドウ → GStreamer sink連携
- macOS: `glimagesink`, Windows: `d3d11videosink`

---

### Phase 4a: NDI High Bandwidth ✅

**概要:** appsink + grafton-ndi による NDI送信実装。

**成果物:**
- `pipeline/ndi_sender.rs` - NDI送信管理

**パイプライン:**
```
filesrc → decodebin → videoconvert → capsfilter(UYVY) → appsink
                                                           ↓ callback
                                                    NdiSender (grafton-ndi)
```

**採用技術:**
- grafton-ndi 0.9 (NDI 6 SDK対応)
- UYVY形式でのゼロコピー送信

**重要な技術的決定:**
- **ndisinkではなくappsink方式を採用** → ndisinkはlive sinkのためpositionクエリが13秒ずれる問題
- appsinkのPTSを記録してposition同期
- 詳細: `claudedocs/NDI_SDK_REFERENCE.md`

---

### Phase 4b: NDI|HX ⬜

**概要:** H.264/H.265圧縮によるNDI送信（低帯域環境用）。

**ステータス:** 未着手（必要時に実装）

**要件:**
- NDI Advanced SDK ライセンス（有償）
- grafton-ndi `advanced_sdk` feature

**パイプライン:**
```
appsink (UYVY) → HWエンコード (nvh264enc/vtenc_h264) → NDI SDK
```

---

### Phase 4c: Syphon/Spout ✅

**概要:** 同一PC内の他アプリとGPUテクスチャ共有（macOS Syphon実装）。

**成果物:**
- `pipeline/syphon_sender.rs` - Syphon送信実装（macOS）

**実装方式:**
- objc2 v0.6 FFIでSyphon.framework呼び出し
- appsink (RGBA) → OpenGL texture → SyphonServer

**技術ポイント:**
- 独自CGLコンテキスト作成（GStreamerワーカースレッド用）
- Syphon.framework動的ロード（NSBundle）
- リソースリーク修正・unsafeドキュメント化完了

**注意点:**
- macOSのみ実装（Spout/Windowsは未実装）
- Universal Syphon.framework必須（arm64 + x86_64）

---

### Phase 5: マルチスクリーン（2週間）

#### 機能
- 複数映像を1ストリームに合成（2×2など）
- 1ストリームをリージョン分割

#### GStreamer

```bash
# 合成
gst-launch-1.0 compositor name=c \
  sink_0::xpos=0 sink_0::ypos=0 \
  sink_1::xpos=1920 sink_1::ypos=0 ! \
  ndisink ndi-name="MULTI" \
  filesrc location=a.mp4 ! decodebin ! c.sink_0 \
  filesrc location=b.mp4 ! decodebin ! c.sink_1
```

---

### Phase 6: マルチPC同期システム（3週間）

#### 概要

複数PCでの同期再生システム。マスターPCが再生位置とCue情報を送信し、スレーブPCがローカルファイルを同期再生する。映像データはネットワークに流さず、軽量なタイムコード情報のみを送受信。

```
┌─────────────────┐     SyncPacket (48B)     ┌─────────────────┐
│   Master PC     │ ─────────────────────────→│   Slave PC(s)   │
│                 │   - cue_id               │                 │
│  映像再生       │   - position_us          │  同じファイルを  │
│  TC送信        │   - state                │  ローカル保持    │
└─────────────────┘                          └─────────────────┘
         ↓                                            ↓
    [Display A]                               [Display B, C...]
```

#### Phase 6a: 基本同期（1.5週間）

##### パケット設計

```rust
/// 同期パケット（固定長 48 bytes）
#[repr(C, packed)]
pub struct SyncPacket {
    magic: [u8; 4],           // "TLPS"
    version: u8,              // プロトコルバージョン
    packet_type: PacketType,  // Sync / CueChange / EmergencyStop
    sequence: u16,            // パケットロス検知用
    master_timestamp_us: u64, // マスターのタイムスタンプ
    cue_id: [u8; 16],         // 現在のCue ID (UUID)
    position_us: u64,         // 再生位置 (μs)
    state: PlayState,         // Playing / Paused / Stopped
    speed: f32,               // 再生速度 (1.0 = 100%)
    _reserved: [u8; 3],
}

#[repr(u8)]
pub enum PacketType {
    Sync = 0x01,          // 通常同期（60Hz送信）
    CueChange = 0x02,     // Cue変更通知（即時）
    EmergencyStop = 0x03, // 緊急停止
    CueListRequest = 0x10,
    CueListResponse = 0x11,
}
```

##### 通信方式

3方式に対応:

| 方式 | 送信先 | 用途 |
|------|--------|------|
| Broadcast | 255.255.255.255 | 同一サブネット全体 |
| Multicast | 239.x.x.x | グループ参加者のみ |
| Unicast | 特定IPリスト | 確実な1対多 |

```rust
pub enum SyncTarget {
    Broadcast { port: u16 },
    Multicast { addr: Ipv4Addr, port: u16 },
    Unicast { targets: Vec<SocketAddr> },
}
```

##### チェイスアルゴリズム

```rust
impl SyncSlave {
    pub fn on_packet(&mut self, packet: SyncPacket, player: &mut CuePlayer) {
        // 1. Cue変更チェック → 新しいCueをロード
        // 2. 状態変更チェック → play/pause/stop
        // 3. 位置補正（チェイス）
        //    - マスターの「今の位置」を推定
        //    - 許容誤差（1フレーム）を超えたらseek
    }
}
```

##### 成果物
- [ ] SyncPacket 送受信
- [ ] マスター/スレーブモード切り替え
- [ ] 基本チェイス（seek）
- [ ] UI: 同期設定パネル

#### Phase 6b: 堅牢化（1.5週間）

##### ファイルパス解決

各PCでファイルパスが異なる問題への対応:

```
Master: /Users/vj/Videos/M01.mp4
Slave:  D:\LiveVideos\M01.mp4
```

**方式: 相対パス + プロジェクトフォルダ設定**

```rust
/// 各PCのローカル設定
pub struct LocalConfig {
    /// プロジェクトファイルのルートフォルダ
    /// 相対パスはここから解決
    pub media_root: PathBuf,
    
    /// PC固有の遅延オフセット（μs）
    pub latency_offset_us: i64,
}

// パス解決
fn resolve_path(relative: &str, config: &LocalConfig) -> PathBuf {
    config.media_root.join(relative)
}
```

プロジェクトファイルは相対パスで保存:
```json
{
  "cues": [{
    "items": [
      { "path": "M01/main.mp4" },  // 相対パス
      { "path": "M01/side.mp4" }
    ]
  }]
}
```

##### Cue変更時のプリロード

```
Cue 1 再生中
    ↓
マスターが Cue 2 送信（CueChange パケット）
    ↓
スレーブ: Cue 2 をバックグラウンドでプリロード
    ↓
プリロード完了後、切り替え実行
```

##### seek頻度制御

**課題:** 毎フレーム seek すると重い

**対策案（要調査）:**
- seek 後は N フレーム（例: 10フレーム）待機
- 小さいズレ（数フレーム）は再生速度調整で吸収
- ヒステリシス: 閾値を超えたら seek、戻るまで待機

##### スレーブ管理（Heartbeat）

```rust
/// スレーブ → マスターへの heartbeat
pub struct SlaveHeartbeat {
    slave_id: Uuid,
    slave_name: String,
    current_cue_id: [u8; 16],
    position_us: u64,
    status: SlaveStatus,
}

pub enum SlaveStatus {
    Synced,           // 正常同期中
    Seeking,          // seek中
    FileNotFound,     // ファイルなし
    Disconnected,     // 接続断
}
```

マスター側でスレーブ一覧を表示:
```
┌─────────────────────────────────────┐
│ Slaves (3 connected)                │
├─────────────────────────────────────┤
│ 🟢 Slave-A (192.168.1.101)  +2ms   │
│ 🟢 Slave-B (192.168.1.102)  +5ms   │
│ 🟡 Slave-C (192.168.1.103)  seek中 │
└─────────────────────────────────────┘
```

##### 障害対策

| シナリオ | 検知方法 | 動作 |
|---------|---------|------|
| パケットロス | sequence gap | 次パケットで補正 |
| マスター停止 | タイムアウト (500ms) | **最後の状態で継続** |
| マスター復帰 | パケット受信再開 | **即座にマスターに追従** |
| ファイル不一致 | ファイル存在チェック | 警告UI表示 |

##### 個別遅延調整

各PCの出力遅延を補正:

```
┌─────────────────────────────────────┐
│ 遅延オフセット設定                  │
├─────────────────────────────────────┤
│ このPCのオフセット: [-50    ] ms    │
│                                     │
│ ヒント: マスターより遅れる場合は    │
│ マイナス値を設定                    │
└─────────────────────────────────────┘
```

##### 将来検討: システムクロック同期

現在の設計は「相対遅延」のみで動作するが、より高精度な同期のためにシステムクロック同期も検討可能:

- **NTP**: 数十ms精度（通常十分）
- **PTP (IEEE 1588)**: μs精度（オーバーキルの可能性）

現時点では手動オフセット調整で対応し、必要に応じて将来実装。

##### 成果物
- [ ] ファイルパス解決（相対パス + media_root）
- [ ] Cueプリロード連携
- [ ] seek頻度制御
- [ ] スレーブ管理（heartbeat）
- [ ] タイムアウト処理
- [ ] 個別遅延オフセット設定
- [ ] UI: スレーブ一覧表示（マスター側）

#### UI変更

##### 新規コンポーネント

```
src/
├── components/
│   ├── sync/
│   │   ├── SyncSettings.tsx       # 同期設定パネル
│   │   ├── SyncStatus.tsx         # ステータス表示
│   │   └── SlaveList.tsx          # スレーブ一覧（マスター用）
│   └── ...
├── stores/
│   ├── syncStore.ts               # 同期状態管理
│   └── ...
└── ...
```

##### Header 変更

```tsx
// 同期インジケータ追加
<div className="flex items-center gap-2">
  {syncMode === 'master' && <Crown className="w-4 h-4 text-yellow-500" />}
  {syncMode === 'slave' && <Link className="w-4 h-4 text-blue-500" />}
  <span>{syncStatus}</span>
</div>
```

##### PlayView 変更

```tsx
// スレーブモード時はトランスポート無効化
const isSlaveMode = syncMode === 'slave';

<Button onClick={handlePlayPause} disabled={isSlaveMode}>
  ...
</Button>

{isSlaveMode && (
  <div className="text-muted-foreground">
    🔗 Slave Mode - Following Master
  </div>
)}
```

##### 設定UI（Settings タブ）

```
┌─────────────────────────────────────────────────┐
│  同期設定                                       │
├─────────────────────────────────────────────────┤
│  モード: ○ マスター  ● スレーブ  ○ 無効       │
│  ─────────────────────────────────────────────  │
│  送信方式 (マスター時)                          │
│  ● ブロードキャスト  ポート: [7000]             │
│  ○ マルチキャスト    [239.255.0.1]:[7000]       │
│  ○ ユニキャスト      [+ ターゲット追加]          │
│  ─────────────────────────────────────────────  │
│  受信設定 (スレーブ時)                          │
│  リッスンポート: [7000]                         │
│  遅延オフセット: [-15   ] ms                    │
│  ─────────────────────────────────────────────  │
│  メディアフォルダ: [/Users/vj/Videos] [選択]    │
│  ─────────────────────────────────────────────  │
│  ステータス                                     │
│  🟢 マスターに接続中 (遅延: 3ms)                │
│  最終受信: 0.02s前                              │
└─────────────────────────────────────────────────┘
```

#### タイムコード表示

GStreamer の TC エレメントは不要。UI 側で μs → SMPTE 変換:

```tsx
function TimecodeDisplay({ positionUs, fps = 59.94 }: Props) {
  const tc = useMemo(() => {
    const totalSec = positionUs / 1_000_000;
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = Math.floor(totalSec % 60);
    const f = Math.floor((totalSec % 1) * fps);
    return `${pad(h)}:${pad(m)}:${pad(s)}:${pad(f)}`;
  }, [positionUs, fps]);
  
  return <span className="font-mono text-lg">{tc}</span>;
}

// 表示例: 00:01:23:15
```

---

### Phase 7: リリース（1週間）

#### タスク
- GStreamerバンドル
- インストーラー作成（.msi / .dmg / .AppImage）
- GitHub Releases

---

## マイルストーン

| MS | 達成条件 | 目標 |
|----|---------|------|
| M0 | 環境構築完了、ASIOプラグイン動作 | 1週目 |
| M1 | GStreamer再生動作 | 3週目 |
| M2 | CueベースUI完成 | 5週目 |
| M3 | マルチ出力動作 | 7週目 |
| M4a | NDI送受信動作 | 9週目 |
| M4b | NDI\|HX動作（必要時） | 11週目 |
| M4c | Syphon/Spout動作（必要時） | 12週目 |
| M5 | マルチスクリーン動作 | 14週目 |
| M6a | 基本同期動作（マスター/スレーブ） | 16週目 |
| M6b | 同期堅牢化（スレーブ管理、障害対策） | 17週目 |
| M7 | リリースビルド完成 | 18週目 |

**Phase 6 詳細マイルストーン:**

| 項目 | チェック |
|------|---------|
| SyncPacket 送受信 | [ ] |
| Broadcast/Multicast/Unicast 対応 | [ ] |
| 基本チェイス（seek） | [ ] |
| マスター/スレーブUI | [ ] |
| ファイルパス解決（相対パス） | [ ] |
| Cueプリロード連携 | [ ] |
| スレーブ heartbeat | [ ] |
| マスター側スレーブ一覧 | [ ] |
| タイムアウト処理 | [ ] |
| 個別遅延オフセット | [ ] |

---

## リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| GStreamer学習コスト | 中 | チュートリアルから段階的に |
| **Windows ASIO** | **高** | **自前ビルド、WASAPIフォールバック** |
| ASIO SDKライセンス | 中 | Steinberg取得、再配布注意 |
| ~~ndisinkライブシンク問題~~ | ~~高~~ | ~~解決済み: appsink + NDI SDK 方式採用~~ |
| NDI SDK FFI作成 | 中 | bindgen使用、コミュニティcrate調査 |
| NDI Advanced SDK | 中 | まずHigh Bandwidthで開発 |
| Syphon/Spout FFI | 低 | Objective-C/C++ ブリッジ、既存crateあり |
| HWエンコーダ環境差 | 中 | x264enc(CPU)フォールバック |
| Linux NDI\|HXデコード | 高 | 公式サポート限定 |
| マルチOS対応 | 中 | CI/CDで自動テスト |
| 長時間稼働 | 高 | Rust + GStreamerで信頼性確保 |
| **同期seek頻度** | **中** | **頻度制御、再生速度調整（要調査）** |
| **ネットワーク遅延ばらつき** | **中** | **個別オフセット調整、将来NTP/PTP検討** |
| **ファイルパス不整合** | **中** | **相対パス + media_root、起動時検証** |

### 解決済みの問題

**ndisink ライブシンク問題（13秒オフセット）**
- 症状: `pipeline.query_position()` が不正確な値を返す
- 原因: ndisink がライブシンク、filesrc が非ライブソース
- 解決: `appsink` + NDI SDK 直接呼び出し方式を採用
- 詳細: [技術設計書](./TECHNICAL_DESIGN.md) セクション1参照

---

## 付録: 技術詳細

### A. プロジェクト構成

```
tauri-live-player/
├── src/                          # フロントエンド
│   ├── components/
│   │   ├── views/
│   │   │   ├── PlayView.tsx
│   │   │   └── EditView.tsx
│   │   ├── player/
│   │   ├── output/
│   │   ├── sync/                 # Phase 6
│   │   └── ui/
│   ├── stores/
│   │   ├── playerStore.ts
│   │   ├── projectStore.ts
│   │   ├── outputStore.ts
│   │   └── syncStore.ts          # Phase 6
│   ├── hooks/
│   └── types/
├── src-tauri/                    # バックエンド
│   ├── src/
│   │   ├── commands/
│   │   ├── pipeline/
│   │   │   ├── cue_player.rs
│   │   │   ├── ndi_sender.rs
│   │   │   └── syphon_sender.rs
│   │   ├── output/
│   │   ├── sync/                 # Phase 6
│   │   │   ├── mod.rs
│   │   │   ├── packet.rs
│   │   │   ├── master.rs
│   │   │   ├── slave.rs
│   │   │   └── transport.rs
│   │   ├── audio/
│   │   ├── types/
│   │   ├── state.rs
│   │   └── lib.rs
│   └── Cargo.toml
└── claudedocs/                   # ドキュメント
    ├── PROJECT_PLAN_v7.md
    ├── TECHNICAL_DESIGN.md
    └── docs/
        ├── GSTREAMER_PIPELINE.md
        └── NDI_SDK_REFERENCE.md
```

### B. 参照リンク

**公式ドキュメント**
- [GStreamer](https://gstreamer.freedesktop.org/documentation/)
- [gstreamer-rs](https://gitlab.freedesktop.org/gstreamer/gstreamer-rs)
- [Tauri](https://tauri.app/)
- [NDI SDK](https://ndi.video/for-developers/)

**コミュニティ**
- [gst-plugin-ndi](https://github.com/teltek/gst-plugin-ndi)
- [grafton-ndi](https://github.com/GrantSparks/grafton-ndi)

---

## 関連ドキュメント

- **[技術設計書 (TECHNICAL_DESIGN.md)](./TECHNICAL_DESIGN.md)** - 詳細な実装コード、型定義、パイプライン設計
- **[同期アルゴリズム (SYNC_ALGORITHM.md)](./SYNC_ALGORITHM.md)** - チェイスアルゴリズム、音声マスター方式、パラメータ設計
- **[パイプラインリファレンス (docs/GSTREAMER_PIPELINE.md)](./docs/GSTREAMER_PIPELINE.md)** - GStreamer パイプライン設計、エレメント一覧、コマンド例
- **[NDI SDK リファレンス (docs/NDI_SDK_REFERENCE.md)](./docs/NDI_SDK_REFERENCE.md)** - grafton-ndi 使用法、ゼロコピー送信、GStreamer 統合

---

## 次のアクション

1. [ ] macOS: GStreamerインストール、動作確認
2. [ ] Windows: GStreamer + ASIO SDKセットアップ
3. [ ] Tauriプロジェクト作成
4. [ ] `gst-launch-1.0` で動画再生テスト
5. [ ] gstreamer-rs Hello World

---

## 要調査事項

- [ ] seek 頻度制御の最適値（フレーム数、再生速度調整）
- [ ] マスター復帰時の挙動（メンバーと検討）

---

*最終更新: 2025-12-31*  
*バージョン: v7.1* (進捗状況セクション追加、完了フェーズ要約)  
*作成者: Claude + Kota*
