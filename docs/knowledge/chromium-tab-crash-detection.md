# Chromiumにおけるタブのクラッシュ検出

Atelier Browserにおいて、Chromiumのタブ（レンダラープロセス）がクラッシュしたことを検出・表示するための調査結果をまとめます。

## 検出方法

Chromiumの `content::WebContentsObserver` クラスを継承し、以下の仮想メソッドをオーバーライドすることで、プロセスの消失を検出できます。

```cpp
virtual void PrimaryMainFrameRenderProcessGone(base::TerminationStatus status);
```

### 終了ステータスの判定

引数の `base::TerminationStatus` を確認することで、クラッシュかどうかを判断できます（`base/process/kill.h` で定義）。

- `TERMINATION_STATUS_PROCESS_CRASHED`: プロセスがクラッシュした。
- `TERMINATION_STATUS_ABNORMAL_TERMINATION`: 異常終了。
- `TERMINATION_STATUS_OOM`: メモリ不足による終了。
- `TERMINATION_STATUS_PROCESS_WAS_KILLED`: タスクマネージャー等で強制終了された。

## 実装方針

現在のプロジェクト構造において、この機能を実装するには以下の3つのレイヤーでの変更が必要です。

### 1. Mojom インターフェースの拡張
`chromium/src/chrome/browser/cbf/mojom/cbf_browser.mojom` の `CbfProfileObserver` に、ブラウザプロセスからUIプロセスへ通知するためのメソッドを追加します。

```mojom
interface CbfProfileObserver {
  // ...既存のメソッド
  OnRenderProcessGone(uint64 web_page_id, bool crashed);
};
```

### 2. Chromium側の実装 (C++)
`CbfTabManager` または `CbfProfileService` 内で `WebContentsObserver` を実装し、イベントをフックします。

- `PrimaryMainFrameRenderProcessGone` が呼ばれた際、`cbf::mojom::CbfProfileObserver::OnRenderProcessGone` を通じて通知を送ります。
- `status` が `TERMINATION_STATUS_PROCESS_CRASHED` 等であれば `crashed = true` とします。

### 3. IPCブリッジとRust側の対応
- **C++ Bridge**: `cbf_bridge.cc` にて Mojom からのイベントを受け取り、FFI経由で `CbfBridgeEvent` としてキューに追加します。
- **Rust (cbf-sys)**: FFI構造体にイベント種別を追加します。
- **Rust (cbf)**: `browser_event.rs` にある既存の `WebPageEvent::RenderProcessGone` に変換して上位レイヤー（UIプロセス）へ送出します。

## 補足
UI側（React）では、このイベントを受け取った際に「このページはクラッシュしました」といったプレースホルダーやリロードボタンを表示する実装が必要になります。
