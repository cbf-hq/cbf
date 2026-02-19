# 03: IME payload 分解（generic core + chrome詳細）

## 対象ファイル/モジュール

- `crates/cbf/src/data/ime.rs`
- `crates/cbf/src/command.rs`（IME関連 command）
- `crates/cbf/src/event.rs`（IME bounds event）
- `crates/cbf/src/ffi/map.rs`（IME変換）

## 現状APIと問題点

- `ImeTextSpan` の見た目属性（thickness/style/color/flags）が Chromium実装依存。
- `cbf` の抽象層で表現密度が高すぎる。

## 移行先

- `cbf`:
  - IMEの generic core（編集・確定に必要な最小情報）
- `cbf-chrome`:
  - `ui::ImeTextSpan` 対応の詳細属性

## 必要な再設計内容

1. `ImeTextSpan` を core と chrome詳細へ分割
2. `ImeComposition` / `ImeCommitText` の payload を再構築
3. `ImeBoundsUpdate` の generic表現と chrome詳細の境界を固定

## 受け入れ条件

- `cbf` 側 IME型が browser-generic として説明可能。
- `cbf-chrome` 側で既存Chromium機能が欠落しない。
- IME関連 command/event の型変換が一貫している。

## 実装メモ（2026-02-19）

- `ImeTextSpan` を generic core（`type/start/end`）中心へ再構成し、Chromium依存の見た目属性は `chrome_style: Option<ChromeImeTextSpanStyle>` に隔離。
- 非公開プロジェクト方針に合わせ、従来フィールドと従来名 enum の互換レイヤは削除（破壊的更新許容）。
- FFI変換は `chrome_style` を優先し、未指定時は `ChromeImeTextSpanStyle::default()` を適用。
- `BrowserCommand::{SetComposition, CommitText}` と `BrowsingContextEvent::ImeBoundsUpdated` のコメントを更新し、generic境界を明文化。
