# 右クリック「新しいタブで開く / 新しいウィンドウで開く」処理フロー

Atelier Browser は Chromium を別プロセスで動かしているため、右クリックメニューのコマンド実行は **Chromium -> CBF -> Rust(App)** の往復が必要になる。ここでは、リンク上の右クリックから「新しいタブで開く」「新しいウィンドウで開く」までの経路を整理する。

## 目的

- 右クリックの「新しいタブ/ウィンドウで開く」がどこで実行され、どこでイベント化されるかを明確にする
- 不発時の原因切り分けポイントを示す

## 全体の流れ（概要）

1. Chromium がコンテキストメニューを生成して CBF に通知
2. CBF -> Rust でメニュー表示
3. UI でコマンド選択（「新しいタブ/ウィンドウ」）
4. Rust -> CBF -> Chromium に「メニューコマンド実行」を送信
5. Chromium が OpenURL を実行し、WebContentsDelegate::OpenURLFromTab が呼ばれる
6. CBF が NewWebPageRequested を IPC で Rust に通知
7. Rust が新規タブ or 新規ワークベンチを作成

## 詳細フロー

### 1) コンテキストメニューの表示

- Chromium: `CbfWebContentsDelegate::HandleContextMenu` が呼ばれる
- Chromium: `CbfProfileService::HandleContextMenu` で `RenderViewContextMenu` を構築
- Chromium -> CBF (mojo): `OnContextMenuRequested` を送る
- CBF -> Rust: `WebPageEvent::ContextMenuRequested` へ変換
- Rust: `BrowserManager::handle_context_menu_requested` -> `Workbench::show_context_menu`

### 2) コマンド選択（UI）

- macOS: UI で選択すると `UserEvent::MacContextMenuCommand` が発火
- Rust: `AtelierApp::note_context_menu_disposition` が
  - `CMD_CONTENT_OPEN_LINK_NEW_TAB` / `CMD_CONTENT_OPEN_LINK_NEW_WINDOW` を検出
  - `pending_new_window_dispositions` に **現在の web_page_id と処分(新規タブ/新規ワークベンチ)** を記録
- Rust: `BrowserManager::handle_mac_context_menu_command` が
  - CBF に `ExecuteContextMenuCommand(menu_id, command_id, event_flags)` を送る

### 3) Chromium 側でのコマンド実行

- Chromium: `CbfProfileService::ExecuteContextMenuCommand` が
  - `RenderViewContextMenu::ExecuteCommand` を実行
- Chromium: `RenderViewContextMenu::ExecuteCommand` が
  - `IDC_CONTENT_CONTEXT_OPENLINKNEWTAB` / `IDC_CONTENT_CONTEXT_OPENLINKNEWWINDOW` を処理
  - `source_web_contents_->OpenURL(...)` を呼ぶ
- Chromium: `WebContentsImpl::OpenURL` が
  - `WebContentsDelegate::OpenURLFromTab` を呼ぶ

### 4) NewWebPageRequested の発火

- Chromium: `CbfWebContentsDelegate::OpenURLFromTab` が
  - `WindowOpenDisposition::NEW_*` を検知
  - `CbfProfileService::NotifyNewWebPageRequested` を呼ぶ
- Chromium -> CBF (mojo): `OnNewWebPageRequested(web_page_id, target_url, is_popup)`
- CBF -> Rust: `WebPageEvent::NewWebPageRequested { target_url, is_popup }`
- Rust: `AtelierApp::handle_new_window_request` が
  - `pending_new_window_dispositions` を参照し
  - 新規タブ or 新規ワークベンチを生成

## 実装ポイント

- **重要**: `CbfWebContentsDelegate::OpenURLFromTab` の実装が無いと、
  「新しいタブで開く」は `NotifyNewWebPageRequested` まで到達しない。
- `AddNewContents` は `window.open` 等で使われるが、
  右クリックの「新しいタブで開く」は `OpenURLFromTab` 経由。

## 切り分けポイント

1. **メニュー表示の確認**
   - `WebPageEvent::ContextMenuRequested` が来ているか
2. **コマンド実行の確認**
   - `CbfProfileService::ExecuteContextMenuCommand` が呼ばれているか
3. **OpenURLFromTab の到達確認**
   - `CbfWebContentsDelegate::OpenURLFromTab` にログを入れる
4. **NewWebPageRequested の通知確認**
   - `CbfProfileService::NotifyNewWebPageRequested` のログ

## 関連ファイル

- `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`
- `chromium/src/chrome/browser/cbf/cbf_web_contents_delegate.cc`
- `chromium/src/chrome/browser/renderer_context_menu/render_view_context_menu.cc`
- `crates/atelier-app/src/app.rs`
- `crates/atelier-app/src/browser/manager.rs`
- `crates/cbf/src/data/context_menu.rs`

