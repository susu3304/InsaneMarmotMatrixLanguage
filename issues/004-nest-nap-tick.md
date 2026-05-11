# 004 - Nest, Nap, And Tick

## Goal

Add task grouping and time primitives on top of the howl runtime.

## Public API And Syntax

### `nest`

```imm
howl marmot main {
    let group = nest {
        scatter work(1)
        scatter work(2)
        scatter work(3)
    }

    let results = wait group
}
```

`nest { ... }` returns a `TaskGroup`. `wait TaskGroup` returns an array of
results in declaration order.

### `nap`

```imm
howl marmot main {
    wait nap(1000)
}
```

```text
nap(ms: Int) -> Task<Void>
```

### `tick`

```imm
let start = tick.now()
let end = tick.now()
squeak end - start
```

```text
tick.now() -> Int
```

Initial unit is UNIX milliseconds.

## `nest` Semantics

- Only `scatter` statements inside `nest` contribute tasks.
- The first implementation may reject arbitrary statements inside `nest`.
- Result order is the lexical order of `scatter` statements.
- If any task fails, `wait group` raises an error.
- Remaining task cancellation is best-effort.

Recommended strict first grammar:

```text
nest_expr := "nest" "{" nest_item* "}"
nest_item := insane? "scatter" expr
```

The sample with `for url in urls { scatter ... }` can be a later enhancement.

## Implementation Plan

1. Add `TaskGroup` host-backed value.
2. Parse and evaluate `nest` as an expression.
3. Implement `nap` in core or a built-in namespace.
4. Add `tick` built-in namespace with `now`.
5. Teach `wait` to unwrap both `Task` and `TaskGroup`.

## Acceptance Criteria

- `wait nap(10)` pauses without blocking other scattered tasks.
- `tick.now()` returns an integer millisecond timestamp.
- `nest` gathers results in lexical order.
- Failure in one nested task causes `wait group` to fail.
- `nest` with no tasks returns an empty array when waited.

## Test Plan

- `tick.now()` type and monotonic-ish behavior.
- `nap` duration smoke test with generous tolerance.
- `nest` result ordering test.
- `nest` error propagation test.
- Static/parser error for unsupported statements inside strict initial `nest`.

## Future Work

- Allow loops inside `nest`.
- Add cancellation APIs.
- Add `Task.done()` and `Task.cancel()`.
- Add all-settled mode, perhaps as a library function instead of syntax.

