# IMM Completion Plan

This is the strict completion plan for turning IMM into a non-placeholder implementation. The project should treat a feature as complete only when parsing, execution, checking, documentation, and tests agree.

## Track A: Language Semantics

1. Runtime semantics
   - Keep the tree-walking interpreter as the reference implementation.
   - Every language feature must have runtime tests before any VM work begins.
   - Runtime checks stay even after static checking exists.

2. Object semantics
   - `den`, `hatch`, `self`, `init`, `fur`, `fang`, `mask`, `wear`, and `under` must work in both direct examples and nested module examples.
   - `mask`-typed values expose only mask members.
   - Private access rules must be explicit in safe mode and insane mode.

3. Module semantics
   - Module loading must use canonical paths, a cache, and cycle detection.
   - Exports must become explicit or consistently documented.
   - Imported den/mask/function names must be available through namespaces without accidental global leakage.

## Track B: Static Checking

1. Declaration preparation
   - Parse files and imports.
   - Register functions, den types, masks, and module namespaces.
   - Validate inheritance, mask implementation, duplicate declarations, and init signatures.

2. Type inference and checking
   - Infer literals, arrays, matrices, points, hatch expressions, function calls, member access, and tunnel chains.
   - Check assignments, function calls, returns, conditions, and object member access.
   - Treat unknown/dynamic values as explicit `Any`, not as silent success.

3. Check command
   - `imm check` must not execute user code.
   - `imm check` must still report resolvable type and semantic errors.

## Track C: Tooling

1. Formatting
   - Replace the current whitespace normalizer with an AST/token-aware formatter.
   - Preserve comments.
   - Add `imm fmt --check`.

2. Diagnostics
   - Add spans to tokens and AST nodes.
   - Report file, line, column, and diagnostic code.
   - Keep messages short and actionable.

3. Spec output and LSP
   - Add a machine-readable command after diagnostics are stable.
   - Build LSP support on the same checker/formatter libraries.

## Track D: Runtime Quality

1. Insane mode
   - Specify which checks are skipped, which are relaxed, and which remain hard errors.
   - Make every difference between safe and insane mode tested.

2. Performance
   - Add Matrix-heavy benchmarks.
   - Only start bytecode/VM work after the interpreter is semantically complete.

3. Domain libraries
   - Expand pathfinding options.
   - Add board utilities.
   - Add CHaser runtime after module and object semantics are stable.

## Current Implementation Slice

This slice starts Track A and Track B cleanup:

- Runtime declaration preparation separated from main execution.
- Module cache and cycle detection.
- Mask-typed object views.
- Tests for non-executing `check`, module cycles, and mask view restrictions.

Completed in this slice:

- `imm check` now uses declaration preparation and focused static checks instead of executing top-level user statements.
- Local module loading uses canonical paths, a shared cache, and cycle detection.
- Mask-typed values now expose only mask methods at runtime.
- `imm check` now walks function, method, and main bodies for common type and member-access errors.
- `imm fmt --check` and indentation formatting are available.
- `imm spec --json` emits machine-readable metadata.
- `chaser` runtime helpers are available.
- `store` object persistence is available as a built-in JSON-backed database.
