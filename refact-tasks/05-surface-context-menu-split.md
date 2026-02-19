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

