# RUST-001 - Workspace, Toolchain, And CI Baseline

## Goal

Turn `native/imm-native` from a preview scaffold into a maintainable Rust project
with stable toolchain expectations, CI-ready commands, and room for runtime
modules.

## Dependencies

None.

## Scope

- Decide whether `native/imm-native` remains a single crate or becomes a
  workspace.
- Add source module layout.
- Add baseline dependencies.
- Add formatting/lint/test commands.
- Add a small native smoke test.

## Recommended Project Layout

```text
native/imm-native/
├─ Cargo.toml
├─ src/
│  ├─ main.rs
│  ├─ cli.rs
│  ├─ error.rs
│  ├─ source.rs
│  ├─ token.rs
│  ├─ lexer.rs
│  ├─ ast.rs
│  ├─ parser.rs
│  ├─ checker.rs
│  ├─ runtime/
│  │  ├─ mod.rs
│  │  ├─ value.rs
│  │  ├─ env.rs
│  │  ├─ eval.rs
│  │  ├─ object.rs
│  │  ├─ matrix.rs
│  │  ├─ store.rs
│  │  └─ task.rs
│  └─ stdlib/
│     ├─ mod.rs
│     ├─ core.rs
│     ├─ math.rs
│     ├─ path.rs
│     ├─ web.rs
│     └─ tick.rs
└─ tests/
   ├─ cli_smoke.rs
   └─ golden.rs
```

## Baseline Dependencies

Use only dependencies that are likely to survive `pack --pelt native`.

Recommended initial set:

```toml
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Later issues may add:

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

## Acceptance Criteria

- `cargo fmt --check` passes.
- `cargo clippy -- -D warnings` passes or is documented as pending.
- `cargo test` passes.
- `cargo run -- --version` prints native IMM version.
- Project structure has modules ready for lexer/parser/runtime work.

## Test Plan

- Add a CLI smoke test for `--version`.
- Add an error-code smoke test for unsupported command.
- Run `cargo test`.

## Notes

Keep `Cargo.lock` committed once real dependencies are added. For application
binaries, a committed lockfile makes native builds reproducible.

