# Chromium (macOS) における `beginDraggingSession` の処理フロー調査

Chromium の macOS 実装で、リンク/画像/選択テキストのドラッグ開始時に `NSDraggingSession` がどのように開始・終了されるかを調査した結果。

## 概要

- リンクドラッグ時に表示される灰色の横長でないポップアップ（タイトル+URL）は、Blink が生成するドラッグ画像である。
- macOS では Browser Process 側で `beginDraggingSessionWithItems:event:source:` を呼び、ネイティブの source drag session を開始している。
- ドラッグ終了時は `draggingSession:endedAtPoint:operation:` から `EndDrag` 経由で Blink の drag state をクリーンアップする設計であり、この終了通知に依存して整合性を保っている。

## 処理フロー

### 1. Blink がドラッグ画像を生成する

- リンクドラッグ画像の生成:
  - `chromium/src/third_party/blink/renderer/core/page/drag_image.cc:143`
  - 背景色は `SkColorSetRGB(140, 140, 140)`:
    - `chromium/src/third_party/blink/renderer/core/page/drag_image.cc:224`
- リンクドラッグ時の採用箇所:
  - `chromium/src/third_party/blink/renderer/core/page/drag_controller.cc:1319`
  - `DragController::DoSystemDrag` で Browser 側へ開始要求:
    - `chromium/src/third_party/blink/renderer/core/page/drag_controller.cc:1394`
    - `chromium/src/third_party/blink/renderer/core/page/drag_controller.cc:1416`

### 2. Browser Process で drag 開始要求を受ける

- 入口:
  - `chromium/src/content/browser/renderer_host/render_widget_host_impl.cc:2893`
- URL/ファイルパスのフィルタ後、ビューへ委譲:
  - `chromium/src/content/browser/renderer_host/render_widget_host_impl.cc:2984`

### 3. macOS 実装で `NSDraggingSession` を開始する

- `WebContentsViewMac::StartDragging`:
  - `chromium/src/content/browser/web_contents/web_contents_view_mac.mm:202`
  - `remote_ns_view_->StartDrag(...)` / `in_process_ns_view_bridge_->StartDrag(...)`:
    - `chromium/src/content/browser/web_contents/web_contents_view_mac.mm:261`
- Mojo 定義（`StartDrag`）:
  - `chromium/src/content/common/web_contents_ns_view_bridge.mojom:48`
- 実際の開始処理:
  - `chromium/src/content/app_shim_remote_cocoa/web_contents_view_cocoa.mm:278`
  - `beginDraggingSessionWithItems:event:source:` を直接呼ぶ。

### 4. macOS で drag 終了通知を受け、Blink 状態を閉じる

- AppKit の drag 終了コールバック:
  - `chromium/src/content/app_shim_remote_cocoa/web_contents_view_cocoa.mm:291`
  - `_host->EndDrag(...)` を呼ぶ:
    - `chromium/src/content/app_shim_remote_cocoa/web_contents_view_cocoa.mm:307`
- Browser 側終了処理:
  - `chromium/src/content/browser/web_contents/web_contents_view_mac.mm:653`
  - `web_contents_->SystemDragEnded(...)`:
    - `chromium/src/content/browser/web_contents/web_contents_view_mac.mm:671`
  - `web_contents_->DragSourceEndedAt(...)`:
    - `chromium/src/content/browser/web_contents/web_contents_view_mac.mm:690`
- Blink 側の最終クリーンアップ:
  - `RenderWidgetHostImpl::DragSourceSystemDragEnded`:
    - `chromium/src/content/browser/renderer_host/render_widget_host_impl.cc:2076`

## 重要な観察

### A. 「灰色ポップアップ」はステータスバーではなくドラッグ画像

リンクドラッグ画像は Blink 側で明示生成され、macOS の `NSDraggingSession` に渡される。  
したがって表示だけ隠しても、source drag session 自体が開始されていれば drag state の競合は残る。

### B. 整合性の鍵は「開始」より「終了通知の保証」

Chromium の drag state machine は `DragSourceSystemDragEnded` / `DragSourceEndedAt` で閉じる。  
この経路が欠落すると、Renderer 側の drag 状態が残留しうる。

### C. 既存の「開始横取り」パターンが存在する

DevTools InputHandler は `StartDragging` を interception し、OS drag を始めずに Chromium 内部状態のみで drag を進める実装を持つ。

- `chromium/src/content/browser/devtools/protocol/input_handler.cc:1533`
- `chromium/src/content/browser/devtools/protocol/input_handler.cc:956`

この実装は、Atelier/CBF で「Chromium 側 source drag session を起動しない」設計を導入する際の参照になる。

## 結論

`beginDraggingSession` は macOS 側で source drag session を開始する中核であり、ここを回避しない限り Chromium がドラッグ UI/状態管理を握る。  
根本対策は「Chromium の source drag session を開始しないモード」を Browser 層に持ち、代わりにホスト（Atelier）が drag lifecycle を駆動する方式である。

## 追加知見（今回の不具合対応）

### 1. `operation.is_empty()` のみで cancel 判定すると内部 drop を取りこぼす

- `NSDraggingSource` の `draggingSession:endedAtPoint:operation:` において、`operation == None` だけで cancel 扱いすると、WebView 内で完結する drop でも cancel されるケースがある。
- 実運用では `endedAtPoint` の座標を compositor host view ローカルへ変換し、view 内なら drop とみなす補完判定が必要。

### 2. `document_is_handling_drag` を保持しないと `drop` が発火しない

- CBF 経路で `DragTargetDragEnter/DragTargetDragOver` callback の `document_is_handling_drag` を捨てると、`SendDragDrop` 時の `DropData` に反映されない。
- その結果、`DropDataToDragData` で `force_default_action = true` となり、`dragover/dragleave` は動作しても `drop` が発火しないケースが発生する。
- 対策として、セッション状態に `document_is_handling_drag` を保持し、`SendDragDrop` 直前に `session.drop_data.document_is_handling_drag` へ反映する。

### 3. 外部ドロップ成功と Web 内 drop 成功は別条件

- Finder など外部アプリへのドロップが成功していても、Web ページ内 drop 成功を保証しない。
- host-owned D&D の検証では「外部ドロップ可否」と「ページ内 `drop` 発火」を必ず分けて確認する必要がある。
