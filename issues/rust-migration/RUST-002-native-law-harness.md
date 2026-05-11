# RUST-002 - Native Law Harness

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Create the native-side harness that compares Rust behavior with the Python
reference through shared `.law.imm` files and golden examples.

## Dependencies

- RUST-001

## Scope

- Add `imm-native law` command stub.
- Add test utilities that locate repository `laws/`.
- Add parity runner script or Rust integration test.
- Define the output normalization rules.

## Behavior Contract

Python remains reference:

```bash
./imm law
native/imm-native/target/debug/imm-native law
```

Both should eventually run the same law files. Before runtime parity exists, the
native command may report `not implemented`, but the harness must be able to
discover files and enumerate planned tests.

## Golden Comparison Rules

- Compare stdout exactly after LF normalization.
- Compare stderr by error category first, then message text where stable.
- Compare exit code.
- Do not compare wall time except for explicit async timing tests with generous
  thresholds.

## Acceptance Criteria

- Native test harness can discover all `laws/*.law.imm`.
- A native integration test records each law file as pending until implemented.
- Python `imm law` is invoked from the parity harness or documented as a required
  pre-step.
- Parity matrix can be generated or updated from harness output.

## Test Plan

- Add a test that fails if no law files are found.
- Add a test that lists `core.law.imm`, `matrix.law.imm`, `web.law.imm`, and
  `howl.law.imm`.
- Add a test that can compare a simple manually supplied stdout fixture.

## Risks

- Law files may rely on features not yet native. Mark unsupported features as
  pending instead of silently skipping.
- Avoid baking absolute developer paths into tests.
