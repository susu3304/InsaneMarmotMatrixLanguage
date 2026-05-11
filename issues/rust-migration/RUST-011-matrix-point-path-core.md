# RUST-011 - Matrix, Point, Path, And Core Stdlib

## Goal

Port the matrix-centered language core and standard library to Rust.

## Dependencies

- RUST-008
- RUST-009
- RUST-010

## Scope

### Core

- `len`
- `type`
- `str`
- `int`
- `float`
- `bool`
- `map`
- `filter`
- `reduce`

### Matrix And Point

- `matrix` literals.
- Rectangular row validation.
- Matrix indexing `[y, x]`.
- Point indexing `[p]`.
- Assignment.
- `width()`
- `height()`
- `in_bounds(p)`
- `points()`
- `neighbors4(p)`
- `neighbors8(p)`
- `find(v)`
- `find_all(v)`
- `@point(x, y)`
- Point equality and addition.

### Path

- `path.bfs`
- `path.astar`

## Acceptance Criteria

- Native runs `examples/matrix.imm`.
- Native runs `examples/path.imm`.
- Matrix neighbor order matches Python.
- `points()` row-major order matches spec.
- A* and BFS route output matches Python for current examples.

## Test Plan

- Matrix literal validation.
- Index get/set.
- Point member access.
- Neighbor ordering.
- Find/find_all.
- BFS/A* golden outputs.

## Risks

- Formatting values must match Python reference closely enough for golden tests.

