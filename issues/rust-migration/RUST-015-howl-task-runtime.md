# RUST-015 - Howl Task Runtime

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Port IMM asynchronous semantics to Rust.

## Dependencies

- RUST-008
- RUST-009
- RUST-010

## Scope

- `howl marmot main`.
- `insane howl marmot main`.
- `howl dig`.
- `wait`.
- `scatter`.
- `insane scatter`.
- `nest`.
- `nap`.
- `Task<T>`.
- `TaskGroup<T>`.
- Error propagation through `wait`.
- Best-effort cancellation on task group failure.

## Runtime Recommendation

Use Tokio:

```text
tokio runtime
Task -> JoinHandle<Result<Value>>
TaskGroup -> Vec<Task>
nap -> tokio::time::sleep
web.fetch -> async HTTP task
```

## Semantics To Match

- `wait` is valid only inside howl context.
- `scatter expr` starts concurrent work immediately.
- `wait Task<T>` returns `T`.
- `wait TaskGroup<T>` returns `Array<T>` in lexical scatter order.
- `scatter` of an expression that returns a Task unwraps it, matching Python
  reference behavior.
- Errors inside tasks rethrow at `wait`.

## Acceptance Criteria

- Native runs current howl tests.
- Two `scatter nap(...)` tasks demonstrate concurrent behavior.
- `nest` preserves lexical result order.
- Task error propagation works.
- `wait` outside howl is a static error.

## Test Plan

- Howl function return.
- Scatter/wait.
- Nest result ordering.
- Nap concurrency timing with generous threshold.
- Task failure.
- Insane scatter parity.

## Risks

- Captured environments across tasks must be safe. Start with single-threaded
  runtime if needed, then widen to multi-threaded once values are Send-safe.
