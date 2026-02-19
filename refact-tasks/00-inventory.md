# フェーズ0 棚卸し: API Inventory (cbf + cbf-sys/bridge)

## 目的

- `cbf` / `cbf-chrome` / `cbf-chrome-sys` への分割に向けて、現行APIを列挙する。
- 各APIについて、`cbf名` と `Chromium実装名` の対応を記録する。
- 分類タグ（`cbf-keep` / `cbf-chrome-keep` / `split-rebuild` / `sys-to-chrome-sys`）を初期付与する。

## スコープ

- 含む:
  - `crates/cbf/src` の browser関連公開API
  - `crates/cbf-sys/src/ffi.rs` の公開FFI型/定数
  - `chromium/src/chrome/browser/cbf/*` / `mojom/cbf_browser.mojom` の対応実装
- 除外（このファイルでは詳細化しない）:
  - middlewareの実装詳細
  - product向け補助ロジック

## 分類タグ定義

- `cbf-keep`: `cbf` にそのまま残す
- `cbf-chrome-keep`: `cbf-chrome` へそのまま移動
- `split-rebuild`: 分解して `cbf` と `cbf-chrome` へ再配置
- `sys-to-chrome-sys`: `cbf-sys` から `cbf-chrome-sys` へ移管

## A. モジュール単位インデックス

| ID | 現在のAPI/モジュール | 主用途 | 初期分類 | 補足 |
|---|---|---|---|---|
| A-01 | `crates/cbf/src/browser.rs` (`Backend`, `BrowserHandle`, `connect`) | generic command/event入口 | `split-rebuild` | `RawCommand/RawEvent` と `OpaqueEvent` 設計へ更新対象 |
| A-02 | `crates/cbf/src/command.rs` (`BrowserCommand`) | upstream request語彙 | `split-rebuild` | command自体はgeneric維持、payload型の再分割あり |
| A-03 | `crates/cbf/src/event.rs` (`BrowserEvent`, `WebPageEvent`) | backend facts語彙 | `split-rebuild` | event自体はgeneric維持、payload型の再分割あり |
| A-04 | `crates/cbf/src/data/ids.rs` (`WebPageId`) | stable logical ID | `cbf-keep` | Chromium側 `WebPageId` と一致、抽象として有効 |
| A-05 | `crates/cbf/src/data/profile.rs` (`ProfileInfo`) | profile metadata | `split-rebuild` | `profile_path` など実装依存度の確認が必要 |
| A-06 | `crates/cbf/src/data/surface.rs` (`SurfaceHandle`) | rendering surface | `split-rebuild` | 現状 `MacCaContextId` のみでplatform/chrome依存が強い |
| A-07 | `crates/cbf/src/data/key.rs` | keyboard payload | `split-rebuild` | `windows_key_code` 等がChromium寄り |
| A-08 | `crates/cbf/src/data/mouse.rs` | mouse payload | `split-rebuild` | wheel phase等がChromium寄り |
| A-09 | `crates/cbf/src/data/ime.rs` | IME payload | `split-rebuild` | `ImeTextSpan` 細粒度属性が Chromium/UI依存 |
| A-10 | `crates/cbf/src/data/drag.rs` | drag payload | `split-rebuild` | 現状は一部のみ露出、拡張設計要 |
| A-11 | `crates/cbf/src/data/context_menu.rs` | context menu payload | `split-rebuild` | command ID群がChromium由来 |
| A-12 | `crates/cbf/src/ffi/mod.rs` (`IpcClient`, `IpcEvent`) | bridge adapter | `cbf-chrome-keep` | `cbf` から分離し `cbf-chrome` 側へ |
| A-13 | `crates/cbf/src/chromium_backend.rs` | Chromium backend impl | `cbf-chrome-keep` | crate丸ごと `cbf-chrome` へ |
| A-14 | `crates/cbf/src/chromium_process.rs` | Chromium launch helper | `cbf-chrome-keep` | `start_chromium` は chrome拡張入口 |
| A-15 | `crates/cbf-sys/src/ffi.rs` (`Cbf*` ABI) | C ABI contract | `sys-to-chrome-sys` | `cbf-chrome-sys` に移し Chromium語彙化 |
| A-16 | `crates/cbf-sys/src/modifiers.rs` | modifier bit constants | `sys-to-chrome-sys` | key/mouse raw層として移管 |

