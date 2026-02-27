調査結果:
CEF（Chromium）のウィンドウを隠す（`orderOut`等）と`CALayerHost`（の表示内容）が消えてしまう主な原因は、**Chromiumがウィンドウ非表示時にリソース節約のためにCompositor（および描画実体であるCAContext）を意図的に破棄する仕様になっているため**です。

### メカニズムの詳細

1.  **ウィンドウの非表示検知**: ウィンドウが`orderOut`等で隠れると、`RenderWidgetHostViewMac::Hide()`が呼び出されます。
2.  **非表示状態の伝播**: `Hide()`内で`WasOccluded()`が呼ばれ、その中で`BrowserCompositorMac::SetRenderWidgetHostIsHidden(true)`が実行されます。
3.  **Compositorの破棄**: これを受けて`BrowserCompositorMac::UpdateState()`が実行され、状態が`HasNoCompositor`（Compositorなし）に遷移します。
4.  **CAContextの消失**: この遷移処理で`recyclable_compositor_`（およびその配下の`ui::Compositor`）が破棄されます。これにより、GPUプロセス側で保持されていた**CAContextも破棄**されます。
5.  **CALayerHostの無効化**: `CALayerHost`は`CAContext`のID（`contextId`）を参照して描画を行っていますが、参照先の`CAContext`が破棄されるため、別ウィンドウでその`CALayerHost`を表示していても中身が消えてしまいます。

### 該当コード箇所

**`content/browser/renderer_host/render_widget_host_view_mac.mm`**
`Hide()`メソッドが起点となり、`WasOccluded()`を通じてCompositorへ非表示状態を通知しています。

```cpp
void RenderWidgetHostViewMac::Hide() {
  // ...
  WasOccluded(); // ここで非表示を通知
  // ...
}

void RenderWidgetHostViewMac::WasOccluded() {
  // ...
  // BrowserCompositorMacに非表示であることを伝える
  browser_compositor_->SetRenderWidgetHostIsHidden(true);
  // ...
}
```

**`content/browser/renderer_host/browser_compositor_view_mac.mm`**
`UpdateState()`で非表示状態（`render_widget_host_is_hidden_`）を確認し、Compositorを持たない状態へ遷移させます。

```cpp
void BrowserCompositorMac::UpdateState() {
  // ...
  // ホストが非表示なら、Compositorをリサイクル（破棄）する状態へ遷移
  if (!render_widget_host_is_hidden_) {
    TransitionToState(HasOwnCompositor);
    return;
  }
  TransitionToState(HasNoCompositor);
}

void BrowserCompositorMac::TransitionToState(State new_state) {
  // ...
  if (state_ == HasOwnCompositor) {
    // ここでCompositorの実体が破棄される（CAContextも道連れになる）
    recyclable_compositor_.reset();
  }
  // ...
}
```

### 対策の方向性

Chromiumにパッチを当ててこの挙動を回避する場合、`BrowserCompositorMac::UpdateState`メソッド内で、`render_widget_host_is_hidden_`が`true`であっても`TransitionToState(HasNoCompositor)`に遷移しないように変更する（常に`HasOwnCompositor`を維持する）ことが有効と考えられます。これにより、ウィンドウが隠れても`CAContext`が維持されます。

他に詳細な調査や修正案の実装が必要であれば、お知らせください。

**補足**: `CALayerHost`自体が消えるというよりは、参照先の`CAContext`が無効になることで表示が消える（あるいはCALayerHostがツリーから外される）という挙動になります。現象としては「中身が消える」形になります。

```bash
# 参考ファイル
chromium/src/content/browser/renderer_host/render_widget_host_view_mac.mm
chromium/src/content/browser/renderer_host/browser_compositor_view_mac.mm
```
