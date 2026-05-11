# insane marmot matrix

`insane marmot matrix`、略称 `IMM` は、行列・座標・盤面処理を簡潔に書くための小さな実験言語です。

このリポジトリには、仕様 v0.1 の最初のインタプリタ実装が入っています。依存パッケージなしの Python 実装です。

## 実行

```bash
./imm --version
./imm check examples/hello.imm
./imm run examples/hello.imm
./imm probe
./imm law
./imm pack examples/hello.imm --crate dist/hello.pyz --pelt python
./imm pack examples/hello.imm --crate dist/hello-native --pelt native
cd native/imm-native && cargo run -- run ../../examples/hello.imm
```

出力例:

```text
Hello, insane marmot matrix!
```

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
- `howl` / `wait` / `scatter` / `nest`
- `nap` / `tick.now()`
- `probe` / `expect` / `imm law`
- `trace` と `imm run --trace`
- `imm pack --pelt python` による zipapp パッケージ
- `imm pack --pelt native` による Python-free Rust 実行バイナリ
- `imm-native` Rust CLI による `run` / `check` / `fmt` / `probe` / `law` / `pack` / `spec`

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
./imm run main.imm
./imm run main.imm --trace
./imm check main.imm
./imm fmt main.imm
./imm fmt --check main.imm
./imm probe [file.imm]
./imm law
./imm pack main.imm --crate dist/app.pyz --pelt python
./imm pack main.imm --crate dist/app --pelt native
./imm spec --json
./imm --version
```

`check` は構文解析に加えて、宣言準備、モジュール解決、循環 import 検出、静的に判定できるリテラル型チェックを行います。`marmot main` やトップレベル文は実行しません。

`fmt` はコメントと文字列を保ちながら、改行コード、行末空白、ブロックインデントを整えます。

`chaser` ライブラリには `direction`、`step`、`parse_field`、`safe_moves`、`random_move` が入っています。

`store` ライブラリには、外部DBなしで `den` オブジェクトを永続化する `open`、`save`、`load`、`all`、`find`、`get`、`delete`、`count`、`clear` が入っています。保存ファイルの拡張子は慣例として `.immstore` です。

`web` ライブラリには同期 `grab` と howl 向けの `fetch` が入っています。`howl` タスク、`probe`/`law`、`trace`、`pack`、Rust native track の詳細は `docs/web-spec.md`、`docs/howl-spec.md`、`docs/pack-spec.md`、`native/README.md` を参照してください。

## 開発

```bash
python3 tests/run_tests.py
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
