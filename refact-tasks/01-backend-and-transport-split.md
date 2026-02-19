# 01: backend/transport 分離 (`cbf` core と `cbf-chrome` 拡張)

## 対象ファイル/モジュール

- `crates/cbf/src/browser.rs`
- `crates/cbf/src/command.rs`
- `crates/cbf/src/event.rs`
- `crates/cbf/src/backend_delegate/*`
- `crates/cbf/src/error.rs`

## 現状APIと問題点

- `Backend` が `BrowserCommand` / `BrowserEvent` に固定されており、raw経路の設計余地が不足。
- `BrowserEvent::BackendReady { backend_name }` が実装依存情報を含む。
- `WebPageEvent::PermissionRequested` が `response_channel` に依存している。

## 移行先

- `cbf`:
  - browser-generic APIのみ
  - `BackendReady` は ready事実のみ
  - permission応答は `BrowserCommand::ConfirmPermission`（`request_id` 相関）
- `cbf-chrome`:
  - `ChromeCommand` / `ChromeEvent` と raw拡張API

## 必要な再設計内容

1. `Backend` trait を `RawCommand` / `RawEvent` ベースに再定義
2. `EventStream::recv` を `OpaqueEvent` 経由へ整理
3. `BrowserEvent::BackendReady` から `backend_name` を削除
4. `WebPageEvent::PermissionRequested` から `response_channel` を削除
5. `BrowserCommand::ConfirmPermission` を追加

## 受け入れ条件

- `cbf` 公開APIに Chromium固有語彙がない。
- permission応答が command相関で完結する。
- `backend_name` を使う呼び出しが `cbf` 側に残っていない。
- `cargo check -p cbf` が通る。

