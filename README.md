# insane marmot matrix

`insane marmot matrix`、略称 `IMM` は、行列・座標・盤面処理を簡潔に書くための小さな実験言語です。

このリポジトリには、仕様 v0.1 の Rust 実装が入っています。lexer / parser / checker / runtime は Rust 側で直接持ち、Python 版への委譲や同梱は行いません。

## 実行

```bash
cargo run -- --version
cargo run -- check examples/hello.imm
cargo run -- run examples/hello.imm
cargo run -- probe
cargo run -- law
cargo run -- pack examples/hello.imm --crate dist/hello-native --pelt native
```

出力例:

```text
Hello, insane marmot matrix!
```

## インストール

Windows では winget 公開用の portable zip とマニフェストを用意しています。通常のリリース手順は `docs/release.md`、winget-pkgs への提出手順は `docs/winget.md` にまとめています。winget-pkgs に取り込まれた後は次で入れられます。

```powershell
winget install --id susu3304.InsaneMarmotMatrixLanguage
imm --version
```

GitHub Release は `v*` タグを push すると作成されます。

## 実装済み

- `.imm` ファイル読み込み
- `marmot main` / `insane marmot main`
- `dig` / `return`
- `let` / `stash`
- `Int` / `Float` / `Bool` / `String` / `Array` / `Matrix` / `Point` / `Null`
- `if` / `else if` / `else`
- `for in` / `0..n` / `while`
- `break` / `continue`
- `squeak` / `sniff`
- `panic` / `try catch` / `insane try`
- `matrix`、`@point(x, y)`、`field[y, x]`、`field[p]`
- `width` / `height` / `in_bounds` / `points` / `neighbors4` / `neighbors8` / `find` / `find_all`
- `tunnel map/filter/reduce`
- `insane choose`
- `den` / `hatch` / `self`
- `fur` / `fang` による public/private
- `mask` / `wear`
- 単一継承 `under` と `under.init(...)` / `under.method(...)`
- `use math` / `use path`
- 同じディレクトリの `foo.imm` を `use foo` で読み込む簡易モジュール
- `use web` による `web.grab` / `web.fetch`
- `web.den` / `web.burrow` / `web.release` / `web.peek` によるHTTPサーバー
- `howl` / `wait` / `scatter` / `nest`
- `nap` / `tick.now()`
- `probe` / `expect` / `imm law`
- `trace` と `imm run --trace`
- `imm-native pack --pelt native` による単体 Rust 実行バイナリ
- Rust CLI による `run` / `check` / `fmt` / `probe` / `law` / `pack` / `spec`

## 例

```imm
mask Movable {
    dig move(dir: String) -> Void
}

den Player wear Movable {
    fur let name: String
    fang let hp: Int

    fur dig init(name: String) {
        self.name = name
        self.hp = 100
    }

    fur dig move(dir: String) {
        squeak self.name + " digs " + dir
    }
}

marmot main {
    let p = hatch Player("marmot")
    p.move("UP")
}
```

## コマンド

```bash
cargo run -- run main.imm
cargo run -- run main.imm --trace
cargo run -- check main.imm
cargo run -- fmt main.imm
cargo run -- fmt --check main.imm
cargo run -- probe [file.imm]
cargo run -- law
cargo run -- pack main.imm --crate dist/app --pelt native
cargo run -- spec --json
cargo run -- --version
```

`check` は構文解析に加えて、宣言準備、モジュール解決、循環 import 検出、静的に判定できるリテラル型チェックを行います。`marmot main` やトップレベル文は実行しません。

`fmt` はコメントと文字列を保ちながら、改行コード、行末空白、ブロックインデントを整えます。

`chaser` ライブラリには `direction`、`step`、`parse_field`、`safe_moves`、`random_move` が入っています。

`store` ライブラリには、外部DBなしで `den` オブジェクトを永続化する `open`、`save`、`load`、`all`、`find`、`get`、`delete`、`count`、`clear` が入っています。保存ファイルの拡張子は慣例として `.immstore` です。

`web` ライブラリには async HTTP の `grab`、howl 向けの `fetch`、IMM語彙のHTTPサーバーAPI `den` / `burrow` / `release` / `peek` が入っています。`howl` タスク、`probe`/`law`、`trace`、`pack` の詳細は `docs/web-spec.md`、`docs/howl-spec.md`、`docs/pack-spec.md` を参照してください。

## 開発

```bash
cargo fmt --check
cargo test
cargo run -- law
```

性能確認用の小さいベンチは `benchmarks/` にあります。

```bash
benchmarks/run.sh
```

## VS Code

VS Code 拡張機能は `editors/vscode/imm` にあります。

```bash
cd editors/vscode/imm
npm test
npm run package
code --install-extension imm-vscode-0.1.0.vsix
```

`.imm` のシンタックスハイライト、スニペット、保存時 `imm check`、`imm fmt`、`IMM: Run File` / `IMM: Run Law Suite` などのコマンドを提供します。

完全版までの道筋は `docs/roadmap.md`、仕様対応状況は `docs/compliance.md` にまとめています。
