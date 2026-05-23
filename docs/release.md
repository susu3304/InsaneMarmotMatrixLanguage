# Release workflow

このリポジトリでは、ユーザーに配る実行バイナリが必要な更新だけを GitHub Release にする。
内部メモ、未公開の issue 整理、ユーザー影響のない小修正は `main` へ入れるだけでよい。

## Versioning

- `patch`: 後方互換のバグ修正、ドキュメント修正、配布物の修正。
- `minor`: 後方互換の言語機能、標準ライブラリ、CLI 機能の追加。
- `major`: 既存 `.imm` プログラム、CLI、`.immstore` を壊す変更。`1.0.0` までは、破壊的変更を入れる場合も release notes に明記して minor を上げる。

Rust CLI の公開バージョンは `Cargo.toml` の `[package].version` を唯一の正とする。
`imm-native --version` と `imm-native spec --json` はこの値を使う。

VS Code 拡張だけを出す場合は `editors/vscode/imm/package.json` を別に上げる。CLI と拡張を同時に配る場合でも、拡張の version は VSIX 用として独立に扱う。

## Release checklist

1. 変更内容から次の version を決める。
2. `Cargo.toml` の `[package].version` を更新する。
3. 挙動が変わった場合は `README.md`、`docs/compliance.md`、関連 spec、examples、laws を更新する。
4. ローカル gate を通す。

```bash
cargo fmt --check
cargo test --locked
cargo run -- law
cargo run -- run examples/hello.imm
cargo run -- pack examples/hello.imm --crate dist/release-smoke --pelt native
./dist/release-smoke
```

5. 変更を commit する。
6. 注釈付き tag を作る。

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

7. `v*` tag push で `.github/workflows/release-windows.yml` が Windows x64 の Release を作る。
   成果物は `imm-windows-x64.zip` と `imm-winget-manifests.zip`。
8. GitHub Release の asset と SHA256 を確認する。
9. winget に出す場合は `docs/winget.md` の手順で `imm-winget-manifests.zip` の内容を `microsoft/winget-pkgs` へ提出する。

## Release notes

GitHub Release には最低限これを書く。

```text
## Changes
- ...

## Compatibility
- ...

## Install
- Windows portable zip: imm-windows-x64.zip
- winget: submitted / pending / not submitted

## Verification
- cargo fmt --check
- cargo test --locked
- cargo run -- law
```

互換性を壊した場合は `Compatibility` を省略しない。`.immstore` の読み書き形式を変えた場合は、移行方法も release notes と docs に書く。

## Fixing a bad release

tag を push する前なら、ローカル tag を消して作り直す。

```bash
git tag -d vX.Y.Z
git tag -a vX.Y.Z -m "vX.Y.Z"
```

公開済み tag は基本的に上書きしない。問題が出たら修正 commit を作り、次の patch version を出す。