## B. フィールド粒度詳細（優先領域）

### 境界方針（確定）: BrowserEvent / ChromeEvent

- `ChromeEvent` を source of truth とする（bridge/Chromium由来情報を欠落なく保持）。
- `BrowserEvent` は `ChromeEvent` の browser-generic への射影結果（lossy projection）とする。
- 変換契約は `to_browser_event(&ChromeEvent) -> Option<BrowserEvent>` を基本とする。
- `None` は「browser-generic 語彙へ安全に射影できない」ことを意味する。
- Chromium固有語彙・内部都合（bitmask詳細、内部フラグ、実装依存属性）は `BrowserEvent` へ持ち込まない。
- 受信経路は単一路線を維持し、必要時のみ拡張側で raw event（`ChromeEvent`）へアクセスする。

### B-1. `WebPageId` / WebContents 対応

| ID | `cbf`現在名 | `cbf-chrome`候補名 | `cbf-chrome-sys`/bridge名 | Chromium元名 | 参照 | 初期分類 | 方針 |
|---|---|---|---|---|---|---|---|
| B-01 | `WebPageId` | `WebPageId` | `uint64 web_page_id` | `using WebPageId = std::uint64_t` + `content::WebContents` 対応 | `chromium/src/chrome/browser/cbf/cbf_tab_manager.h` | `cbf-keep` | 論理IDは維持。実体(`WebContents`)は公開しない |
| B-02 | `WebPageEvent::*` の `web_page_id` | 同名維持 | `CbfBridgeEvent.web_page_id` | `tab_manager_.GetWebPageIdForWebContents(...)` | `chromium/src/chrome/browser/cbf/cbf_profile_service.cc` | `cbf-keep` | ID+re-resolve原則と整合 |

### B-2. Key (`crates/cbf/src/data/key.rs`)

| ID | `cbf`現在名 | bridge名 | Chromium元名 | 参照 | 初期分類 | 命名方針/メモ |
|---|---|---|---|---|---|---|
| B-10 | `KeyEventType::{RawKeyDown,KeyDown,KeyUp,Char}` | `CbfKeyEvent.type_` | `blink::WebInputEvent::Type` | `cbf_profile_service.cc` (ToWebInputEventType) | `split-rebuild` | generic enumは維持候補 |
| B-11 | `KeyEvent.modifiers` | `CbfKeyEvent.modifiers` | `WebInputEvent` modifiers / ui flags | `cbf_profile_service.cc` (SendKeyEvent) | `split-rebuild` | genericに残すが raw flags扱い方を明記 |
| B-12 | `KeyEvent.windows_key_code` | `CbfKeyEvent.windows_key_code` | `input::NativeWebKeyboardEvent.windows_key_code` | `cbf_profile_service.cc` (SendKeyEvent) | `cbf-chrome-keep` | Chromium raw側へ寄せる |
| B-13 | `KeyEvent.native_key_code` | `CbfKeyEvent.native_key_code` | `input::NativeWebKeyboardEvent.native_key_code` | 同上 | `cbf-chrome-keep` | Chromium raw側へ |
| B-14 | `KeyEvent.dom_code` / `dom_key` | 同名 | `ui::KeycodeConverter` 経由 DomCode/DomKey | 同上 | `split-rebuild` | genericでは抽象化、rawは文字列維持 |
| B-15 | `KeyEvent.is_system_key` / `location` | 同名 | `NativeWebKeyboardEvent` field | 同上 | `cbf-chrome-keep` | raw側中心 |

### B-3. Mouse (`crates/cbf/src/data/mouse.rs`)

