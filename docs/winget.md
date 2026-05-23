# winget release

このプロジェクトは Windows 向けに `imm.exe` を zip で配布し、winget では portable package として `imm` コマンドを登録する。

## 公開後のインストール

winget-pkgs にマニフェストが取り込まれた後は、次でインストールできる。

```powershell
winget install --id susu3304.InsaneMarmotMatrixLanguage
imm --version
```

## リリース成果物

`v*` タグを push すると `.github/workflows/release-windows.yml` が Windows x64 で次を作る。

- `imm-windows-x64.zip`: `imm.exe` を含む portable zip。
- `imm-winget-manifests.zip`: winget-pkgs にコピーできる 3 分割マニフェスト。

workflow_dispatch でも同じ成果物を Actions artifact として作成できる。タグ実行時は GitHub Release にも添付する。

## 手動でマニフェストを作る

Windows zip の SHA256 を取得してから、ローカルでマニフェストを生成する。

```bash
scripts/generate-winget-manifest.sh \
  --version 0.1.0 \
  --sha256 <IMM_WINDOWS_X64_ZIP_SHA256> \
  --release-date 2026-05-23
```

出力先は既定で `dist/winget/manifests/s/susu3304/InsaneMarmotMatrixLanguage/<version>/`。

リリース URL が標準形と違う場合は `--installer-url` を渡す。

```bash
scripts/generate-winget-manifest.sh \
  --version 0.1.0 \
  --sha256 <IMM_WINDOWS_X64_ZIP_SHA256> \
  --installer-url https://example.com/imm-windows-x64.zip
```

## Windows で検証する

```powershell
winget validate .\dist\winget\manifests\s\susu3304\InsaneMarmotMatrixLanguage\0.1.0
winget install --manifest .\dist\winget\manifests\s\susu3304\InsaneMarmotMatrixLanguage\0.1.0
imm --version
```

`winget validate` は Windows Package Manager Community Repository へ出す前の検証に使う。

## winget-pkgs への提出

1. `microsoft/winget-pkgs` を fork する。
2. 生成された `manifests/s/susu3304/InsaneMarmotMatrixLanguage/<version>/` を fork 側へコピーする。
3. fork 側で `winget validate` を通す。
4. pull request を作成する。

このリポジトリには再配布ライセンスがまだ明示されていないため、生成マニフェストの既定値は `License: Proprietary` にしている。MIT などへ変更する場合は、先にリポジトリ直下へライセンスファイルを追加し、`--license` で値を差し替える。
