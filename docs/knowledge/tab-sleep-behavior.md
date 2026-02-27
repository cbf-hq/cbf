# Chromiumにおけるタブのスリープ（破棄/Discarding）の挙動

Chromiumには、メモリ使用量とCPU消費を削減するために、バックグラウンドの非アクティブなタブを「スリープ」させる機能があります。これは主に「メモリセーバー」や「タブの破棄（Tab Discarding）」として知られています。

## スリープのメカニズム

Chromiumの内部実装では、タブのスリープに関して2つの主要なモードが存在します。これは `kWebContentsDiscard` というFeatureフラグの状態によって挙動が分岐します。

### 1. `kWebContentsDiscard` 無効時 (PC版Chromiumのデフォルト挙動)

*   **何がスリープされるか:** `content::WebContents` オブジェクトそのものが破棄されます。
*   **詳細:**
    *   `TabLifecycleUnit::Discard` メソッド内で、現在の `WebContents` (例えば `old_contents`) が完全に削除されます。
    *   代わりに、`content::WebContents::Create` を使って、レンダラープロセスを持たない空っぽの新しい `WebContents` が作成されます (`null_contents` と命名されています)。
    *   この新しい `null_contents` に、元の `old_contents` のナビゲーション履歴などの状態がコピーされます。
    *   `TabStripModel::DiscardWebContentsAt` によって、タブストリップ内の `WebContents` が新しい `null_contents` に置き換えられます。
    *   つまり、**スリープが起こると、タブに関連付けられていた `WebContents` オブジェクトは存在しなくなり、新しい `WebContents` にIDやポインタが置き換わる**ことになります。

### 2. `kWebContentsDiscard` 有効時 (Android版Chromiumのデフォルト挙動 / Featureフラグで有効化可能)

*   **何がスリープされるか:** `WebContents` オブジェクト自体は維持されますが、その内部状態、特にレンダラープロセスや関連するメモリリソースが解放されます。
*   **詳細:**
    *   `TabLifecycleUnit::Discard` メソッド内で、`web_contents()->Discard(...)` が直接呼び出されます。
    *   このモードでは `WebContents` オブジェクトのインスタンス自体は保持されるため、IDやポインタは変わりません。
    *   しかし、その `WebContents` に関連付けられたレンダラープロセスは終了し、ページはレンダリングされていない「空の状態」になります。

## Atelier Browserへの影響と推奨

Atelier Browserのように、Chromiumを抽象化して利用する場合、このタブのスリープ挙動は非常に重要です。

*   **IPC接続の維持:** `kWebContentsDiscard` が無効なデフォルト挙動（`WebContents` が置き換わる）の場合、スリープが発生すると、Studio側で保持している `Page` オブジェクトとChromium側の `WebContents` とのIPC接続が切れてしまいます。再活性化された際には、新しい `WebContents` オブジェクトとの接続を再確立する必要があります。
*   **Rust側設計への影響:**
    *   `Tab` を永続的なUIコンテナ、`Page` を一時的なWebContentsの実体として設計するアプローチが推奨されます。
    *   `Page` はChromiumから「Discardされた」通知を受け取ったら、自身の持つWebContentsへの参照を破棄し、`Tab` は `Page` が `None` の状態になることを許容するべきです。
    *   ユーザーがタブを再度アクティブにした際、`Tab` はChromiumに対しページの再ロードを要求し、その結果として新しい（または再活性化された）`Page` オブジェクトを構築します。

### 推奨事項

**Atelier Browserでは、可能な限り `kWebContentsDiscard` Featureフラグを有効にしてChromiumをビルドすることを強くお勧めします。**

*   `kWebContentsDiscard` を有効にすることで、`WebContents` オブジェクトのIDやポインタがスリープ後も変わらないため、Studio側の `Page` とChromium側の `WebContents` の紐付けが切れにくくなります。
*   これにより、Rust側での `Page` のライフサイクル管理が簡素化され、「中身が空」かどうかだけをチェックすれば良くなります。
*   ただし、いずれのモードであっても、レンダラープロセスは停止するため、ページとのIPC通信は切断される（あるいは応答がなくなる）ことを前提に設計する必要があります。

この知識ベースは、`crates/cbf` でChromiumとのIPCを設計する際の重要な考慮事項となります。
