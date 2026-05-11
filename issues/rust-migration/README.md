# Rust Migration Issue Plan

This directory fully breaks down the Rust/native migration track for IMM. The
Python interpreter remains the reference implementation until the Rust runtime
passes the shared `law` suite and can build a native `imm pack --pelt native`
artifact.

## Migration Rule

Rust must not redefine IMM by accident. The native runtime is accepted only when
observable behavior matches the Python reference for the same `.imm` programs.

The shared checks are:

- `imm law` conformance files.
- Golden output examples.
- Error category tests.
- Pack smoke tests.
- Store file compatibility tests.

## Dependency Order

1. [RUST-001 - Workspace, Toolchain, And CI Baseline](RUST-001-workspace-toolchain-ci.md)
2. [RUST-002 - Native Law Harness](RUST-002-native-law-harness.md)
3. [RUST-003 - Lexer With Source Spans](RUST-003-lexer-source-spans.md)
4. [RUST-004 - AST And Parser Parity](RUST-004-ast-parser-parity.md)
5. [RUST-005 - Diagnostics And Error Categories](RUST-005-diagnostics-error-categories.md)
6. [RUST-006 - CLI Command Parity](RUST-006-cli-command-parity.md)
7. [RUST-007 - Runtime Values And Environments](RUST-007-runtime-values-environments.md)
8. [RUST-008 - Expressions And Control Flow](RUST-008-expressions-control-flow.md)
9. [RUST-009 - Functions, Lambdas, Tunnel, And Modules](RUST-009-functions-lambdas-tunnel-modules.md)
10. [RUST-010 - Static Checker And Type Model](RUST-010-static-checker-type-model.md)
11. [RUST-011 - Matrix, Point, Path, And Core Stdlib](RUST-011-matrix-point-path-core.md)
12. [RUST-012 - Object Model](RUST-012-object-model.md)
13. [RUST-013 - Store Compatibility](RUST-013-store-compatibility.md)
14. [RUST-014 - Web And Tick Stdlib](RUST-014-web-tick-stdlib.md)
15. [RUST-015 - Howl Task Runtime](RUST-015-howl-task-runtime.md)
16. [RUST-016 - Probe, Law, And Trace](RUST-016-probe-law-trace.md)
17. [RUST-017 - Native Pack](RUST-017-native-pack.md)
18. [RUST-018 - Parity Release Gate](RUST-018-parity-release-gate.md)

## Milestones

### Native A: Executable Shell

Complete RUST-001 through RUST-006. The Rust binary has real CLI shape, can
discover law files, can parse enough metadata to report meaningful diagnostics,
and can participate in parity runs.

### Native B: Core Language

Complete RUST-007 through RUST-011. The Rust runtime can run core examples:
hello, arithmetic, arrays, maps, control flow, functions, modules, matrix, point,
and path.

### Native C: IMM Runtime Features

Complete RUST-012 through RUST-016. Objects, store, web, howl, probe, law, and
trace reach Python-reference parity.

### Native D: Single Binary

Complete RUST-017 and RUST-018. `imm pack --pelt native` is enabled and the
result runs without Python.

## Global Definition Of Done

- Rust `imm-native --version` works.
- Rust `imm-native run examples/hello.imm` works.
- Rust `imm-native law` passes the same conformance suite as Python.
- Rust and Python produce the same stdout/stderr for golden examples.
- Rust can read and write the same `.immstore` format.
- Rust `web`, `howl`, and `pack` behavior matches the current specs.
- `imm pack --pelt native` is enabled only after the parity gate passes.

## Non-Goals For The First Native Release

- Bytecode VM.
- JIT.
- LSP server.
- Full optimizer.
- Parallel `insane for`.

Those can come after the native interpreter is stable.

