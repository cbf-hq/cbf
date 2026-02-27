# Chromium macOS Host-owned DnD: ドラッグプレビュー巨大化の原因と対処

macOS の host-owned drag and drop 実装で、ドラッグプレビュー画像が過大表示される問題の原因と修正内容を整理する。

## 問題

- リンクや draggable 要素のドラッグ時に、プレビュー画像が意図より大きく表示される。

## 主原因

- Chromium 側で `image->scale` を `drag_obj_rect_in_dip.width / drag_image.width`（DIP/pixel）として送っていた。
  - `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`
- Rust/macOS 側は `NSImage` の表示サイズを `pixel_width / scale` で計算していた。
  - `crates/cbf/src/platform/macos/browser_view.rs`

この 2 つを組み合わせると、Retina 環境（pixel > DIP）では `scale < 1` になり、`pixel / scale` で過大なサイズになる。

## 修正

- Chromium 側の `image->scale` を `drag_image.width / drag_obj_rect_in_dip.width`（pixel/DIP）へ変更した。
  - `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`

これにより、macOS 側の `pixel_width / scale` は正しく DIP/point 相当のサイズを返し、表示が適正化される。

## 仕様上の取り決め（再発防止）

- `CbfDragImage.scale` は「pixel-per-DIP（device scale factor 相当）」として扱う。
- 送信側（Chromium）は `pixel / dip` を設定する。
- 受信側（host/macOS）は `display_size = pixel / scale` を使う。

## 補足

- この問題はドラッグ画像サイズの単位不整合であり、drop 未反映問題とは独立である。
- `scale <= 0` の場合は 1.0 fallback を維持し、異常値によるクラッシュや無限拡大を防ぐ。
