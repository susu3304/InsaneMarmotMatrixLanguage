# RUST-013 - Store Compatibility

## Goal

Port the built-in `.immstore` persistence layer to Rust while preserving file
format compatibility with the Python reference.

## Dependencies

- RUST-012
- RUST-011

## Scope

- `store.open(path)`.
- `store.save(db, object)`.
- `store.load(db, Type, id)`.
- `store.all(db, Type)`.
- `store.find(db, Type, field, value)`.
- `store.get(db, Type, field, value)`.
- `store.delete(db, Type, id)`.
- `store.count(db, Type)`.
- `store.clear(db, Type)`.
- JSON-backed `.immstore` format.
- Object field serialization including private fields.
- Point, Matrix, Array, primitive, Null serialization.
- Cycle rejection.

## Compatibility Rule

A store written by Python must be readable by Rust. A store written by Rust must
be readable by Python.

## Acceptance Criteria

- Native runs `examples/store.imm`.
- Cross-runtime read/write compatibility test passes.
- Saving the same object to the same store updates the existing record.
- `*.immstore.tmp` atomic-ish write behavior is preserved or improved.

## Test Plan

- Python write -> Rust read.
- Rust write -> Python read.
- Store lifecycle test matching Python suite.
- Private field persistence test.
- Nested object test.
- Cycle error test.

## Risks

- JSON type tags must stay stable. Document any format extension before writing
  it.

