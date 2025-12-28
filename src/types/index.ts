// ========================================
// メディアアイテム（1つのファイル）
// ========================================
export interface MediaItem {
  id: string;
  type: "video" | "audio";
  name: string;
  path: string;
  outputId: string;
  offset?: number; // 開始オフセット（秒）
  trimStart?: number; // トリム開始位置
  trimEnd?: number; // トリム終了位置
}

// ========================================
// キュー（同期再生するメディアのグループ）
// ========================================
export interface Cue {
  id: string;
  name: string;
  items: MediaItem[];
  duration: number; // 最長アイテムの長さ
  loop: boolean;
  autoAdvance: boolean; // 終了時に次のキューへ
  color?: string; // UI表示用カラー
}

// ========================================
// 出力先の定義
// ========================================
export type OutputType = "display" | "ndi" | "audio";
export type AudioDriver =
  | "auto"
  | "asio"
  | "wasapi"
  | "coreaudio"
  | "jack"
  | "alsa";

export interface OutputTarget {
  id: string;
  name: string;
  type: OutputType;

  // 映像出力共通
  brightness?: number | null; // null = Masterに連動、number = 個別値

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
// プロジェクト設定
// ========================================
export interface ProjectSettings {
  defaultBrightness: number;
  autoSave: boolean;
  previewQuality: "low" | "medium" | "high";
}

// ========================================
// プロジェクト
// ========================================
export interface Project {
  id: string;
  name: string;
  masterBrightness: number;
  masterVolume: number;
  outputs: OutputTarget[];
  cues: Cue[];
  settings: ProjectSettings;
}

// ========================================
// プレイヤー状態
// ========================================
export type PlayerStatus =
  | "idle"
  | "loading"
  | "ready"
  | "playing"
  | "paused"
  | "error";

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

// ========================================
// ASIOデバイス情報
// ========================================
export interface AsioDevice {
  name: string;
  clsid: string;
}

// ========================================
// ユーティリティ型
// ========================================
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};
