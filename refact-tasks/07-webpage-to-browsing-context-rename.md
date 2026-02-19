# 07: `WebPage*` -> `BrowsingContext*` 命名移行

## 対象ファイル/モジュール

- `crates/cbf/src/data/ids.rs` (`WebPageId`)
- `crates/cbf/src/command.rs` (`*WebPage*` command variants)
- `crates/cbf/src/event.rs` (`WebPageEvent`, `BrowserEvent::WebPage`)
- `crates/cbf/src/browser.rs`（メソッド名）
- `crates/cbf-chrome/src/chromium_backend.rs`（中継名）
- `crates/cbf-chrome/src/ffi/mod.rs`（Chromium固有語彙）
- 関連ドキュメント（ADR/refact-tasks/docs）

## 現状APIと問題点

- `WebPage` は HTMLドキュメント単位の語感を持ちやすく、実際の継続単位（ナビゲーションを跨ぐ文脈）とズレる。

## 移行先

- `cbf`:
  - `WebPage*` 系を `BrowsingContext*` へ改名
  - `WebPageId` は `BrowsingContextId` へ改名
- `cbf-chrome` / `cbf-chrome-sys`:
  - Chromium実名に合わせ `WebContents` ベースを採用

## 必要な再設計内容

1. 型名・variant名・メソッド名の一括rename
2. `cbf` と `cbf-chrome` の語彙境界（BrowsingContext vs WebContents）を文書化
3. 非公開前提のため deprecate なしで直置換

## 受け入れ条件

- `cbf` の公開面に `WebPage` 名称が残っていない。
- `cbf-chrome` 側の公開面は `WebContents` 語彙で統一される。
- ドキュメント上の語彙説明が一貫している。

## 実施結果（2026-02-19）

- `cbf` 公開APIを `WebPage*` から `BrowsingContext*` へリネームした
  - `WebPageId` -> `BrowsingContextId`
  - `WebPageEvent` -> `BrowsingContextEvent`
  - `BrowserEvent::WebPage` -> `BrowserEvent::BrowsingContext`
  - `BrowserCommand` / `BrowserHandle` の `*WebPage*` 系を `*BrowsingContext*` へ統一
- `cbf-chrome` の IPC イベント名を `WebContents*` 語彙へ変更した
  - `WebContentsCreated`
  - `WebContentsClosed`
  - `WebContentsResizeAcknowledged`
  - `WebContentsDomHtmlRead`
  - `NewWebContentsRequested`
- 例: `examples/simpleapp` の API 呼び出しを新命名へ追従した
- ドキュメント語彙を更新した（`README.md`, `docs/architecture.md`, `docs/implementation-guide.md`）
