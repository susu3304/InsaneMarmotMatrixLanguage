# insane marmot matrix v0.1

`insane marmot matrix` は、行列・座標・盤面処理を簡潔に記述するための小さなプログラミング言語である。略称は `IMM`、ソースファイル拡張子は `.imm` とする。

## 基本

- 実行コマンド: `imm run main.imm`
- trace 付き実行: `imm run main.imm --trace`
- 構文チェック: `imm check main.imm`
- フォーマット: `imm fmt main.imm`
- テスト: `imm probe` / `imm law`
- パッケージ: `imm pack main.imm --pelt python`
- 機械可読メタデータ: `imm spec --json`
- エントリーポイント: `marmot main`
- async エントリーポイント: `howl marmot main`
- 狂気エントリーポイント: `insane marmot main`
- 出力: `squeak`
- 入力: `sniff`
- 関数定義: `dig`
- 変数定義: `let`
- 定数定義: `stash`

```imm
marmot main {
    squeak "Hello, insane marmot matrix!"
}
```

## 値

基本型は `Int`、`Float`、`Bool`、`String`、`Array<T>`、`Matrix<T>`、`Point`、`Null` とする。初期実装では動的型付けを中心とし、型注釈がある場合は実行時に検査する。

```imm
let x: Int = 10
let name: String = "marmot"
stash MAX_TURN = 100
```

## Matrix と Point

`matrix` は組み込みリテラルで、全行の長さが一致していなければならない。Matrix は `[y, x]` または `[point]` でアクセスする。

```imm
let field = matrix [
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 0]
]

let p = @point(2, 1)
squeak field[p]
field[1, 2] = 9
```

利用できる Matrix メソッド:

- `width()`
- `height()`
- `in_bounds(p)`
- `points()`
- `neighbors4(p)`
- `neighbors8(p)`
- `find(v)`
- `find_all(v)`

## 制御構文

```imm
if score >= 80 {
    squeak "good"
} else {
    squeak "bad"
}

for i in 0..5 {
    squeak i
}

while i < 5 {
    i = i + 1
}
```

## 関数

```imm
dig add(a: Int, b: Int) -> Int {
    return a + b
}
```

## tunnel

`tunnel` は左辺の値を右辺の関数へ渡す。

```imm
let result = [1, 2, 3, 4]
    tunnel filter(x => x % 2 == 0)
    tunnel map(x => x * 10)
```

標準で `map`、`filter`、`reduce` を提供する。

## insane

`insane` は危険・高速モードを表す構文属性である。v0.1 の初期実装では多くの挙動は通常実行と同じだが、構文として受理し、`insane choose` と `insane try` を実装する。

```imm
insane {
    field[y, x] = 1
}

let move = insane choose ["UP", "DOWN", "LEFT", "RIGHT"]

insane try {
    risky()
}
```

## howl とタスク

`howl dig` は async 関数を定義し、呼び出すと `Task<T>` を返す。
`howl marmot main` の中では `wait`、`scatter`、`nest` を使える。

```imm
howl dig load() -> String {
    wait nap(10)
    return "ok"
}

howl marmot main {
    let task = scatter load()
    squeak wait task
}
```

`wait` は `Task<T>` を `T` に、`TaskGroup<T>` を `Array<T>` に変換する。
`scatter expr` は並行タスクを開始する。`nest { scatter ... }` は複数の
タスクをまとめ、待機時に字句順の結果配列を返す。

## probe / expect / trace

`probe` は IMM レベルのテストブロックで、`expect expr` が `true` でなければ失敗する。

```imm
probe "add" {
    expect 1 + 1 == 2
}
```

`trace expr` は `imm run --trace` のときだけ stderr に出力する。通常の
`squeak` 出力とは分離される。

## 標準ライブラリ

`core` はデフォルト読み込みとし、`len`、`type`、`str`、`int`、`float`、`bool`、`map`、`filter`、`reduce` を提供する。

`math` は `abs`、`min`、`max`、`sqrt`、`floor`、`ceil`、`random` を提供する。

`path` は `bfs` と `astar` を提供する。

`chaser` は CHaser 風ボット向けに `direction`、`step`、`parse_field`、`safe_moves`、`random_move` を提供する。

`store` は外部DBなしの標準永続化機能として、`den` オブジェクトを `.immstore` ファイルへ保存・復元する `open`、`save`、`load`、`all`、`find`、`get`、`delete`、`count`、`clear` を提供する。

`web` は同期 HTTP の `grab` と async HTTP の `fetch` を提供する。
戻り値 `Response` は `status`、`headers`、`body`、`url`、`ok`、`text()`、`json()` を持つ。

`tick` は `now()` で UNIX ミリ秒を返す。`nap(ms)` は howl タスクとしてスリープする。

## pack

`pack` ブロックまたは `imm pack` で Python pelt の zipapp artifact、
または native pelt の Python-free Rust 実行 artifact を作成できる。

```imm
pack {
    entry "examples/hello.imm"
    crate "dist/hello.pyz"
    pelt "python"
}
```

`--pelt native` は Rust `imm-native` の law suite ゲートを通る Python-free evaluator として有効。
