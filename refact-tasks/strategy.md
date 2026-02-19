# CBF API再編リファクタリング戦略

## 目的

ADR 0001で決定した `cbf` / `cbf-chrome` / `cbf-chrome-sys` の3レイヤー構成への移行を、体系的かつ段階的に進めるための戦略を策定する。
`cbf` / `cbf-chrome` / `cbf-chrome-sys` の3レイヤー構成へ移行し、以下を同時に満たす。

- `cbf` を browser-generic 語彙に限定する
- Chromium固有語彙を `cbf-chrome` / `cbf-chrome-sys` に隔離する
- 既存利用者への移行コストを段階的に抑える

## 基本方針

1. 最初の全体棚卸しは単一エージェントで実施する  
分類基準のぶれを防ぐため、最初のAPI一覧化と初期分類は一人で行う。

2. 分類結果をタスク化してから並列実装する  
分類後はモジュールごとに独立タスクへ分解し、複数エージェントで並列実装する。

3. 進捗管理は `refact-tasks/` を一次ソースにする  
GitHub Issue は要約・追跡用途にとどめ、詳細設計と判断履歴はリポジトリ内で管理する。

## フェーズ設計

## フェーズ0: 棚卸し基盤の作成

- `refact-tasks/00-inventory.md` を作成する
- 対象API（型、関数、モジュール、イベント/コマンド、FFI境界）を列挙する
- 各項目に以下メタ情報を付与する
  - 定義場所
  - 主要利用箇所
  - Chromium依存度
  - 破壊的変更リスク

## フェーズ1: カテゴライズ

全APIを以下カテゴリに分類する。

- `cbf-keep`: `cbf` にそのまま残せる
- `cbf-chrome-keep`: `cbf-chrome` にそのまま移せる
- `split-rebuild`: 分解＆再構築が必要
- `sys-to-chrome-sys`: `cbf-sys` から `cbf-chrome-sys` へ移行すべき

### 分類ルール（優先順）

1. Chromium内部名詞・内部意味に依存するものは原則 `cbf-chrome*`
2. 複数バックエンドで意味が安定する語彙は `cbf`
3. 概念は汎用だが表現がChromium寄りなら `split-rebuild`
4. ABI/unsafe/IPC wire は `cbf-chrome-sys`

### 命名対応メタ情報（必須）

棚卸し時は、`cbf` 名と Chromium 実装名の対応を必ず記録する。
これは `cbf-chrome` / `cbf-chrome-sys` の語彙を Chromium 側の実名に寄せるための基盤情報とする。

`refact-tasks/00-inventory.md` の各項目に、少なくとも以下を持たせる。

- `cbf` 現在名
- `cbf-chrome` 候補名
- `cbf-chrome-sys` / `cbf_bridge` 側名
- Chromium 元名（型名/フィールド名/enum値）
- 参照元ファイル（Chromium 側パス）
- 乖離理由（抽象化意図、歴史的事情など）
- 命名方針（維持 / Chromium名へ寄せる / 分割）

## フェーズ2: タスク分解

分類結果を基に `refact-tasks/NN-*.md` を作成し、モジュール単位で実装可能にする。

各タスクファイルに必ず含める項目:

- 対象ファイル/モジュール
- 現状APIと問題点
- 移行先（`cbf` / `cbf-chrome` / `cbf-chrome-sys`）
- 必要な再設計内容（型分割、変換、互換層）
- 受け入れ条件（テスト、ドキュメント、互換性）

## フェーズ3: 実装順序

1. 境界の土台実装  
`Backend` / `RawCommand` / `RawEvent` / `OpaqueEvent` / `send_raw` / `as_raw`

2. 影響範囲が明確な入力系から移行  
`key` / `mouse` / `ime` / `drag`

3. `cbf-sys` から `cbf-chrome-sys` への移設  
wire型をChromium固有表現に寄せる

4. deprecate と互換層整理  
段階的に旧APIを非推奨化し、移行ガイドを整備する

## 実装体制

- 棚卸しと分類基準確定までは単一エージェント
- タスク分解後は独立性の高いモジュールから並列実装
- 境界定義（trait/イベントモデル）は常に単一担当で最終決定する

## GitHub Issueとの使い分け

- エピック/進捗の見える化: GitHub Issue
- 詳細設計/分類理由/作業メモ: `refact-tasks/`
- Issue本文には `refact-tasks/*.md` の参照リンクのみを置き、重複管理を避ける

## 完了条件

- `cbf` の公開APIから Chromium固有語彙が排除されている
- `cbf-chrome` が Chromium固有の safe API を提供している
- `cbf-chrome-sys` が unsafe/wire責務を一元化している
- 主要移行対象のテストとドキュメントが更新されている

## 初手の具体アクション

1. `refact-tasks/00-inventory.md` を作成する
2. `key` / `mouse` / `ime` / `drag` を先行棚卸し対象にする
3. 分類基準に沿って初期タグ付けを完了する
4. `01-` 以降の実装タスクへ分解する
