# IMM Runtime Expansion Issues

This directory is the implementation issue plan for the next IMM expansion:
networking, asynchronous execution, task management, conformance testing, debug
tracing, and single-binary packaging.

The adopted vocabulary is intentionally IMM-flavored:

| Word | Role |
| --- | --- |
| `web` | network standard library |
| `fetch` | async HTTP request |
| `grab` | sync HTTP request |
| `howl` | async function/block/main |
| `wait` | await task result |
| `scatter` | spawn concurrent task |
| `nest` | task group |
| `nap` | async sleep |
| `tick` | timer/time library |
| `pack` | package/build command |
| `crate` | output artifact |
| `pelt` | bundled runtime |
| `probe` | language-level test |
| `law` | conformance test suite |
| `expect` | assertion |
| `trace` | debug trace |

## Dependency Order

1. [001 - Reserve Keywords And Grammar Surface](001-reserve-keywords-and-grammar.md)
2. [002 - Web Standard Library](002-web-standard-library.md)
3. [003 - Howl Task Runtime](003-howl-task-runtime.md)
4. [004 - Nest, Nap, And Tick](004-nest-nap-tick.md)
5. [005 - Probe And Law](005-probe-and-law.md)
6. [006 - Trace Debugging](006-trace-debugging.md)
7. [007 - Pack With Python Pelt](007-pack-python-pelt.md)
8. [008 - Native Runtime Binary](008-native-runtime-binary.md)
9. [009 - Release And Compatibility Gate](009-release-compatibility-gate.md)

The Rust migration is further broken down in
[`rust-migration/`](rust-migration/README.md), from workspace setup through
native `pack`.

## Milestones

### Milestone A: Spec Lock

Complete issues 001 and the public API sections of 002-006. At this point the
language surface should be stable enough that examples can be written before the
runtime is complete.

### Milestone B: Python Interpreter Implementation

Complete issues 002-006 in the current Python interpreter. The interpreter may
use Python standard library networking and `asyncio`, but the public IMM behavior
must not expose Python-specific details.

### Milestone C: Single Binary Preview

Complete issue 007. This gives users a practical `imm pack` path while the
native runtime is still being designed.

### Milestone D: Native Runtime Track

Complete issue 008. This creates the long-term route to a true single native
binary, with the same law tests as the Python implementation.

### Milestone E: Release Gate

Complete issue 009. This defines compatibility, migration, and release criteria
so that the feature set can become a real IMM version rather than a pile of
working experiments.

## Definition Of Done For This Expansion

- `web.grab` works from normal `marmot main`.
- `web.fetch` works from `howl marmot main` with `wait`.
- `scatter` can run multiple tasks concurrently.
- `nest` can collect multiple scattered task results.
- `nap` and `tick.now()` are available without external dependencies.
- `probe` and `expect` can run IMM-level tests.
- `imm law` can run conformance tests shared by future runtimes.
- `trace` writes to stderr only when tracing is enabled.
- `imm pack main.imm --pelt python` can produce a runnable artifact.
- A native runtime path exists and is validated against the same law suite.
