import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * 秒数を MM:SS 形式にフォーマット
 */
export function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

/**
 * 秒数を HH:MM:SS 形式にフォーマット
 */
export function formatTimeLong(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours.toString().padStart(2, "0")}:${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

/**
 * UUIDを生成
 */
export function generateId(): string {
  return crypto.randomUUID();
}

/**
 * ファイルパスからファイル名を取得
 */
export function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

/**
 * ファイル拡張子からメディアタイプを判定
 */
export function getMediaType(path: string): "video" | "audio" {
  const ext = path.split(".").pop()?.toLowerCase();
  const audioExtensions = ["wav", "mp3", "flac", "aac", "ogg", "m4a", "aiff"];
  return audioExtensions.includes(ext || "") ? "audio" : "video";
}
