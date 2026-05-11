# RUST-008 - Expressions And Control Flow

## Goal

Implement native evaluation for expressions and statement-level control flow.

## Dependencies

- RUST-007

## Scope

- Literals.
- Unary and binary operators.
- Logical short-circuit.
- Range `a..b`.
- Array and map literals.
- Index get/set.
- Member get/set for built-in types that already exist.
- `if` / `else if` / `else`.
- `while`.
- `for in`.
- `break`.
- `continue`.
- `return`.
- `panic`.
- `try` / `catch`.
- `insane` block semantics at current Python parity.

## Operator Parity

Match Python reference for:

```text
+ - * / %
== != < <= > >=
&& || !
```

String concatenation with `+` should use IMM formatting for non-string values,
matching the current Python behavior.

## Acceptance Criteria

- Native can run a core hello/arithmetic program.
- Native can run loops and conditionals.
- Native can execute `try` / `catch`.
- Native can relax matrix/array bounds only where Python currently does in
  insane mode after matrix lands.

## Test Plan

- Golden tests for arithmetic, strings, arrays, maps, if, while, for.
- Error tests for non-bool conditions.
- Error tests for out-of-range array access.
- Panic/catch tests.

## Notes

This issue can land before functions/modules. Keep test programs self-contained.

