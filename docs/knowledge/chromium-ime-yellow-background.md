# ChromiumにおけるIME未確定文字列の黄色ハイライトについて

## 現象
Chromium (Blink) を使用したアプリケーションにおいて、IMEでの入力中（未確定状態）のテキスト背景が明るい黄色でハイライトされる。
この黄色は非常に明るいため、ダークテーマなどで白い文字を使用している場合に視認性が著しく低下する。

## 原因
Chromiumのレンダリングエンジン（Blink）は、IMEからの入力（`SetComposition`）を受け取る際、テキストに対する装飾情報（`ImeTextSpan` / `CompositionSpan`）が一つも指定されていない場合、デフォルトのスタイルを適用する。

特定の条件下（あるいは実装上のフォールバック）において、このデフォルトスタイルとして「黄色い背景」が適用される。

## 解決策
IMEイベントをChromiumに転送する際、空のリストではなく、明示的に装飾情報（Span）を付与することで回避できる。

### 具体的な実装（Rust/CBFの例）
テキスト全体に対して、下線のみを指定し背景色を透明（`0`）にしたSpanを作成して渡す。

```rust
let utf16_len = text.encode_utf16().count();
let spans = if utf16_len > 0 {
    vec![ImeTextSpan {
        r#type: ImeTextSpanType::Composition,
        start_offset: 0,
        end_offset: utf16_len as u32,
        underline_color: 0, // 透明またはデフォルト
        thickness: ImeTextSpanThickness::Thin,
        underline_style: ImeTextSpanUnderlineStyle::Solid,
        text_color: 0,
        background_color: 0, // ここを0にすることで黄色ハイライトを回避
        // ... その他のフィールド
    }]
} else {
    Vec::new()
};

let composition = ImeComposition {
    text,
    spans,
    // ...
};
```

## 知見
- Chromiumは装飾情報がない場合に「親切心」でスタイルを当てるが、それがモダンなUI（特にダークモード）では仇となることがある。
- OSネイティブのIMEの挙動に合わせたい場合でも、明示的に「標準的なスタイル（細い下線、背景透明）」をSpanとして送るのが安全である。
