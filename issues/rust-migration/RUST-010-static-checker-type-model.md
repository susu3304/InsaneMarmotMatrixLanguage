# RUST-010 - Static Checker And Type Model

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Port the current static checker behavior into Rust and make it a stable gate for
`imm-native check`.

## Dependencies

- RUST-004
- RUST-009

## Scope

- Type model:
  - `Any`
  - `Void`
  - `Null`
  - `Int`
  - `Float`
  - `Bool`
  - `String`
  - `Array<T>`
  - `Map`
  - `Matrix<T>`
  - `Point`
  - `Range`
  - `Task<T>`
  - `TaskGroup<T>`
  - den/mask types
- Checks:
  - non-bool `if` / `while`
  - assignment to constants
  - declaration annotations
  - return type
  - function call arity/types
  - object member access where known
  - `wait` / `scatter` / `nest` howl-context restrictions
  - private member access

## Acceptance Criteria

- Native `check` passes current valid examples.
- Native `check` fails the same broad invalid tests as Python.
- Error categories are stable even if exact wording differs.

## Test Plan

- Port the current Python static-check cases.
- Add type parser tests for generics.
- Add `Task<T>` / `TaskGroup<T>` type tests.
- Add mask-view restriction tests.

## Notes

Do not aim for a perfect static type system in the first native release. Match
the current Python checker first.
