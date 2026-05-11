# RUST-006 - CLI Command Parity

## Goal

Give the Rust binary the same command surface as Python, even while individual
commands are still gated.

## Dependencies

- RUST-001
- RUST-005

## Commands

```text
imm-native --version
imm-native run <file>
imm-native check <file>
imm-native fmt [--check] <file>
imm-native probe [files...]
imm-native law
imm-native pack <file> [--crate path] [--pelt native|python]
imm-native spec [--json]
```

## Initial Gating

- `--version`: implemented.
- `spec --json`: can be implemented early from static metadata.
- `run`, `check`, `probe`, `law`: become active as parser/runtime land.
- `fmt`: may stay `not implemented` until parser is stable.
- `pack --pelt native`: stays gated until RUST-017.
- `pack --pelt python`: remains Python reference responsibility unless there is
  a deliberate reason to reimplement it in Rust.

## Acceptance Criteria

- Every command parses arguments.
- Unsupported commands fail with category `not implemented` or `pack`.
- `spec --json` includes the same keywords/libraries as Python.
- Exit codes are documented.

## Test Plan

- CLI argument tests.
- `--version` test.
- Unsupported command error test.
- `spec --json` parseable JSON test.

## Notes

This issue is about command shape, not feature parity. Keep command stubs honest
and explicit.