| ID | `cbf`現在名 | bridge名 | Chromium元名 | 参照 | 初期分類 | 命名方針/メモ |
|---|---|---|---|---|---|---|
| B-20 | `MouseEventType` | `CbfMouseEvent.type_` | `blink::WebInputEvent::Type` | `cbf_profile_service.cc` | `split-rebuild` | generic候補 |
| B-21 | `MouseButton` | `CbfMouseEvent.button` | `blink::WebMouseEvent::Button` | 同上 | `split-rebuild` | generic候補 |
| B-22 | `PointerType` | `CbfMouseEvent.pointer_type` | `blink::WebPointerProperties::PointerType` | 同上 | `split-rebuild` | generic候補だがraw準拠要 |
| B-23 | `MouseWheelEvent.phase` / `momentum_phase` | 同名 | `blink::WebMouseWheelEvent::Phase` | `SendMouseWheelEvent` | `cbf-chrome-keep` | Chromium特有。`cbf-chrome`へ |
| B-24 | `MouseWheelEvent.delta_units` | `CbfMouseWheelEvent.delta_units` | `ui::ScrollGranularity` | `ToScrollGranularity` | `split-rebuild` | enum抽象化の余地あり |

### B-4. IME (`crates/cbf/src/data/ime.rs`)

| ID | `cbf`現在名 | bridge名 | Chromium元名 | 参照 | 初期分類 | 命名方針/メモ |
|---|---|---|---|---|---|---|
| B-30 | `ImeTextSpanType` | `CbfImeTextSpan.type_` | `ui::ImeTextSpan::Type` | `ToImeTextSpanType` | `split-rebuild` | generic subset化を検討 |
| B-31 | `ImeTextSpanThickness` | `CbfImeTextSpan.thickness` | `ui::ImeTextSpan::Thickness` | `ToImeTextSpanThickness` | `cbf-chrome-keep` | 見た目属性はchrome寄り |
| B-32 | `ImeTextSpanUnderlineStyle` | `CbfImeTextSpan.underline_style` | `ui::ImeTextSpan::UnderlineStyle` | `ToImeTextSpanUnderlineStyle` | `cbf-chrome-keep` | chrome寄り |
| B-33 | `ImeTextSpan.*color` / flags | 同名 | `ui::ImeTextSpan` details | `ToImeTextSpans` | `cbf-chrome-keep` | 詳細属性は拡張層 |
| B-34 | `ImeComposition` / `ImeCommitText` | `CbfImeComposition` / `CbfImeCommitText` | `RenderWidgetHostImpl::ImeSetComposition/ImeCommitText` | `SetComposition` / `CommitText` | `split-rebuild` | generic core + chrome span拡張で再構築 |
| B-35 | `ImeBoundsUpdate` | `CbfImeBoundsUpdate` | `TextInputManager` 由来 bounds | `NotifyImeBoundsUpdated` 系 | `split-rebuild` | generic候補 |

### B-5. Drag (`crates/cbf/src/data/drag.rs`)

| ID | `cbf`現在名 | bridge名 | Chromium元名 | 参照 | 初期分類 | 命名方針/メモ |
|---|---|---|---|---|---|---|
| B-40 | `DragData{text,html,html_base_url,url_infos}` | `CbfDragData` | `content::DropData` subset | `NotifyDragStartRequested` | `split-rebuild` | generic coreとして維持可 |
| B-41 | `DragUrlInfo` | `CbfDragUrlInfo` | `content::DropData::UrlInfo` | 同上 | `split-rebuild` | generic候補 |
| B-42 | `DragImage` | `CbfDragImage` | `SkBitmap` -> PNG 変換データ | 同上 | `split-rebuild` | image payloadは要方針 |
| B-43 | `DragStartRequest.allowed_operations` | 同名 | `blink::DragOperationsMask` | 同上 | `cbf-chrome-keep` | raw bitmask |
| B-44 | `DragUpdate/DragDrop.position_*` | 同名 | `gfx::PointF` / DnD routing coords | `SendDragUpdate/Drop` | `split-rebuild` | generic座標語彙化可能 |
| B-45 | 未露出: `filenames`, `file_mime_types`, `custom_data` 等 | なし（未実装） | `content::DropData` fields | `content/public/common/drop_data.h` | `split-rebuild` | `drag-data-field-mapping.md` に沿って拡張 |

### B-6. Surface / Context Menu

