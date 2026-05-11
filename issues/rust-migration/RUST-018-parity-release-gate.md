# RUST-018 - Parity Release Gate

## Goal

Define the final gate for declaring the Rust runtime ready and enabling native
packaging publicly.

## Dependencies

- RUST-001 through RUST-017

## Required Checks

```bash
python3 tests/run_tests.py
./imm law
cd native/imm-native && cargo fmt --check
cd native/imm-native && cargo clippy -- -D warnings
cd native/imm-native && cargo test
cd native/imm-native && cargo run -- law
```

Plus native pack smoke tests once RUST-017 lands.

## Parity Matrix Requirements

`native/parity-matrix.md` must show `done` or explicitly documented gaps for:

- Lexer/parser.
- CLI.
- Static checker.
- Core runtime.
- Matrix/Point/path.
- Objects/masks.
- Store.
- Web/tick.
- Howl tasks.
- Probe/law/trace.
- Native pack.

## Release Criteria

- All law files pass on Python and Rust.
- Golden examples match stdout.
- Error category tests match.
- Store cross-read/write tests pass.
- Native packed hello/module/howl examples run.
- Docs state remaining gaps honestly.

## Documentation Updates

- `README.md`.
- `docs/compliance.md`.
- `docs/pack-spec.md`.
- `docs/roadmap.md`.
- `native/README.md`.
- `native/parity-matrix.md`.

## Acceptance Criteria

- `imm pack --pelt native` is no longer gated.
- The native artifact runs without Python.
- The Python reference remains available until at least one release after native
  parity, so users have a fallback.

## Final Notes

This is the point where Rust becomes either the default runtime or an officially
supported alternative. Do not cross this gate on partial behavior.

