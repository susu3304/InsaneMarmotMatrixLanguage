# RUST-016 - Probe, Law, And Trace

## Goal

Port IMM test and debug facilities to Rust.

## Dependencies

- RUST-006
- RUST-010
- RUST-015 for async probe coverage

## Scope

- `probe "name" { ... }`.
- `expect expr`.
- `imm-native probe [files...]`.
- `imm-native law`.
- `trace expr`.
- `imm-native run --trace`.

## Probe Rules

- Probe blocks do not run during normal `run`.
- Each probe runs in a fresh local scope.
- Top-level definitions are available to probes.
- Failed expect reports file and probe name.

## Trace Rules

- `squeak` writes stdout.
- `trace` writes stderr only with `--trace`.
- `trace` is valid inside normal and howl contexts.

## Acceptance Criteria

- Native `probe` runs explicit probe files.
- Native `law` runs all `laws/*.law.imm`.
- Native `trace` output does not pollute stdout.
- Python and Rust law results match.

## Test Plan

- Passing probe.
- Failing probe.
- Multiple probes in one file.
- Probe with objects/matrix/store.
- Trace disabled/enabled.
- Trace inside scattered task.

## Notes

This issue is the main acceptance path for every previous runtime issue.

