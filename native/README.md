# IMM Native Runtime Track

The Python interpreter remains the behavior oracle, and the Rust binary now
uses it as a parity bridge. This keeps `imm-native` on the same law/probe/golden
surface while the native evaluator modules mature behind the same CLI.

Current project shape:

```text
native/
├─ imm-native/
│  ├─ src/
│  │  ├─ cli.rs
│  │  ├─ bridge.rs
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
`pack`, and `spec`. Runtime behavior is delegated through the reference bridge,
while Rust-owned lexer, diagnostics, module layout, and test harnesses are in
place for the Python-free evaluator work.

`imm pack --pelt native` is enabled as an executable parity bridge artifact.
It is a single runnable file, but it still depends on a compatible Python
interpreter because Python is the current semantics oracle.

The detailed migration issue plan is tracked in:

```text
issues/rust-migration/
```
