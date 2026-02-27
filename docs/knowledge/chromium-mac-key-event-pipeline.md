# Chromium macOS: RenderWidgetHostViewMac から Renderer までのキーイベント経路

このドキュメントは、macOS で発生したキー入力が `RenderWidgetHostViewMac` を経由して Renderer 側に届くまでの実装経路を追った調査メモ。

対象: キーイベント (`NSEvent` -> `WebKeyboardEvent`) の Browser Process -> Renderer Process 連携。

## まとめ (結論だけ先に)

- macOS の `NSEvent` は `input::NativeWebKeyboardEvent` に変換され、`RenderWidgetHostViewMac` -> `RenderWidgetHostImpl` -> `RenderInputRouter` -> `InputRouterImpl` -> Mojo (`WidgetInputHandler::DispatchEvent`) の順で Renderer に送られる。
- `RenderWidgetHostImpl::ForwardKeyboardEventWithCommands` が「編集コマンド」(insertTab など) を **次のキーイベントに紐づけて** `WidgetInputHandler::SetEditCommandsForNextKeyEvent` で送るのが重要なポイント。
- Renderer 側では `WidgetInputHandlerImpl` が受け取り、`WidgetInputHandlerManager` が InputHandlerProxy / MainThreadEventQueue に振り分ける。

## 関係ファイル (入口と出口)

- macOS での `NSEvent` 受信/変換
  - `components/remote_cocoa/app_shim/bridged_content_view.mm`
  - `components/input/native_web_keyboard_event_mac.mm`
  - `components/input/web_input_event_builders_mac.mm`
- Browser Process の中継
  - `content/browser/renderer_host/render_widget_host_view_mac.mm`
  - `content/browser/renderer_host/render_widget_host_impl.cc`
  - `components/input/render_input_router.h`
  - `components/input/input_router_impl.cc`
- Renderer Process 側
  - `third_party/blink/renderer/platform/widget/input/widget_input_handler_impl.cc`
  - `third_party/blink/renderer/platform/widget/input/widget_input_handler_manager.cc`

## Browser Process: macOS -> RenderWidgetHostViewMac

### 1) NSView でのキーイベント受信

macOS では NSView が `keyDown:` / `keyUp:` / `flagsChanged:` を受け取る。Chromium の macOS 実装では `BridgedContentView` がこの入口になる。

- `components/remote_cocoa/app_shim/bridged_content_view.mm`
  - `-keyDown:` で `interpretKeyEvents:` を呼び、`doCommandBySelector:` / `insertText:` の結果を振り分ける。
  - `doCommandBySelector:` で `insertTab` などの **insert 系コマンド**は「キーイベント扱い」に戻す方針になっている。
  - `insertTextInternal:` は Tab を文字入力として扱わない旨のコメントがある。

### 2) NSEvent -> NativeWebKeyboardEvent

`NSEvent` は `WebKeyboardEventBuilder::Build` で `blink::WebKeyboardEvent` に変換され、
`input::NativeWebKeyboardEvent` へ詰め替えられる。

- `components/input/native_web_keyboard_event_mac.mm`
  - `NativeWebKeyboardEvent(gfx::NativeEvent)` が `WebKeyboardEventBuilder::Build` を使用。
- `components/input/web_input_event_builders_mac.mm`
  - `WebKeyboardEventBuilder::Build(NSEvent*)` が `windows_key_code` / `dom_code` / `text` / `unmodified_text` を埋める。
  - Tab の `text` / `unmodified_text` を `\x9` に強制するコードがある。

### 3) RenderWidgetHostViewMac での受け渡し

`RenderWidgetHostViewMac` は `ForwardKeyboardEvent` / `ForwardKeyboardEventWithCommands` で
`RenderWidgetHostImpl` にイベントを渡す。

- `content/browser/renderer_host/render_widget_host_view_mac.mm`
  - `ForwardKeyboardEvent` -> `RenderWidgetHostImpl::ForwardKeyboardEventWithLatencyInfo`
  - `ForwardKeyboardEventWithCommands` -> `RenderWidgetHostImpl::ForwardKeyboardEventWithCommands`

## Browser Process: RenderWidgetHostImpl -> RenderInputRouter

### 4) RenderWidgetHostImpl の前処理

`RenderWidgetHostImpl::ForwardKeyboardEventWithCommands` が、ブラウザ側のショートカット処理や
イベント抑制を行う。

