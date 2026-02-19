# 04: drag payload 再編（mapping準拠）

## 対象ファイル/モジュール

- `crates/cbf/src/data/drag.rs`
- `crates/cbf/src/command.rs`（drag command）
- `crates/cbf/src/event.rs`（drag event）
- `docs/knowledge/drag-data-field-mapping.md`
- bridge側 drag conversion 関連

## 現状APIと問題点

- `DragData` は `content::DropData` の subset のみ露出。
- 追加予定項目の配置基準が曖昧になりやすい。

## 移行先

- `cbf`:
  - browser-generic に意味固定できる drag項目
- `cbf-chrome` / `cbf-chrome-sys`:
  - Chromium固有/内部依存項目

## 必要な再設計内容

1. `drag-data-field-mapping.md` の分類を実装設計へ反映
2. `allowed_operations` など raw寄り項目の配置を確定
3. 未露出項目追加時の配置ルールをコードコメント/ドキュメントへ明記

## 受け入れ条件

- `cbf` 側 drag型に Chromium内部語彙が直接漏れていない。
- `cbf-chrome` 側で必要な drag詳細にアクセスできる。
- 既存 drag start/update/drop 流れが維持される。

