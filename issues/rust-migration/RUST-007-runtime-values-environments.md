# RUST-007 - Runtime Values And Environments

## Goal

Implement the native value model and lexical environment foundation.

## Dependencies

- RUST-004
- RUST-005

## Runtime Values

Required initial value enum:

```text
Null
Bool
Int
Float
String
Array
Map
Matrix
Point
Range
Function
BuiltinFunction
Namespace
Object
ObjectView
MaskType
DenType
Response
Task
TaskGroup
Store
```

Some variants can be placeholders until their issue lands.

## Environment Rules

- Lexical scoping.
- Block scoping.
- Function scoping.
- Shadowing allowed.
- `stash` constants cannot be reassigned.
- Type annotation metadata stored with bindings.

## Control Signals

Implement internal control flow signals:

```text
Return(value)
Break
Continue
Panic(message)
```

## Acceptance Criteria

- Values can be formatted with IMM `str`/`squeak` behavior.
- Variables can be defined, shadowed, read, and assigned.
- `stash` assignment errors.
- Null, Bool, Int, Float, String, Array, and Map values work.

## Test Plan

- Environment unit tests.
- Value formatting tests.
- Shadowing test.
- Stash reassignment test.
- Type name test.

## Risks

- Rust ownership around closures and mutable environments can get tricky. Prefer
  `Rc<RefCell<...>>` or an arena design early; optimize later.

