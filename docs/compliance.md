# IMM Spec Compliance

Status legend:

- `done`: implemented and covered by examples or tests.
- `partial`: implemented enough for early use, but missing important behavior.
- `planned`: not implemented yet.

| Spec Area | Status | Notes |
| --- | --- | --- |
| File loading, UTF-8, LF/CRLF | done | `.imm` files are read as UTF-8 and CRLF is normalized by the lexer. |
| Single-line and block comments | done | `#` and `/* ... */` are ignored by the lexer. |
| `marmot main` | done | Safe and `insane marmot main` entry points are accepted. |
| Keywords | partial | The networking/task/test/pack expansion words are reserved. Some older contextual words remain accepted as names for source compatibility. |
| Newline statements and semicolons | partial | Newlines and semicolons work for common cases. Parser recovery is not implemented. |
| Blocks | done | `{ ... }` blocks with lexical scope are implemented. |
| Basic values | done | `Int`, `Float`, `Bool`, `String`, `Array`, `Matrix`, `Point`, and `Null` exist at runtime. |
| Object values | partial | `den`, `hatch`, `self`, fields, methods, constructors, public/private access, `mask`/`wear`, and single inheritance are implemented at runtime. Static object checking is planned. |
| String escapes | done | `\n`, `\t`, `\"`, and `\\` are supported. |
| `matrix` literals | done | Rectangular row validation is implemented at runtime. |
| `let` and `stash` | done | Mutable variables and constants are implemented. |
| Type annotations | partial | Runtime checks include recursive `Array<T>`, `Matrix<T>`, den types, mask types, and object subtype validation. Static checking is planned. |
| Operators | done | Arithmetic, comparison, logical, unary, and string `+` are implemented. |
| `if` / `else if` / `else` | done | Conditions must be `Bool` at runtime. |
| `for`, ranges, `while` | done | Ranges are half-open. `break` and `continue` are implemented. |
| `dig` functions | done | Parameters, optional annotations, and returns are implemented. |
| Lambdas | partial | Expression and block lambdas work; static typing for lambdas is planned. |
| `Point` | done | `@point`, `.x`, `.y`, equality, and addition are implemented. |
| `Matrix` access and methods | done | `[y, x]`, `[p]`, assignment, size, bounds, points, neighbors, find, and find_all are implemented. |
| `tunnel` | done | `map`, `filter`, and `reduce` work with lambdas. |
| `squeak` / `sniff` | done | Output and one-line input are implemented. |
| `panic` / `try catch` | done | Runtime errors can be thrown and caught. |
| `insane` block | partial | Accepted as a mode marker. Array/String/Matrix out-of-bounds reads return `null`, and out-of-bounds writes are ignored in insane mode. More safety-difference work is planned. |
| `insane for` | partial | Accepted and executed in shuffled order. Parallel execution is planned. |
| `insane choose` | done | Random choice returns `null` for empty collections. |
| `insane try` | done | Recoverable runtime errors are swallowed. |
| `burrow` / `use` | partial | Local `.imm` modules and built-in namespaces work with cache-backed loading and cycle detection. Explicit export controls are planned. |
| `den` / `hatch` / `self` | partial | User-defined object types, constructors, methods, and instance fields work. Static initialization analysis is planned. |
| `fur` / `fang` | partial | Runtime public/private checks work in safe and insane modes. Static access checking is planned. |
| `mask` / `wear` | partial | Runtime declaration checks catch missing methods and signature mismatches. Mask-typed values expose only mask methods at runtime. Static mask-view checking is planned. |
| `under` | partial | Single inheritance, overrides, `under.init(...)`, and parent method calls work. Deeper semantic checks are planned. |
| `core` library | done | `len`, `type`, `str`, `int`, `float`, `bool`, `map`, `filter`, `reduce` are available. |
| `math` library | done | Required math functions are available. |
| `path` library | done | Basic BFS and A* are available. |
| `store` library | partial | Built-in JSON-backed object persistence supports open/save/load/all/find/get/delete/count/clear for `den` objects. Transactions, indexes, and concurrent writers are planned. |
| `web` library | done | `web.grab` and `web.fetch` use the Python standard library, return `Response`, expose fields/methods, and keep HTTP error statuses as values. |
| `howl` tasks | done | `howl marmot main`, `howl dig`, `wait`, `scatter`, `nest`, and `nap` run on an isolated asyncio-backed task layer. |
| `tick` library | done | `tick.now()` returns UNIX milliseconds. |
| `probe` / `expect` | done | Probe blocks parse, check, and run through `imm probe`; failed expects report the file and probe name. |
| `law` suite | done | `imm law` runs shared `.law.imm` probe files under `laws/`. |
| `trace` | done | `trace` writes to stderr only when `imm run --trace` is enabled. |
| Block/function scope | done | Nested lexical environments and shadowing are implemented. |
| Static type system | partial | Runtime checks exist, and `imm check` checks functions, methods, main blocks, conditions, assignments, returns, calls, object members, mask views, literal declarations, and field initializers. Full flow-sensitive checking and source spans are planned. |
| Null safety | partial | Safe-mode runtime errors exist. Insane-mode relaxed behavior is planned. |
| `imm run` | done | Runs `.imm` files. |
| `imm check` | partial | Parses, prepares declarations, resolves modules, detects module cycles, and performs broad static checks without executing top-level statements or `marmot main`. Source-span diagnostics are planned. |
| `imm fmt` | partial | Preserves comments/strings and normalizes indentation, line endings, and trailing whitespace. Full AST-preserving reflow is planned. |
| `imm probe` | done | Discovers `tests/imm/*.probe.imm` by default or runs explicit files. |
| `imm law` | done | Runs the conformance probes in `laws/`. |
| `imm pack --pelt python` | done | Produces a runnable zipapp artifact containing the interpreter and entry-directory IMM sources. |
| Native runtime track | planned | `native/` records the law-gated parity plan; `--pelt native` remains disabled until a native runtime passes the core law suite. |
| Machine-readable spec output | done | `imm spec --json` emits language metadata. |
| VM, bytecode, LSP | planned | Advanced tooling milestones. |
| CHaser runtime | partial | `chaser` helpers are available for directions, steps, field parsing, safe moves, and random moves. Full turn-loop runtime is planned. |
