# Chromium マウスイベント送信経路（別プロセス Renderer）

Atelier Browser のマウス対応に向けて、Chromium 本家の「ブラウザプロセス → Renderer プロセス」へのマウスイベント送信経路を `chromium/src` から確認したメモ。

## 全体の流れ（要約）

1. OS のマウスイベントを `blink::WebMouseEvent` に変換  
2. OOPIF などの別プロセス frame を含む場合は hit-test によって送信先を決定  
3. `RenderWidgetHostImpl` 経由で `InputRouterImpl` に渡す  
4. `InputRouterImpl` が Mojo の `WidgetInputHandler` へ `DispatchEvent` を送信  
5. Renderer 側で `WidgetInputHandler` 実装がイベントを処理

## 主要コードパス

### 1) OSイベント → WebMouseEvent 変換

- `chromium/src/content/browser/renderer_host/render_widget_host_view_event_handler.cc`  
  - `ui::MakeWebMouseEvent` で `ui::MouseEvent` を `blink::WebMouseEvent` に変換  
  - `ShouldRouteEvents()` により `RouteMouseEvent` / `ProcessMouseEvent` を分岐

### 2) OOPIF 含むルーティング（ヒットテスト）

- `chromium/src/components/input/render_widget_host_input_event_router.cc`  
  - `RouteMouseEvent(...)` → `DispatchMouseEvent(...)`  
  - ターゲット決定後 `target->ProcessMouseEvent(...)` へ送る  
  - MouseCapture / MouseEnter/Leave の管理もこの層で行う

### 3) View → RenderWidgetHostImpl

- `chromium/src/content/browser/renderer_host/render_widget_host_view_base.cc`  
  - `ProcessMouseEvent(...)`  
  - `host()->ForwardMouseEventWithLatencyInfo(...)` を呼び出す

### 4) RenderWidgetHostImpl → InputRouterImpl

- `chromium/src/content/browser/renderer_host/render_widget_host_impl.cc`  
  - `ForwardMouseEventWithLatencyInfo(...)`  
  - `input_router()->SendMouseEvent(...)` に流す

### 5) InputRouterImpl → Mojo Dispatch

- `chromium/src/components/input/input_router_impl.cc`  
  - `SendMouseEventImmediately(...)` → `FilterAndSendWebInputEvent(...)`  
  - `client_->GetWidgetInputHandler()->DispatchEvent(...)` を呼ぶ

### 6) Mojo インタフェース定義

- `chromium/src/third_party/blink/public/mojom/input/input_handler.mojom`  
  - `interface WidgetInputHandler { DispatchEvent(...); DispatchNonBlockingEvent(...); }`

### 7) WidgetInputHandler の接続確立

- `chromium/src/components/input/render_input_router.cc`  
  - `RendererWidgetCreated(...)` 内で `GetWidgetInputHandler(...)` を呼び、  
    `widget_input_handler_` を bind する

## Mac の例（Route or Process）

- `chromium/src/content/browser/renderer_host/render_widget_host_view_mac.mm`  
  - `RouteOrProcessMouseEvent(...)` で  
    `GetInputEventRouter()->RouteMouseEvent(...)` か `ProcessMouseEvent(...)` を選択

## 補足: どこで非同期 IPC になるか

- `InputRouterImpl::FilterAndSendWebInputEvent(...)` 内で  
  `WidgetInputHandler::DispatchEvent(...)` を Mojo 経由で送信  
  → ここがブラウザプロセス → Renderer プロセスの非同期 IPC 本体

## 参考観点（Atelier 側のマウス対応の観点）

- 既存のキーイベント経路と同様に、最終的には「Renderer 側の入力ハンドラ」へ  
  Mojo で投げる構造になっている
- OOPIF / hit-test のルーティング層は `RenderWidgetHostInputEventRouter` が担当  
  → マウス移動・ボタン・ホイールごとに target が決まり、`ProcessMouseEvent` が呼ばれる