| ID | `cbf`現在名 | bridge名 | Chromium元名 | 参照 | 初期分類 | 命名方針/メモ |
|---|---|---|---|---|---|---|
| B-50 | `SurfaceHandle::MacCaContextId` | `ChromeSurfaceHandle::MacCaContextId` (候補) | `CbfSurfaceHandle{kind,ca_context_id}` | CAContext / `WebContents` surface bridge | `cbf_surface_provider_mac.mm` | `cbf-chrome-keep` | `cbf` から除外し `cbf-chrome` のみで提供 |
| B-51 | `ContextMenuItemType` | `CbfMenuItemType` | `ui::MenuModel::ItemType` | `ToCbfMenuItemType` | `split-rebuild` | generic menu core化可 |
| B-52 | `ContextMenu` + command ID const | `CbfContextMenu` | `RenderViewContextMenu` command IDs | `cbf_profile_service.cc` | `cbf-chrome-keep` | command ID constはchrome拡張層へ |

## C. `cbf-sys -> cbf-chrome-sys` 移管対象（初期）

| ID | 現在 (`cbf-sys`) | 移管先 | 理由 |
|---|---|---|---|
| C-01 | `ffi.rs` の `Cbf*` struct群 | `cbf-chrome-sys` | Chromium/Mojo語彙を直接保持する unsafe 層 |
| C-02 | `ffi.rs` の `CBF_*` enum-like const | `cbf-chrome-sys` | raw protocol定数 |
| C-03 | `ffi.rs` の `cbf_bridge_client_*` extern fn | `cbf-chrome-sys` | bridge ABI境界 |
| C-04 | `modifiers.rs` | `cbf-chrome-sys` | input raw bit flags |

## D. 確定方針（フェーズ0決定）

この節は、他エージェントを含む実装時の拘束ルールとして扱う。

1. BrowserEvent 境界:
`cbf` は browser-generic 事実のみを公開し、Chromium固有詳細は `cbf-chrome` 側イベントへ置く。

2. SurfaceHandle:
`SurfaceHandle` は `cbf` から除外し、`cbf-chrome` 側のみで提供する。
`WebPageEvent::SurfaceHandleUpdated` 相当も `cbf` ではなく `cbf-chrome` 拡張イベントで扱う。

3. ProfileInfo:
`profile_id` / `display_name` は `cbf` 残置候補、`profile_path` は `cbf-chrome` 側へ寄せる。
互換期間が必要な場合は `cbf` 側で deprecate を行い段階移行する。

4. Context Menu command IDs:
`CMD_*` 定数群は Chromium由来のため `cbf-chrome` へ移す。
`cbf` は generic payload を中心に保つ。

5. Drag 拡張:
`docs/knowledge/drag-data-field-mapping.md` の分類を正として適用する。
browser-genericに意味が固定できる項目のみ `cbf` へ追加し、内部依存項目は `cbf-chrome` / `cbf-chrome-sys` に限定する。

6. 判断迷い時のデフォルト:
迷った場合は `cbf` に入れず `cbf-chrome` 側へ寄せる。
この方針を覆す提案は、先に ADR と `refact-tasks/strategy.md` 更新を必須とする。

7. `BackendReady`:
`BrowserEvent::BackendReady` から `backend_name` は削除する。
backend識別情報が必要な場合は `cbf-chrome` 拡張イベント/メタ情報で扱う。

8. Permission応答:
`WebPageEvent::PermissionRequested` の `response_channel` は廃止し、
`request_id` 相関で `BrowserCommand::ConfirmPermission` により応答する。

9. 命名（WebPage系）:
`cbf` の公開語彙では `WebPage` を廃止し `BrowsingContext` を採用する。
`cbf-chrome` / `cbf-chrome-sys` では Chromium 実名に合わせ `WebContents` を採用する。
`WebPageId` は `BrowsingContextId` へ改名する。

## D-1. BrowserEvent / WebPageEvent 扱い表（叩き台）

この表はフェーズ1実装前の初期案。最終確定は `01-backend-and-transport-split.md` で行う。

凡例:

- `cbf-keep`: `cbf` の `BrowserEvent`/`WebPageEvent` に残す
- `chrome-only`: `cbf` から外し `cbf-chrome` の拡張イベントのみで扱う
- `split`: generic core と chrome detail に分割

### BrowserEvent

