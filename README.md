# insane marmot matrix

`insane marmot matrix`、略称 `IMM` は、行列・座標・盤面処理を簡潔に書くための小さな実験言語です。

このリポジトリには、仕様 v0.1 の最初のインタプリタ実装が入っています。依存パッケージなしの Python 実装です。

## 実行

```bash
./imm --version
./imm check examples/hello.imm
./imm run examples/hello.imm
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
./imm check main.imm
./imm fmt main.imm
./imm fmt --check main.imm
./imm spec --json
./imm --version
```

`check` は構文解析に加えて、宣言準備、モジュール解決、循環 import 検出、静的に判定できるリテラル型チェックを行います。`marmot main` やトップレベル文は実行しません。

`fmt` はコメントと文字列を保ちながら、改行コード、行末空白、ブロックインデントを整えます。

`chaser` ライブラリには `direction`、`step`、`parse_field`、`safe_moves`、`random_move` が入っています。

`store` ライブラリには、外部DBなしで `den` オブジェクトを永続化する `open`、`save`、`load`、`all`、`find`、`get`、`delete`、`count`、`clear` が入っています。保存ファイルの拡張子は慣例として `.immstore` です。

## 開発

```bash
python3 tests/run_tests.py
```

完全版までの道筋は `docs/roadmap.md`、仕様対応状況は `docs/compliance.md` にまとめています。
