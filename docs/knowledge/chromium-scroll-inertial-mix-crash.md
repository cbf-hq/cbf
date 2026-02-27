# Chromium: inertial scroll + mouse wheel 併用時の DCHECK

## 概要
macOS 上で **トラックパッドの慣性スクロール中にマウスホイールを同時入力**すると、Chromium が DCHECK でクラッシュする現象。
Atelier 側の Rust 実装とは独立に、**CBF パッチ未適用の Chromium / CEF Debug** でも再現するため、Chromium 本体側の前提不一致の可能性が高い。

## 再現条件
- macOS (arm64)
- トラックパッドの慣性スクロールが継続中に、外部マウスホイールでスクロール入力
- Atelier の子プロセスとして起動した Chromium の **標準 UI ウィンドウ** でも再現
- CEF Debug ビルドでも再現

## 典型的なクラッシュ
```
DCHECK failed: !scroll_state.is_in_inertial_phase.
cc::InputHandler::ScrollLatchedScroller
```
- 該当箇所: `chromium/src/cc/input/input_handler.cc:2065`

## 参考情報
- Atelier の Chromium: 145.0.7571.0 (Developer Build) (arm64)
- CEF Debug: 144.0.11+ge135be2+chromium-144.0.7559.97
- CEF Chromium: 144.0.7559.97 (Official Build) (arm64)
- CBF パッチ適用の有無に関わらず再現
- Rust 側入力変換や cbf_bridge_mac.mm に依存しない可能性が高い

## 仮説
- 慣性スクロール中 (`inertial_phase = true`) に **non-precise な wheel 更新**が混入することで
  `ShouldAnimateScroll()` が true となり、`ScrollLatchedScroller` の DCHECK に到達

## 今後の対応案
- Chromium 本体へのバグ報告用メモとして保管
- 必要であれば、Chromium 側で慣性中の `ShouldAnimateScroll()` を抑制する暫定パッチを検討
