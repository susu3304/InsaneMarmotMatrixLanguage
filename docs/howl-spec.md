# IMM Howl Tasks

`howl` marks async entrypoints and functions.

```imm
howl dig load() -> String {
    wait nap(10)
    return "ok"
}

howl marmot main {
    let task = scatter load()
    squeak wait task
}
```

## Constructs

- `howl marmot main { ... }` runs the entrypoint in the task runtime.
- `howl dig f(...) -> T { ... }` returns `Task<T>` when called.
- `wait task` unwraps `Task<T>` to `T` and `TaskGroup<T>` to `Array<T>`.
- `scatter expr` starts work concurrently and returns a task.
- `nest { scatter ... }` starts multiple tasks and gathers results in lexical
  order when waited.
- `nap(ms)` returns a task that sleeps without blocking other tasks.
- `tick.now()` returns UNIX milliseconds.

`wait`, `scatter`, and `nest` are static errors outside a howl context.