| Variant | 初期扱い | 理由/メモ |
|---|---|---|
| `BackendReady` | `cbf-keep` | `backend_name` は廃止。ready事実のみを保持 |
| `BackendStopped { reason }` | `cbf-keep` | 失敗モデルとしてgenericに必要 |
| `BackendError { info, terminal_hint }` | `cbf-keep` | 失敗モデルとしてgenericに必要 |
| `WebPage { profile_id, web_page_id, event }` | `split` | コンテナは維持、`profile_id`/`event` の一部を分割対象 |
| `ProfilesListed { profiles }` | `split` | `ProfileInfo.profile_path` の分離方針に追従 |
| `ShutdownBlocked { request_id, dirty_web_page_ids }` | `cbf-keep` | lifecycle制御としてgeneric |
| `ShutdownProceeding { request_id }` | `cbf-keep` | lifecycle制御としてgeneric |
| `ShutdownCancelled { request_id }` | `cbf-keep` | lifecycle制御としてgeneric |

### WebPageEvent

| Variant | 初期扱い | 理由/メモ |
|---|---|---|
| `Created { request_id }` | `cbf-keep` | generic lifecycle |
| `NavigationStateChanged { ... }` | `cbf-keep` | generic navigation state |
| `TitleUpdated { title }` | `cbf-keep` | generic page metadata |
| `FaviconUrlUpdated { url }` | `cbf-keep` | generic page metadata |
| `UpdateTargetUrl { url }` | `cbf-keep` | generic UI signal |
| `CursorChanged { cursor_type }` | `split` | cursor enumは実装差が出やすく、欠落時は拡張へ |
| `FullscreenToggled { is_fullscreen }` | `cbf-keep` | generic window state |
| `NewWebPageRequested { target_url, is_popup }` | `split` | `is_popup` の意味は実装依存成分あり |
| `CloseRequested` | `cbf-keep` | generic lifecycle |
| `Closed` | `cbf-keep` | generic lifecycle |
| `SurfaceHandleUpdated { handle }` | `chrome-only` | `SurfaceHandle` は `cbf` から除外方針 |
| `ImeBoundsUpdated { update }` | `split` | coreはgeneric、細部はchrome拡張候補 |
| `ContextMenuRequested { menu }` | `split` | menu payloadは残しつつ command IDは拡張へ |
| `JavaScriptDialogRequested { ... }` | `cbf-keep` | `BrowserCommand::ConfirmBeforeUnload` 等と対になる |
| `PermissionRequested { ... }` | `cbf-keep` | `request_id` 相関で `BrowserCommand::ConfirmPermission` を返す方式へ統一 |
| `RenderProcessGone { crashed }` | `split` | genericには `BackendError/Stopped` で十分な可能性。詳細は拡張候補 |
| `AudioStateChanged { is_audible }` | `cbf-keep` | generic media signal |
| `DomHtmlRead { request_id, html }` | `cbf-keep` | generic command結果 |
| `DragStartRequested { request }` | `split` | drag payload再編方針に追従 |
| `SelectionChanged { text }` | `cbf-keep` | generic text signal |
| `ScrollPositionChanged { x, y }` | `cbf-keep` | generic viewport signal |

## E. フェーズ1への入力

この棚卸し結果を基に、次のタスクを作成する。

1. `refact-tasks/01-backend-and-transport-split.md`
2. `refact-tasks/02-input-key-mouse.md`
3. `refact-tasks/03-ime-split.md`
4. `refact-tasks/04-drag-split.md`
5. `refact-tasks/05-surface-context-menu-split.md`
6. `refact-tasks/06-sys-to-chrome-sys-migration.md`
7. `refact-tasks/07-webpage-to-browsing-context-rename.md`

追記（優先反映）:

- `crates/cbf/src/event.rs`:
  - `BackendReady { backend_name: String }` -> `BackendReady`
  - `PermissionRequested` から `response_channel` を削除
- `crates/cbf/src/command.rs`:
  - `BrowserCommand::ConfirmPermission { web_page_id, request_id, allow }` を追加
- naming:
  - `cbf`: `WebPage*` -> `BrowsingContext*`
  - `cbf-chrome` / `cbf-chrome-sys`: `WebContents*` ベースへ統一
