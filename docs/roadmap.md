# IMM Complete Roadmap

This roadmap turns the current v0.1 interpreter into a complete `insane marmot matrix` implementation. The guiding rule is simple: preserve the language's small, playful surface while making the implementation predictable enough for games, competitive programming, and CHaser-style bots.

## Current State

The current implementation is a tree-walking interpreter. It already supports the core executable shape:

- CLI: `imm run`, `imm check`, `imm fmt`, `imm --version`
- Entry points: `marmot main`, `insane marmot main`
- Declarations: `dig`, `let`, `stash`
- Core values: `Int`, `Float`, `Bool`, `String`, `Array`, `Matrix`, `Point`, `Null`
- Control flow: `if`, `else if`, `else`, `for`, `while`, `break`, `continue`
- I/O and errors: `squeak`, `sniff`, `panic`, `try catch`, `insane try`
- Matrix helpers: `width`, `height`, `in_bounds`, `points`, `neighbors4`, `neighbors8`, `find`, `find_all`
- Functional helpers: `tunnel`, `map`, `filter`, `reduce`, lambdas
- Object model: `den`, `hatch`, `self`, `fur`, `fang`, `mask`, `wear`, and single inheritance with `under`
- Libraries: `core`, `math`, `path`
- Simple local modules through `use foo`

The missing pieces are mostly about completeness, rigor, and tooling: full static checking, precise diagnostics, real formatting, optimizer/VM work, LSP support, and CHaser-specific runtime packaging.

## Milestone 0: Stabilize The Interpreter

Goal: make the current interpreter hard to accidentally break.

- Add an automated smoke test suite for CLI behavior, samples, type errors, module loading, and Matrix semantics.
- Normalize runtime error messages and ensure all public commands return useful exit codes.
- Add a compliance table that tracks each spec section as `done`, `partial`, or `planned`.
- Keep generated files out of the tree.

Exit criteria:

- `python3 tests/run_tests.py` passes.
- Every example in `examples/` either runs or has a documented reason not to.
- Roadmap and compliance docs match the actual implementation.

## Milestone 1: Spec-Complete Tree-Walking Runtime

Goal: complete all language behavior that does not require a compiler or VM.

- Implement recursive runtime type checks for `Array<T>` and `Matrix<T>`.
- Support `Void` return checking consistently.
- Harden lambda parsing and block lambdas.
- Make `insane for` explicitly unordered by shuffling iteration order.
- Define safe-mode versus insane-mode behavior for Matrix/Array/Null checks.
- Improve module loading with cycle detection and clearer module export rules.
- Add deterministic seeding hooks for tests involving `insane choose`.
- Extend Array/String convenience methods where they are implied by examples, such as `.len()`.
- Enforce mask-typed object views at runtime.
- Separate declaration preparation from program execution.

Exit criteria:

- All Phase 1 and Phase 2 examples from the spec run.
- `imm check` catches syntax errors and resolvable semantic errors without running `marmot main`.
- Type annotations are enforced for variables, parameters, returns, arrays, matrices, den types, and mask types.
- `use` never recursively loads the same module forever.
- A value typed as a `mask` exposes only that mask's methods.

## Milestone 2: Static Type Checker

Goal: make IMM a "static typed style" language instead of a purely dynamic interpreter.

- Build a symbol-table pass for modules, functions, scopes, and constants.
- Infer local variable types from literals and known functions.
- Validate operator compatibility before execution.
- Validate `if` and `while` conditions are `Bool`.
- Validate `return` type compatibility on all visible return paths.
- Validate Matrix row shape and element types where literals are static.
- Validate object member visibility, mask views, field initialization, and inheritance rules before execution.
- Produce line/column diagnostics with stable error codes.

Exit criteria:

- `imm check main.imm` performs parse + semantic + type checking.
- Runtime type errors remain as a safety net, not the primary checker.
- `imm check` does not execute user code, call `squeak`, read `sniff`, mutate state, or depend on random behavior.

## Milestone 2.5: Proper Diagnostics

Goal: make errors useful enough to fix code without guessing.

- Attach source spans to AST nodes.
- Report `file:line:column` for syntax, semantic, and runtime errors.
- Add stable diagnostic codes, for example `IMM1001` for syntax and `IMM3001` for type errors.
- Recover from parse errors in tooling modes so multiple errors can be shown.

Exit criteria:

- Every failing CLI command prints the source location when one is available.
- Tests cover representative diagnostics.

## Milestone 3: Formatter And Source Tooling

Goal: make `.imm` files pleasant to maintain.

- Preserve comments while formatting.
- Canonicalize indentation, block layout, matrix literals, and tunnel chains.
- Add a `--check` mode to `imm fmt`.
- Add parser recovery so tooling can report multiple errors.
- Add a machine-readable spec output command, for example `imm spec --json`.

Exit criteria:

- `imm fmt` is stable and idempotent.
- Existing examples round-trip without semantic changes.

## Milestone 4: Insane Runtime And Performance

Goal: make `insane` meaningful beyond syntax.

- Define unsafe operations precisely: unchecked Matrix/Array access, relaxed Null/member checks, relaxed dynamic type checks.
- Add `insane for` execution strategies: shuffled, chunked, and optional parallel execution.
- Add profiling hooks for Matrix-heavy programs.
- Implement bytecode or a small VM for hot code paths.
- Keep safe mode as the default and easiest path.

Exit criteria:

- Safe mode and insane mode have documented, tested behavioral differences.
- Matrix-heavy benchmarks show measurable improvement in insane/VM modes.

## Milestone 5: Libraries And CHaser Runtime

Goal: make IMM useful for the intended domains.

- Expand `path.bfs` and `path.astar` with options for weights and diagonal movement.
- Add board/game utility modules.
- Add CHaser input/output helpers and turn-loop runtime.
- Add deterministic simulation harnesses for game AI.
- Add packaging guidance for contest submissions.

Exit criteria:

- A CHaser-style bot can be written entirely in IMM and run through the bundled runtime.
- Pathfinding examples cover common board constraints.

## Milestone 6: Editor And Ecosystem

Goal: make IMM feel like a real small language.

- LSP server with diagnostics, hover, completion, go-to-definition, and formatting.
- Syntax highlighting grammar.
- Package/module layout rules.
- Documentation site generated from the spec and examples.
- Release artifacts for macOS/Linux/Windows.

Exit criteria:

- Users can install `imm`, edit `.imm` with language support, run samples, and build bots without reading the implementation.

## Immediate Implementation Slice

The previous implementation slice completed Milestone 0 plus the first practical pieces of Milestone 1:

- Add `tests/run_tests.py`.
- Add a compliance document.
- Enforce generic runtime types for `Array<T>` and `Matrix<T>`.
- Make `insane for` unordered.
- Improve `imm check` so it validates top-level declarations and module resolution without executing `marmot main`.

The next slice should focus on static analysis:

1. Replace `imm check`'s execution-shaped path with declaration preparation plus focused semantic checks.
2. Add module cache and cycle detection.
3. Enforce mask-typed views, so a `Movable` variable exposes only `Movable` methods.
4. Add tests that prove `check` does not execute `marmot main` and that module cycles fail cleanly.
5. Start the source-span work needed for full diagnostics.

## No More Placeholder Policy

Any feature marked `partial` must now move through this lifecycle:

1. Document the exact missing behavior in `docs/compliance.md`.
2. Add a failing or pending test that names the missing behavior.
3. Implement the behavior or keep it explicitly in the roadmap with an owner milestone.
4. Update compliance only after the behavior is covered by tests.

This keeps "accepted syntax" from pretending to be "complete semantics".
