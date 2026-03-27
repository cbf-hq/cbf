# Chromium (macOS) における IME (Input Method Editor) の処理フロー

Chromium の macOS 版における IME イベント (`NSTextInputClient` のメソッド呼び出し) が、どのように Renderer プロセスへ送信されるかに関する調査結果。

## 概要

キーイベント (`NSEvent` -> `WebKeyboardEvent`) と異なり、IME の操作は **高レベルのテキスト操作コマンド** として抽象化され、専用の Mojo メッセージとして Renderer プロセスへ送信される。これらに `NSEvent` オブジェクトそのものは関与しない。

## 処理フロー詳細

### 1. IME イベントの受信 (Browser Process)

`RenderWidgetHostViewCocoa` (`NSTextInputClient` プロトコル実装) が macOS の Input Manager からメソッド呼び出しを受ける。

#### `insertText:replacementRange:` (テキスト確定)
- **トリガー**: ユーザーが変a換を確定した時、または IME オフ状態で文字を入力した時。
- **処理**:
    - `isHandlingKeyDown` (キー押下処理中) の場合:
        - `_textToBeInserted` にテキストを追加し、即座には送信しない。
        - その後の `keyEvent:` メソッドの終了時に、キーイベント (`Char` イベント) として、あるいは `ImeCommitText` としてまとめて送信される判断が行われる。
    - それ以外 (マウス操作で候補を選択した時など):
        - 即座に `_host->ImeCommitText` を呼び出す。

#### `setMarkedText:selectedRange:replacementRange:` (未確定文字列の更新)
- **トリガー**: 変換中にテキストが変更された時。
- **処理**:
    - `_markedText` (未確定文字列) や `_imeTextSpans` (下線情報など) を更新。
    - `_host->ImeSetComposition` を呼び出す。

### 2. IPC メッセージへの変換 (Browser -> Renderer)

`RenderWidgetHostNSViewHost` (Mojo インターフェース) を経由して、以下のメッセージが送信される。これらは `content/common/render_widget_host_ns_view.mojom` で定義されている。

#### `ImeCommitText`
- **引数**:
    - `text`: `mojo_base.mojom.String16` (確定されたテキスト)
    - `replacement_range`: `gfx.mojom.Range` (置換範囲、通常は現在の選択範囲)
- **意味**: 「現在のカーソル位置（または指定範囲）に、このテキストを挿入・確定せよ」

#### `ImeSetComposition`
- **引数**:
    - `text`: `mojo_base.mojom.String16` (未確定のテキスト)
    - `ime_text_spans`: `array<ui.mojom.ImeTextSpan>` (装飾情報。下線の色、太さ、背景色など)
    - `replacement_range`: `gfx.mojom.Range`
    - `selection_start`: `int32` (未確定文字列内でのカーソル開始位置)
    - `selection_end`: `int32` (未確定文字列内でのカーソル終了位置)
- **意味**: 「現在のカーソル位置に、この未確定文字列を表示し、下線を引け」

### 3. `NSEvent` との関係

- **直接の依存なし**: `ImeCommitText` や `ImeSetComposition` のメッセージには、`NSEvent` の生データ (ネイティブイベント) は含まれない。
- **間接的な関係**: `insertText:` がキー押下によって呼ばれた場合、その処理は `keyEvent:` メソッドのコンテキスト内で行われる。Chromium は「キーイベントによる入力」と「IME による入力」を区別し、適切な順序で Renderer に送る制御を行っている (例: `kRawKeyDown` -> `kChar` vs `ImeCommitText`)。

## 結論

IME の処理において、`NSEvent` を `WebKeyboardEvent` に変換するような「イベント構造体の変換」は主役ではない。代わりに、**`NSTextInputClient` の引数（NSString, NSRange 等）を Mojo の型（String16, gfx::Range 等）に変換して送信している**。

したがって、`cbf_bridge` で IME サポートを実装する場合も、`NSEvent` を渡すのではなく、**テキストや範囲情報を直接受け取る関数** をエクスポートする必要がある。

### 推奨される実装方針

- **キー入力**: use the existing `cbf_bridge_convert_nsevent*` conversion path on the Rust side, then send the converted event through the typed bridge APIs.
- **IME**: 既存の `cbf_bridge_client_commit_text` や `cbf_bridge_client_set_composition` をそのまま利用する（これらは既にテキストと範囲を受け取る設計になっているため）。
