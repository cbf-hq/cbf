# CBFにおける `RequestCloseWebPage` 実装調査

`BrowserCommand::RequestCloseWebPage` を実装し、CBF (Chromium Backend Framework) でウェブページを閉じる機能を有効化するための調査結果をまとめます。

## 現状のアーキテクチャと対象範囲

`BrowserCommand::RequestCloseWebPage` は beforeunload を考慮した「丁寧なクローズ」を行う公開 API として扱います。

### 関連ファイル

- **Rust (Client)**
    - `crates/cbf/src/command.rs`: コマンド定義
    - `crates/cbf/src/chromium_backend.rs`: コマンド処理
    - `crates/cbf/src/ffi/mod.rs`: IPC クライアント
    - `crates/cbf-sys/src/ffi.rs`: FFI 定義
- **C++ (Backend)**
    - `chromium/src/chrome/browser/cbf/mojom/cbf_browser.mojom`: Mojo インターフェース
    - `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`: 実装
    - `chromium/src/chrome/browser/cbf/cbf_tab_manager.cc`: タブ管理
    - `chromium/src/chrome/browser/cbf/bridge/cbf_bridge_web_page.cc`: ブリッジ実装

## 実装ステップ

### 1. Mojo インターフェースの更新 (`cbf_browser.mojom`)

`CbfProfileService` インターフェースに `RequestCloseWebPage` と `ConfirmBeforeUnload` を追加します。
beforeunload の通知には `CbfProfileObserver::OnBeforeUnloadDialogRequested` を追加し、reason も同梱します。

```protobuf
// chromium/src/chrome/browser/cbf/mojom/cbf_browser.mojom

interface CbfProfileObserver {
  // ... (既存のイベント)
  OnBeforeUnloadDialogRequested(uint64 web_page_id,
                                uint64 request_id,
                                CbfBeforeUnloadReason reason);
};

interface CbfProfileService {
  // ... (既存のメソッド)
  CreateWebPage(string initial_url, uint64 request_id) => (uint64 web_page_id);
  RequestCloseWebPage(uint64 web_page_id);
  ConfirmBeforeUnload(uint64 request_id, bool proceed);
  // ...
};
```

### 2. C++ バックエンドの実装 (`CbfProfileService`)

`CbfProfileService::RequestCloseWebPage` を実装します。
`CbfTabManager::CloseWebPage` は即時破壊を行うため、`beforeunload` ハンドラ等を考慮する「丁寧なクローズ」を行う場合は `web_contents->ClosePage()` を使用するのが適切です。
このため `CbfTabManager::RequestCloseWebPage` を追加し、`RequestCloseWebPage` と破壊を分離します。

`ClosePage()` が成功すると、`CbfWebContentsDelegate::CloseContents` が呼ばれ、そこで `tab_manager_.CloseWebContents` (破壊) が実行されるフローが既に存在します。

```cpp
// chromium/src/chrome/browser/cbf/cbf_profile_service.cc

void CbfProfileService::RequestCloseWebPage(uint64_t web_page_id) {
  content::WebContents* web_contents = tab_manager_.GetWebContents(web_page_id);
  if (web_contents) {
    // ClosePage() は非同期でクローズを試み、beforeunload 等を処理します。
    // クローズが確定すると WebContentsDelegate::CloseContents が呼ばれます。
    beforeunload_reasons_[web_contents] =
        mojom::CbfBeforeUnloadReason::kCloseWebPage;
    tab_manager_.RequestCloseWebPage(web_page_id);
  }
}
```

### 3. C++ ブリッジの実装 (`CbfBridgeClient`)

FFI から Mojo への呼び出しを中継します。`SetWebPageSize` などの既存実装を参考にします。

- **`cbf_bridge_web_page.cc`**:
    - `CbfBridgeClient::RequestCloseTab` / `ConfirmBeforeUnload` を実装 (Mojo 呼び出し)。
    - `extern "C"` 関数 `cbf_bridge_client_request_close_tab` / `cbf_bridge_client_confirm_beforeunload` を追加。

