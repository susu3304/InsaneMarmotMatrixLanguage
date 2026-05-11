# RUST-012 - Object Model

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Port IMM object-oriented features to Rust.

## Dependencies

- RUST-009
- RUST-010

## Scope

- `den`.
- `hatch`.
- `self`.
- `init`.
- `fur` public members.
- `fang` private members.
- `mask`.
- `wear`.
- `under`.
- Single inheritance.
- Parent init and method calls through `under`.
- Method overriding checks.
- Object identity equality.
- Mask-typed views.

## Runtime Rules

- Missing `init` allows zero-arg construction only.
- Uninitialized fields are errors.
- `init` cannot return a value.
- Parent `fang` is not directly accessible from child den.
- External access to `fang` is an error.
- Mask views expose only mask methods.

## Acceptance Criteria

- Native runs `examples/objects.imm`.
- Native rejects missing mask methods.
- Native rejects private field access.
- Native supports `under.init(...)`.
- Native supports subtype assignment for child den to parent type.

## Test Plan

- Port object tests from Python suite.
- Add parent init test.
- Add method override signature test.
- Add mask-view restriction test.
- Add object identity equality test.

## Risks

- Object storage and method closures interact with modules and store. Keep object
  metadata stable and serializable.
