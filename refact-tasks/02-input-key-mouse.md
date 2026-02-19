# 02: key/mouse 分解と再配置

## 対象ファイル/モジュール

- `crates/cbf/src/data/key.rs`
- `crates/cbf/src/data/mouse.rs`
- `crates/cbf/src/command.rs`（入力コマンドpayload）
- `crates/cbf/src/ffi/map.rs`（変換）

## 現状APIと問題点

- `windows_key_code` / `native_key_code` など Chromium寄り語彙が `cbf` に露出。
- wheel `phase` / `momentum_phase` が raw相当のまま `cbf` に露出。

## 移行先

- `cbf`:
  - browser-genericな key/mouse の最小語彙
- `cbf-chrome`:
  - Chromium固有フィールドを含む raw/safe 拡張型

## 必要な再設計内容

1. `KeyEvent` の generic core と chrome拡張へ分解
2. `MouseWheelEvent` の phase系を `cbf-chrome` 側へ移動
3. 変換層を `cbf-chrome` 側に寄せる（`to_raw_command` / `to_generic_event`）

## 受け入れ条件

- `cbf` の入力型に `windows_*` 等のChromium名詞が残っていない。
- `cbf-chrome` で raw入力を明示的に扱える。
- 主要入力コマンドの送信経路が壊れていない。

