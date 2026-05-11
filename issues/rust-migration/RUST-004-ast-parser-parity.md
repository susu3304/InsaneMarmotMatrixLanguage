# RUST-004 - AST And Parser Parity

## Goal

Implement the Rust AST and parser for the full current IMM grammar.

## Dependencies

- RUST-003

## Scope

- Top-level items:
  - `marmot main`
  - `insane marmot main`
  - `howl marmot main`
  - `insane howl marmot main`
  - `dig`
  - `howl dig`
  - `den`
  - `mask`
  - `burrow`
  - `use`
  - `probe`
  - `pack`
- Statements:
  - declarations
  - assignments
  - control flow
  - loops
  - `squeak`
  - `panic`
  - `try` / `catch`
  - `insane` blocks
  - `expect`
  - `trace`
- Expressions:
  - literals
  - arrays/maps/matrix
  - point
  - call/member/index
  - lambda
  - tunnel
  - range
  - `hatch`
  - `insane choose`
  - `wait`
  - `scatter`
  - `nest`

## AST Requirements

- Preserve spans on all AST nodes.
- Preserve enough structure for static checking and diagnostics.
- Preserve type annotations as parsed type references, not only strings.

## Acceptance Criteria

- Parses every file under `examples/`, `laws/`, and `tests/imm/`.
- Rejects invalid syntax currently rejected by Python.
- Supports `pack { entry ... crate ... pelt ... }`.
- Supports strict initial `nest { scatter ... }`.

## Test Plan

- Parser snapshot tests for each major construct.
- Round-trip-ish tests using a debug AST format.
- Negative parse tests:
  - missing `}`
  - invalid `pack` item
  - `mask` method with body
  - bad `den wear under` order
  - unsupported statement inside `nest`

## Risks

- Parser drift is the biggest early migration risk. Keep Python and Rust grammar
  examples side by side until the law suite is broad enough.

