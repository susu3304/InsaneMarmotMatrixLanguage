# 006 - Trace Debugging

## Goal

Add a debug trace mechanism separate from normal program output.

## Syntax

```imm
marmot main {
    let x = 10
    trace x
    squeak x
}
```

## CLI

```bash
imm run main.imm --trace
```

## Behavior

- `squeak` writes to stdout.
- `trace` writes to stderr.
- If tracing is disabled, `trace` evaluates as little as practical.
- Initial implementation may evaluate the expression but suppress output.
- `trace` should include location when available.

Suggested output:

```text
[trace] main.imm:3 x = 10
```

If the original expression text is not preserved:

```text
[trace] main.imm:3 10
```

## Static Checker

- `trace` accepts one or more expressions.
- `trace` returns no value.
- `trace` is valid in normal and howl contexts.

## Acceptance Criteria

- `trace x` produces no stdout.
- `trace x` produces stderr only with `--trace`.
- `trace` works inside functions and loops.
- `trace` works inside `howl` tasks without mixing into `squeak` output.

## Test Plan

- Run with and without `--trace`.
- Verify stderr/stdout separation.
- Verify trace from nested function.
- Verify trace from scattered task after issue 003.

## Future Work

- Add trace categories.
- Add task id in howl contexts.
- Add structured trace output for editor tooling.

