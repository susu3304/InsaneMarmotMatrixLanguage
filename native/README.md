# IMM Native Runtime Track

The Python interpreter is the reference runtime. The native runtime track is
gated by the shared `imm law` suite so partial native work cannot silently drift
from Python behavior.

Planned project shape:

```text
native/
├─ lexer
├─ parser
├─ ast
├─ static_checker
├─ runtime
├─ stdlib
├─ cli
└─ law_runner
```

Current acceptance gate:

- Python `imm law` must pass before native parity claims are updated.
- Native implementations must run the same `.law.imm` files.
- `imm pack --pelt native` remains disabled until a native runtime can run the
  core law suite without Python.

`imm-native/` is the initial Rust binary scaffold. It intentionally exposes only
`--version` until lexer/parser/runtime work reaches the law gate.
