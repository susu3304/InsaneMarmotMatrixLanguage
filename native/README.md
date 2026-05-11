# IMM Native Runtime Track

The Rust binary now parses, checks, and executes IMM directly. The Python
interpreter remains in the repository as a reference and fallback, but
`imm-native` no longer delegates runtime behavior to Python.

Current project shape:

```text
native/
├─ imm-native/
│  ├─ src/
│  │  ├─ cli.rs
│  │  ├─ lexer.rs
│  │  ├─ parser.rs
│  │  ├─ checker.rs
│  │  ├─ runtime/
│  │  └─ stdlib/
│  └─ tests/
└─ parity-matrix.md
```

Acceptance gate:

- `python3 tests/run_tests.py`
- `./imm law`
- `cd native/imm-native && cargo fmt --check`
- `cd native/imm-native && cargo clippy -- -D warnings`
- `cd native/imm-native && cargo test`
- `cd native/imm-native && cargo run -- law`

`imm-native` supports `--version`, `run`, `check`, `fmt`, `probe`, `law`,
`pack`, and `spec`. Runtime behavior is handled by the Rust evaluator, including
core expressions, functions, modules, Matrix/Point/path, objects/masks, store,
web data URLs, howl/task syntax, probe/law, and trace.

`imm pack --pelt native` is enabled as a Python-free executable. It embeds the
entry directory's `.imm` sources into a small Rust wrapper linked against
`imm-native`.

The detailed migration issue plan is tracked in:

```text
issues/rust-migration/
```
