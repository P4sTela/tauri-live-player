# CLAUDE.md - プロジェクト固有の指示

このファイルはClaude Codeがこのリポジトリで作業する際のガイダンスを提供します。

## プロジェクト概要

TauriLivePlayer - ライブイベント向けマルチ出力メディアプレイヤー
- **フロントエンド**: React + TypeScript + Vite + Tailwind CSS
- **バックエンド**: Rust + Tauri v2 + GStreamer
- **出力**: Display, NDI, Audio (将来: Syphon, Spout)

## ビルドコマンド

```bash
# 開発サーバー起動
pnpm tauri dev

# ビルド（デバッグ）
pnpm tauri build --debug

# Rustのみチェック（高速）
cd src-tauri && cargo check

# Rustのみビルド
cd src-tauri && cargo build
```

## セッション情報の保存

**重要**: セッションの情報はSerenaのメモリ機能を使って保存してください。

```
mcp__plugin_serena_serena__write_memory
mcp__plugin_serena_serena__read_memory
mcp__plugin_serena_serena__list_memories
```

会話の終了時や重要なマイルストーン時に、以下の情報を保存：
- 作業中のタスク
- 解決した問題と解決方法
- 次に取り組むべき課題
- 重要な技術的決定事項

## ドキュメント

`claudedocs/` ディレクトリに技術ドキュメントを配置：
- `TECHNICAL_DESIGN.md` - アーキテクチャと設計
- `PROJECT_PLAN_v6.md` - 開発計画
- `GSTREAMER_PIPELINE.md` - パイプライン設計
- `NDI_SDK_REFERENCE.md` - NDI実装リファレンス

変更時は該当ドキュメントも更新すること。

## 重要な技術的決定事項

### NDI出力
- `ndisink`ではなく`appsink` + grafton-ndi (NDI SDK直接呼び出し) を使用
- 理由: ndisinkはライブシンクとして動作し、非ライブソース(filesrc)と組み合わせると13秒のpositionオフセットが発生
- 詳細: `claudedocs/NDI_SDK_REFERENCE.md`

### GStreamerパイプライン
- 単一パイプラインで全出力を管理（自動同期のため）
- appsinkはパイプラインに`add()`してからリンクすること
- 詳細: `claudedocs/GSTREAMER_PIPELINE.md`

## コーディング規約

### Rust
- `#![allow(dead_code)]` 等は `src/lib.rs` で開発中のみ許可
- エラーは `AppError` enum を使用
- ログは `tracing` クレートを使用

### TypeScript/React
- Zustand でステート管理
- shadcn/ui コンポーネントを使用
- 型定義は `src/types.ts`

## NDI SDK設定

`.cargo/config.toml` でプラットフォーム別にNDI_SDK_DIRを設定済み：
- macOS: `/Library/NDI SDK for Apple`
- Windows: `C:\Program Files\NDI\NDI 6 SDK`
- Linux: `/usr/share/NDI SDK for Linux`

## コミット規約

```
feat: 新機能追加
fix: バグ修正
docs: ドキュメント更新
refactor: リファクタリング
build: ビルド設定変更
chore: その他
```