```cpp
// chromium/src/chrome/browser/cbf/bridge/cbf_bridge_web_page.cc

namespace cbf_bridge::internal {

bool CbfBridgeClient::RequestCloseTab(const uint64_t tab_id) {
    // ... impl_->browser_remote チェック ...
    // ... tab_id から profile を特定し、connection.remote->RequestCloseTab(tab_id) を呼ぶ
    // (実装パターンは SetWebPageSize と同様)
}

bool CbfBridgeClient::ConfirmBeforeUnload(const uint64_t tab_id,
                                          const uint64_t request_id,
                                          const bool proceed) {
    // ... tab_id から profile を特定し、connection.remote->ConfirmBeforeUnload(request_id, proceed) を呼ぶ
}

} // namespace

extern "C" bool cbf_bridge_client_request_close_tab(CbfBridgeClientHandle* client, uint64_t tab_id) {
  if (!client) return false;
  return reinterpret_cast<cbf_bridge::internal::CbfBridgeClient*>(client)
      ->RequestCloseTab(tab_id);
}

extern "C" bool cbf_bridge_client_confirm_beforeunload(
    CbfBridgeClientHandle* client,
    uint64_t tab_id,
    uint64_t request_id,
    bool proceed) {
  if (!client) return false;
  return reinterpret_cast<cbf_bridge::internal::CbfBridgeClient*>(client)
      ->ConfirmBeforeUnload(tab_id, request_id, proceed);
}
```

### 4. Rust 側の実装 (`cbf-sys`, `cbf`)

- **`crates/cbf-sys/src/ffi.rs`**:
    - `cbf_bridge_client_request_close_tab` / `cbf_bridge_client_confirm_beforeunload` を定義します。

```rust
unsafe extern "C" {
    pub fn cbf_bridge_client_request_close_tab(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
    ) -> bool;

    pub fn cbf_bridge_client_confirm_beforeunload(
        client: *mut CbfBridgeClientHandle,
        tab_id: u64,
        request_id: u64,
        proceed: bool,
    ) -> bool;
}
```

- **`crates/cbf/src/ffi/mod.rs`**:
    - `IpcClient::request_close_tab` / `confirm_beforeunload` メソッドを追加し、FFI 関数を呼び出します。

- **`crates/cbf/src/chromium_backend.rs`**:
    - `BrowserCommand::RequestCloseWebPage` のハンドリングを実装し、`client.request_close_tab(web_page_id)` を呼び出します。
    - `BrowserCommand::ConfirmBeforeUnload` のハンドリングを追加します。

## 注意点

- **WebPageId の管理**: `CbfBridgeClientImpl` 内で `web_page_profiles` マップを使って `web_page_id` から `profile_id` を逆引きする必要があります (既存実装あり)。
- **非同期性**: `RequestCloseWebPage` は即座にページが消えることを保証しません（`beforeunload` でキャンセルされる可能性があるため）。ページが実際に消えたことは、既存の `WebPageEvent::Destroyed` (もしあれば) や、IPC 切断等で検知する必要がありますが、現状のイベント定義に `WebPageClosed` や `Destroyed` があるか確認が必要です。
    - 現状 `BrowserEvent::WebPageEvent` に `Destroyed` はなさそうです。必要であれば追加を検討してください。ただし、`CloseContents` 経由で破壊された場合、イベントを飛ばす仕組みが必要かもしれません。現状 `CbfProfileService::CloseWebContents` で `browser_service_->NotifyWebPageClosed()` を呼んでいますが、これがどこに繋がっているか確認が必要です (おそらく `App` 側への通知)。

## beforeunload の UI 統合

- beforeunload は `WebPageEvent::JavaScriptDialogRequested`（`DialogType::BeforeUnload`）として通知する
- `BeforeUnloadReason`（`CloseWebPage` / `Navigate` / `Reload` / `WindowClose` / `Unknown`）を同梱する
- UI は `BrowserCommand::ConfirmBeforeUnload` で続行/中止を返す

以上
