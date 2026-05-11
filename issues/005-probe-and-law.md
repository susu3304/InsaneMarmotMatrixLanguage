# 005 - Probe And Law

## Goal

Add IMM-native tests and a conformance suite that can be shared by the current
Python interpreter and a future native runtime.

## `probe`

`probe` defines a test block:

```imm
probe "matrix access" {
    let field = matrix [
        [1, 2],
        [3, 4]
    ]

    expect field[0, 1] == 2
}
```

## `expect`

```text
expect expr
```

If `expr` is not `true`, the current probe fails.

Initial behavior outside `probe`:

- Recommended: allow it and fail the program if false.
- `imm probe` should give richer reporting.

## CLI

```bash
imm probe
imm probe tests/matrix.imm
imm law
```

### `imm probe`

- Discovers `*.probe.imm` and possibly regular `.imm` files containing `probe`
  blocks under `tests/imm`.
- Runs probe blocks.
- Reports pass/fail count.

### `imm law`

- Runs conformance files under `laws/`.
- Intended to be implementation-agnostic.
- Must avoid host-specific behavior unless explicitly testing a host feature.

Suggested layout:

```text
laws/
├─ core.law.imm
├─ matrix.law.imm
├─ object.law.imm
├─ store.law.imm
├─ web.law.imm
├─ howl.law.imm
└─ pack.law.md
```

## Runtime Behavior

- Probe blocks do not run during `imm run` unless a file explicitly asks for it
  in a future mode.
- Top-level definitions are available to probes.
- Each probe should run in a fresh local scope.
- Global side effects should be isolated where practical.

## Acceptance Criteria

- `imm probe file.imm` runs probe blocks in that file.
- A passing `expect` is reported as pass.
- A failing `expect` reports file, probe name, and expression location when
  location info is available.
- `imm law` can run a small law suite.
- Existing `imm check` understands `probe` and `expect`.

## Test Plan

- Passing probe.
- Failing probe.
- Multiple probes in one file.
- Probe using `dig`, `den`, `matrix`, and `store`.
- Law command smoke test.
- Ensure `imm run` does not accidentally run probes.

## Future Work

- Add setup/teardown blocks only if needed.
- Add snapshots only after text output behavior is stable.
- Add expected panic syntax later, perhaps:

```imm
expect panic {
    risky()
}
```

