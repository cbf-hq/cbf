# ブラウザタブ実装戦略：WebContents vs TabModel

独自のブラウザフレームワークを開発するにあたり、「ブラウザのタブ／ウェブページ」をChromiumのどのクラスに対応させるかの調査結果と実装方針をまとめる。

## 1. 識別子の候補と比較

「タブ」の単位として採用すべきクラスの比較。

| 候補 | 定義場所 | 役割と特徴 | メリット | デメリット |
| :--- | :--- | :--- | :--- | :--- |
| **`content::WebContents`** | `content/public/browser/` | **推奨**。ChromiumのContent APIにおけるウェブ表示領域の基本単位。HTML描画、ナビゲーション、JS実行を管理。 | ・自己完結しており、`content/` モジュールのみで利用可能。<br>・汎用的で扱いやすい。 | ・タブ固有のUI状態（ピン留め、グループ等）を持たないため、別途管理が必要。 |
| **`tabs::TabModel`** | `chrome/browser/ui/tabs/` | Chromeブラウザにおける「タブ」のラッパークラス。内部に `WebContents` を所有する。 | ・タブのメタデータ（ピン留め、ブロッキング等）を綺麗に抽象化している。 | ・`chrome/` 層にあり、ブラウザ製品としてのChrome固有ロジックと密結合しているため、汎用フレームワークには不向き。 |
| **`content::Page`** | `content/public/browser/` | `WebContents` 内の「ある時点でのドキュメント」。MPArchにより1つのWebContents内に複数存在しうる。 | ・より細かい粒度の管理が可能。 | ・「タブ」としては粒度が細かすぎる（遷移で変わってしまう）。 |

### 結論と推奨構成

フレームワークの独立性と柔軟性を保つため、**`content::WebContents` を基本単位**とし、それをラップする独自のタブクラスを作成することを推奨する。

**推奨実装イメージ:**

```cpp
class MyBrowserTab {
public:
    int GetId() const;
    content::WebContents* GetWebContents() const;
    
    // タブ固有のUI状態はここに実装
    bool IsPinned() const;
    void SetPinned(bool pinned);
    
    // スリープ機能（後述）
    void Sleep();
    void WakeUp();

private:
    std::unique_ptr<content::WebContents> web_contents_;
};
```

---

## 2. タブのスリープ機能（Discarding）の実現

Chromiumの `TabModel` や `Performance Manager` を使わなくとも、`WebContents` の機能だけでスリープ機能は実装可能である。

### 実装アプローチ

1.  **独自実装 (推奨):**
    *   **スリープ時:** 現在の `WebContents` を破棄し、新しい（または復元用の軽量な）`WebContents` に差し替える。あるいは、メモリを解放するために `WebContents::WasDiscarded()` フラグを活用する。
    *   **復帰時:** ナビゲーションコントローラを通じてリロードを行う。

2.  **API利用:**
    *   **Android:** `WebContents::Discard()` メソッドが利用されている。
    *   **Desktop:** `WebContents::WasDiscarded()` フラグと `SetNeedsReload()` を組み合わせて制御する。

Chromiumの `TabLifecycleUnit` (Chromeブラウザの機能) は高度な自動判定ロジックを持つが、依存関係が複雑なため、自作フレームワークでは「ユーザー操作」や「単純なタイマー」ベースの独自ロジックで `WebContents` を操作する方が現実的である。

---

## 3. その他のタブ機能の実装方法

ブラウザのタブに求められる主要な機能は、多くが `WebContents` のメソッドとして提供されている。UIに関わる一部の機能はDelegate/Observerパターンで実装する。

### `WebContents` のメソッドで直接実現可能な機能

| 機能 | WebContents API | 備考 |
| :--- | :--- | :--- |
| **ナビゲーション** | `GetController().GoBack()`, `Forward()`, `Reload()`, `Stop()` | |
| **タイトル取得** | `GetTitle()` | `std::u16string` を返す |
| **ミュート制御** | `SetAudioMuted(bool)`, `IsAudioMuted()` | `IsCurrentlyAudible()` で音の検知も可能 |
| **ページ内検索** | `Find()`, `StopFinding()` | |
| **ズーム** | `SetPageScale()` | 通常は `HostZoomMap` を使うが、個別の倍率設定も可能 |
| **全画面表示** | `ExitFullscreen()`, `ForSecurityDropFullscreen()` | |
| **ページ保存** | `SavePage()`, `GenerateMHTML()` | HTMLまたはMHTML形式 |
| **クラッシュ判定** | `IsCrashed()` | Sad Tab表示の判定に使用 |

### `WebContentsDelegate` / `Observer` で実装が必要な機能

これらは `WebContents` 自体ではなく、それをホストする側（ブラウザUI）が実装する必要がある。

| 機能 | 実装箇所 | 概要 |
| :--- | :--- | :--- |
| **新規タブ作成** | `WebContentsDelegate::AddNewContents()` | `window.open` 等の対応 |
| **右クリックメニュー** | `WebContentsDelegate::HandleContextMenu()` | コンテキストメニューの表示制御 |
| **JSダイアログ** | `WebContentsDelegate::RunJavaScriptDialog()` | alert, confirm, prompt の表示 |
| **権限リクエスト** | `WebContentsDelegate::CheckMediaAccessPermission()` | カメラ、マイク、位置情報など |
| **ファビコン** | `WebContentsObserver::DidUpdateFaviconURL()` | URL検知後、`DownloadImage()` で取得 |
| **ロード表示** | `WebContentsObserver::DidStartLoading()` | タブのスピナー表示制御 |

### フレームワーク独自に実装が必要な機能

ChromiumのContent API範囲外であり、UI層で完全に自作する必要があるもの。

*   タブのグループ化
*   タブのピン留め
*   ドラッグ＆ドロップによる並べ替え
*   セッションの保存と復元（閉じたタブを開く）

## まとめ

独自のブラウザフレームワークにおいては、**`content::WebContents` をコアコンポーネントとして採用し、不足する「タブUIとしての状態（ピン留め、グループ等）」や「スリープ制御」をラップするクラスで補完する構成**が最も効率的かつ標準的である。