- `content/browser/renderer_host/render_widget_host_impl.cc`
  - `KeyPressListenersHandleEvent` による早期処理
  - `delegate_->PreHandleKeyboardEvent` によるブラウザショートカット判定
  - `GetWidgetInputHandler()->SetEditCommandsForNextKeyEvent(...)` による
    **編集コマンドの紐付け**
  - `input_router()->SendKeyboardEvent(...)` で InputRouter へ送信

### 5) RenderInputRouter / InputRouterImpl

`RenderInputRouter` は InputRouter を保持し、Mojo の `WidgetInputHandler` を取得して
イベント送信を委譲する。

- `components/input/render_input_router.h`
  - `GetWidgetInputHandler()` / `DispatchInputEventWithLatencyInfo(...)`
- `components/input/input_router_impl.cc`
  - `SendKeyboardEvent` -> `FilterAndSendWebInputEvent`
  - `FilterAndSendWebInputEvent` -> `WidgetInputHandler::DispatchEvent(...)` (Mojo)

ここで `DispatchEvent` が Renderer 側に飛ぶ。

## Renderer Process: WidgetInputHandler -> EventHandler

### 6) WidgetInputHandlerImpl が Mojo を受信

`WidgetInputHandlerImpl::DispatchEvent` が Mojo メッセージを受け取り、
`WidgetInputHandlerManager` にイベントを渡す。

- `third_party/blink/renderer/platform/widget/input/widget_input_handler_impl.cc`
  - `DispatchEvent(...)` -> `input_handler_manager_->DispatchEvent(...)`

### 7) WidgetInputHandlerManager での処理

`WidgetInputHandlerManager::DispatchEvent` は InputHandlerProxy / MainThreadEventQueue に
振り分ける。ここから先で Blink のイベント処理へ進む。

- `third_party/blink/renderer/platform/widget/input/widget_input_handler_manager.cc`
  - `DispatchEvent(...)` が入力スレッドの入口
  - 入力抑制や latency 記録を行い、イベントを `InputHandlerProxy` に渡す

## キー入力に関連する設計上のポイント

### EditCommand と KeyEvent の二重経路

`insertTab` のような編集コマンドは `SetEditCommandsForNextKeyEvent` で
**次のキーイベント**に付随して送られる。

- Browser 側: `RenderWidgetHostImpl::ForwardKeyboardEventWithCommands`
- Renderer 側: `WidgetInputHandlerImpl` -> `WidgetInputHandlerManager`

### Tab の扱い

Tab は macOS 側のイベント変換 (`WebKeyboardEventBuilder`) で `text` が `\t` に
強制される。一方で Blink は `InsertTab` / `InsertBacktab` コマンドにより
編集挙動とフォーカス移動を決定する。

- `components/input/web_input_event_builders_mac.mm`
- `third_party/blink/renderer/core/editing/editing_behavior.cc`
- `third_party/blink/renderer/core/editing/commands/insert_commands.cc`

## 補足: Renderer 側の編集コマンド処理

`InsertTab` は Blink 内で `HandleTextInputEvent("\t")` に変換される。
ただし、フォームのフォーカス移動などの挙動は編集対象/フォーカス状態に依存する。

- `third_party/blink/renderer/core/editing/commands/insert_commands.cc`

## TODO / 調査メモ

### app_shim / remote_cocoa に関する補足 (tips)

- `RenderWidgetHostViewMac` は **デフォルトで in-process NSView ブリッジ**を使う。
  - `content/browser/renderer_host/render_widget_host_view_mac.mm` で
    `in_process_ns_view_bridge_` を生成して `ns_view_` にセットする。
- `RenderWidgetHostNSViewBridge` は **同一プロセス / 別プロセスの両方で使える**設計。
  - `content/app_shim_remote_cocoa/render_widget_host_ns_view_bridge.h`
- `WebContentsViewMac` 側で `remote_cocoa_application` がある場合のみ
  `MigrateNSViewBridge` が呼ばれ、**remote NSView（app_shim 側）へ移行**する。
  - `content/browser/web_contents/web_contents_view_mac.mm`

### TODO / 調査メモ

- `SetEditCommandsForNextKeyEvent` と `DispatchEvent` の送信順序は
  コメント上「先に EditCommands を送る必要がある」とされており、
  Tab の二重送信問題の観点で重要。
