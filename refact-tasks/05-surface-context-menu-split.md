# 05: surface/context-menu 分離

## 対象ファイル/モジュール

- `crates/cbf/src/data/surface.rs`
- `crates/cbf/src/data/context_menu.rs`
- `crates/cbf/src/event.rs`（`SurfaceHandleUpdated`, `ContextMenuRequested`）
- bridge変換（surface/menu）

## 現状APIと問題点

- `SurfaceHandle` が platform/chromium依存のまま `cbf` に存在。
- context menu command ID定数が Chromium由来の値として `cbf` に露出。

## 移行先

- `cbf`:
  - surface handle は公開しない（または最小genericのみ）
  - context menu は generic payload中心
- `cbf-chrome`:
  - `SurfaceHandleUpdated` 相当
  - `CMD_*` 定数群

## 必要な再設計内容

1. `WebPageEvent::SurfaceHandleUpdated` を `cbf` から外し拡張イベント化
2. `CMD_*` 定数を `cbf-chrome` へ移設
3. `ContextMenu` payload の generic core を定義

## 受け入れ条件

- `cbf` から `SurfaceHandle` 依存が除去される。
- `cbf` の menu API が Chromium command IDへ依存しない。
- `cbf-chrome` 側で既存機能を代替可能。

## 実施結果（2026-02-20）

- `cbf` から `SurfaceHandle` 公開を削除した
  - `crates/cbf/src/event.rs` の `BrowsingContextEvent::SurfaceHandleUpdated` を削除
  - `crates/cbf/src/data/mod.rs` から `surface` モジュール公開を削除
  - `crates/cbf/src/data/surface.rs` を削除
- `cbf` の context menu から Chromium command ID 依存を削除した
  - `crates/cbf/src/data/context_menu.rs` から `CMD_*` 定数群と allowlist/filter helper を削除
  - `ContextMenu` / `ContextMenuItem` など generic payload 型は維持
- `cbf-chrome` に Chromium 固有 surface/menu 拡張を移した
  - 追加: `crates/cbf-chrome/src/surface.rs`
  - 追加: `crates/cbf-chrome/src/context_menu.rs`（`CMD_*`, `filter_supported`, helper）
  - `crates/cbf-chrome/src/ffi/map.rs` の menu filter 呼び出しを `crate::context_menu::filter_supported` に変更
  - `crates/cbf-chrome/src/ffi/*` の surface 型参照を `crate::surface::SurfaceHandle` に変更
- `cbf-chrome` backend から generic event への surface 変換を停止した
  - `IpcEvent::SurfaceHandleUpdated` は `cbf` の `BrowserEvent` へは射影しない（`cbf` generic 境界を維持）
