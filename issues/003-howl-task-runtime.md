# 003 - Howl Task Runtime

## Goal

Implement IMM's asynchronous execution model with `howl`, `wait`, and `scatter`.

## Language Model

```imm
howl dig fetch_body(url: String) -> String {
    let res = wait web.fetch(url)
    return res.body
}

howl marmot main {
    let a = scatter fetch_body("https://a.example")
    let b = scatter fetch_body("https://b.example")

    squeak wait a
    squeak wait b
}
```

## Semantics

| Construct | Meaning |
| --- | --- |
| `howl dig f() -> T` | Defines async function; calling it returns `Task<T>` |
| `howl marmot main` | Runs entrypoint inside event loop |
| `wait task` | Waits for `Task<T>` and returns `T` |
| `scatter expr` | Starts `expr` concurrently and returns `Task<T>` |
| `insane scatter expr` | Initially same as `scatter expr` |

## Runtime Types

### `Task<T>`

Host-backed object representing async work.

Fields or methods exposed later:

```text
done() -> Bool
cancel() -> Bool
```

Initial implementation only needs `wait`.

### Event Loop

- The interpreter should create an event loop for `howl marmot main`.
- Normal `marmot main` should not create an async context unless a sync bridge
  such as `web.grab` needs host blocking I/O.
- `wait` outside `howl` is a static error.

## Python Implementation Approach

Use `asyncio` internally, but isolate it behind IMM runtime objects:

```text
IMM HowlFunction -> Python coroutine factory
IMM Task         -> wrapper around asyncio.Task or coroutine
wait            -> await wrapped task
scatter         -> create_task(...)
```

Interpreter execution needs two paths:

1. Existing synchronous evaluation.
2. Async evaluation for howl contexts.

Recommended incremental approach:

- Add task wrappers first.
- Let `howl` functions run through an async evaluator.
- Add async-aware versions of statement/block evaluation only where needed.
- Keep synchronous code path untouched for current behavior.

## Static Checker Rules

- `wait` only inside howl function or howl main.
- `scatter` only inside howl context initially.
- Calling a `howl dig` function returns `Task<T>`.
- Returning `T` from a `howl dig f() -> T` body is valid.
- Returning `Task<T>` from the body should be rejected unless the declared return
  type is explicitly `Task<T>` in a future type system.

## Error Behavior

- If a scattered task fails, `wait task` rethrows the runtime error.
- If `howl marmot main` exits with un-awaited tasks, initial behavior may cancel
  them at shutdown.
- Cancellation should be best-effort in the Python implementation.

## Acceptance Criteria

- `howl marmot main` runs.
- `howl dig` functions can return values.
- `wait` unwraps task results.
- `scatter` starts concurrent work.
- Existing normal programs behave exactly as before.
- Errors inside tasks propagate through `wait`.

## Test Plan

- A howl function returning a string.
- `wait` on a task.
- Two `scatter nap(...)` calls complete faster than sequential waits.
- Error propagation from a task.
- Static error for `wait` in normal main.
- Regression suite for all current IMM features.

## Risks

- Duplicating evaluator logic can create drift. Keep shared helper methods for
  expression semantics where possible.
- Python `asyncio` details should not leak into IMM error messages.
- `scatter expr` must capture the current scope carefully so loop variables and
  object references behave predictably.

