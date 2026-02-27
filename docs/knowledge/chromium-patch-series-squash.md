# Chromium パッチ系列の単一化とパッチ化

## 概要

`chromium/src` に積み上げた CBF 関連のコミットを 1 つにまとめ、
`chromium/patches/cbf/0001-Atelier-cbf-bridge-series.patch` に単一パッチとして書き出す手順をまとめる。

本プロジェクトでは `chromium/patches/cbf/series.toml` の `base_commit` を基準にする。

## 背景と目的

- Chromium への変更は「パッチ管理」が基本運用。
- `chromium/src` 側は通常クリーンに保つ。
- 作業履歴は `git format-patch` で管理できる状態にする。

## 前提

- `chromium/src` が存在する
- `chromium/patches/cbf/series.toml` に `base_commit` がある

## 手順

### 1) 対象コミット範囲を確認

`base_commit` から `HEAD` までのコミットを確認する。

```bash
git -C chromium/src log --oneline <base_commit>..HEAD
```

### 2) 未コミット変更を確認

```bash
git -C chromium/src status -sb
```

### 3) 未コミット変更をコミット

```bash
git -C chromium/src add -A
git -C chromium/src commit -m "Add context menu handling for CBF"
```

GPG 署名が失敗する環境では `--no-gpg-sign` を付ける。

```bash
git -C chromium/src commit -m "Add context menu handling for CBF" --no-gpg-sign
```

### 4) コミットを 1 つにまとめる

`base_commit` まで soft reset し、変更をステージに残す。

```bash
git -C chromium/src reset --soft <base_commit>
```

まとめた状態で 1 コミット作成。

```bash
git -C chromium/src commit -m "Atelier cbf bridge series" --no-gpg-sign
```

### 5) 単一パッチとして書き出す

```bash
git -C chromium/src format-patch -1 HEAD --stdout \
  > chromium/patches/cbf/0001-Atelier-cbf-bridge-series.patch
```

### 6) クリーン確認

```bash
git -C chromium/src status -sb
```

## 注意点

- `reset --soft` は履歴を 1 つにまとめるための操作。履歴が必要なら別ブランチへ退避する。
- `format-patch` の出力先は `series.toml` の運用と一致させる。
- 署名付きコミットが必須の場合は GPG 環境を整備してから実行する。

## 関連ドキュメント

- `docs/architecture/chromium-fork-workflow.md`
- `chromium/patches/cbf/series.toml`
