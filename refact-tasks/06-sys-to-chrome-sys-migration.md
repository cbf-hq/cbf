# 06: `cbf-sys` -> `cbf-chrome-sys` 移行

## 対象ファイル/モジュール

- `crates/cbf-sys/src/ffi.rs`
- `crates/cbf-sys/src/modifiers.rs`
- `crates/cbf/src/ffi/*`
- `chromium/src/chrome/browser/cbf/bridge/*` との契約面

## 現状APIと問題点

- `cbf-sys` が実質 Chromium専用 ABI を持つが、命名上は汎用に見える。
- unsafe/wire責務が `cbf` 側実装と密結合。

## 移行先

- `cbf-chrome-sys`:
  - `Cbf*` ABIと `cbf_bridge_client_*` extern を一元化
- `cbf`:
  - `cbf-chrome-sys` 非依存

## 必要な再設計内容

1. `cbf-sys` の公開面を `cbf-chrome-sys` へ移設
2. `cbf-chrome` から `cbf-chrome-sys` を使用する構造へ変更
3. crate説明・README・セットアップ導線を更新

## 受け入れ条件

- `cbf` が `cbf-chrome-sys` に依存しない。
- `cbf-chrome` が bridge ABI を経由して正常に接続可能。
- 既存FFI呼び出しの機能退行がない。

