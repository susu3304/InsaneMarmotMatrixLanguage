# 001 - Reserve Keywords And Grammar Surface

## Goal

Reserve and parse the new words for networking, asynchronous execution, testing,
debugging, and packaging without changing existing IMM behavior.

## Keywords To Add

```text
web
fetch
grab
howl
wait
scatter
nest
nap
tick
pack
crate
pelt
probe
law
expect
trace
```

`web`, `tick`, and `law` are mostly namespace/CLI-facing words, but reserving
them early avoids future ambiguity.

## Grammar Additions

```text
item              := main_def
                   | howl_main_def
                   | function_def
                   | howl_function_def
                   | module_def
                   | use_stmt
                   | den_def
                   | mask_def
                   | probe_def
                   | pack_def
                   | statement

howl_main_def     := insane? "howl" "marmot" "main" block

howl_function_def := "howl" "dig" IDENT "(" params? ")" return_type? block

probe_def         := "probe" STRING block

pack_def          := "pack" "{" pack_item* "}"

pack_item         := "entry" STRING
                   | "crate" STRING
                   | "pelt" STRING

statement         := existing_statement
                   | expect_stmt
                   | trace_stmt

expect_stmt       := "expect" expr

trace_stmt        := "trace" expr_list?

wait_expr         := "wait" expr

scatter_expr      := insane? "scatter" expr

nest_expr         := "nest" block

primary           := existing_primary
                   | wait_expr
                   | scatter_expr
                   | nest_expr
```

`entry` should initially be a contextual keyword inside `pack { ... }`, not a
global reserved word.

## Parser Work

- Add tokenizer keywords.
- Add AST nodes for:
  - `HowlFunctionDef`
  - `HowlMainDef`
  - `WaitExpr`
  - `ScatterExpr`
  - `NestExpr`
  - `ProbeDef`
  - `ExpectStmt`
  - `TraceStmt`
  - `PackDef`
- Preserve old AST shape for existing programs.
- Ensure formatter can round-trip all new constructs.

## Static Checker Work

- Reject `wait` outside a `howl` context.
- Allow `howl marmot main` as the program entrypoint.
- Reject having both `marmot main` and `howl marmot main` in the same program.
- Treat `howl dig f() -> T` as callable with result type `Task<T>`.
- Treat `wait Task<T>` as `T`.
- Treat `scatter expr` as `Task<T>`.
- Treat `nest { ... }` as `TaskGroup<T>` or `TaskGroup<Any>` in the first pass.
- Reject `expect` outside `probe` only if the initial design chooses strict test
  blocks. The recommended initial behavior is allowing `expect` anywhere while
  `imm probe` gives it special reporting.

## Acceptance Criteria

- Existing examples still parse, format, check, and run.
- These programs parse:

```imm
howl dig load() -> String {
    return "ok"
}

howl marmot main {
    let task = scatter load()
    squeak wait task
}
```

```imm
probe "add" {
    expect 1 + 1 == 2
}
```

```imm
pack {
    entry "main.imm"
    crate "dist/app"
    pelt "python"
}
```

- This program fails static check:

```imm
marmot main {
    wait nap(100)
}
```

Expected error:

```text
wait can only be used inside howl context
```

## Test Plan

- Parser tests for each new construct.
- Formatter check and rewrite tests.
- Static checker tests for invalid `wait`, duplicate main entrypoints, and
  `pack` item validation.
- Regression test for all current examples.

## Risks

- `wait expr` precedence can become confusing. Make it bind like unary.
- `nest { ... }` uses a block-like expression. Keep it explicit in parser tests.
- Reserving `web` could break a user variable named `web`; this is acceptable
  for the language expansion but should be documented.

