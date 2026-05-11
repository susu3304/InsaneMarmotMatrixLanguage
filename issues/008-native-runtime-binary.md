# 008 - Native Runtime Binary

## Goal

Create the route to a true single native `imm` binary. The interpreter language
may change, but observable IMM behavior must be preserved by the law suite.

## Recommended Runtime Language

Rust is the preferred native implementation target.

Reasons:

- Excellent single-binary distribution.
- Strong parser/runtime ecosystem.
- Mature async support.
- Good HTTP libraries.
- Good fit for future VM or bytecode work.

Go remains a reasonable fallback if implementation speed is prioritized over
fine-grained runtime control.

## Architecture

```text
imm-native
├─ lexer
├─ parser
├─ ast
├─ static_checker
├─ runtime
│  ├─ values
│  ├─ objects
│  ├─ matrix
│  ├─ store
│  ├─ tasks
│  └─ modules
├─ stdlib
│  ├─ core
│  ├─ math
│  ├─ path
│  ├─ web
│  ├─ tick
│  └─ store
├─ cli
└─ law_runner
```

## Migration Strategy

### Stage 1: Law Harness

Before native code is large, make sure the existing Python interpreter can run
`imm law`.

### Stage 2: Lexer And Parser

Native runtime parses current IMM examples and law files.

### Stage 3: Core Runtime

Implement:

- literals
- variables
- functions
- control flow
- arrays
- matrix
- point
- objects

### Stage 4: Standard Libraries

Implement:

- `core`
- `math`
- `path`
- `store`
- `web`
- `tick`

### Stage 5: Howl Runtime

Implement:

- `howl`
- `wait`
- `scatter`
- `nest`
- `nap`

### Stage 6: Pack Native

```bash
imm pack main.imm --pelt native --crate dist/app
```

The output should be one platform-native executable containing:

- native IMM runtime
- entry source or compiled representation
- reachable module sources or compiled representation
- metadata needed for runtime errors

## Compatibility Rule

The native runtime is not accepted as complete until it passes the same `law`
suite as the Python interpreter.

## Acceptance Criteria

- A native `imm --version` works.
- Native `imm run examples/hello.imm` works.
- Native `imm law` can run core law files.
- Native `imm pack --pelt native` creates a single executable.
- The executable runs without Python.

## Test Plan

- Cross-check Python and native output for every law file.
- Golden output tests for examples.
- Error message category tests.
- Pack smoke tests per target platform.

## Risks

- Full feature parity is large. Keep the law suite granular so partial progress
  is visible.
- Async semantics must be specified tightly enough not to diverge.
- Store file format should remain runtime-independent.

## Deliverables

- Native runtime project skeleton.
- Law runner shared with Python implementation.
- Feature parity matrix.
- Native `imm` binary preview.

## Detailed Issue Breakdown

The full Rust migration issue set lives under:

```text
issues/rust-migration/
```

Start with `issues/rust-migration/README.md` and follow the RUST-001 through
RUST-018 dependency order.
