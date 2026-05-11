# RUST-009 - Functions, Lambdas, Tunnel, And Modules

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Implement native user functions, lambdas, pipeline processing, and module import.

## Dependencies

- RUST-008

## Scope

- `dig` definitions.
- Function calls.
- Parameter count checks.
- Return type runtime checks once type model exists.
- Lambdas:
  - `x => expr`
  - `(a, b) => expr`
  - block lambdas
- `tunnel`.
- Built-in `map`, `filter`, and `reduce`.
- `burrow` module definition.
- `use` module loading.
- Module cache.
- Module cycle detection.

## Module Rules

- Resolve `use math/path/store/web/tick/chaser` as built-ins.
- Resolve local `use foo` as `foo.imm` next to the importing file.
- Cache modules by canonical path and mode.
- Detect cycles like `a -> b -> a`.

## Acceptance Criteria

- Native runs `examples/use_module.imm`.
- Native runs lambda/tunnel examples.
- Native catches module cycles.
- Top-level statements in modules behave like Python reference.

## Test Plan

- Function return tests.
- Lambda expression/block tests.
- `tunnel map/filter/reduce` tests.
- Module import test.
- Module cycle test.

## Risks

- Captured environments are central to objects, howl functions, and lambdas. Keep
  closure representation shared and well-tested.
