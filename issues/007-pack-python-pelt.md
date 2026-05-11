# 007 - Pack With Python Pelt

## Goal

Provide a practical single-artifact packaging command for the current Python
interpreter before the native runtime is complete.

## CLI

```bash
imm pack main.imm
imm pack main.imm --crate dist/app
imm pack main.imm --pelt python
```

## Pack Config

```imm
pack {
    entry "main.imm"
    crate "dist/imm-app"
    pelt "python"
}
```

CLI flags override config values.

## Meaning

| Term | Meaning |
| --- | --- |
| `pack` | Build/package operation |
| `crate` | Output artifact path |
| `pelt` | Runtime bundle strategy |

`pelt "python"` means the Python interpreter and IMM sources are bundled into a
runnable artifact.

## Implementation Options

### Option A: Zipapp

Use Python `zipapp` to create a runnable `.pyz`.

Pros:

- Standard library.
- Simple.
- Good first milestone.

Cons:

- Requires Python on target machine.
- Not a true standalone binary.

### Option B: PyInstaller

Bundle the Python runtime and IMM interpreter into a platform-specific binary.

Pros:

- Closer to user expectation for standalone.
- Good bridge before native runtime.

Cons:

- External build dependency.
- Platform-specific artifacts.

### Recommended Path

1. Implement `imm pack --pelt python` with zipapp as the no-extra-tool baseline.
2. Add optional PyInstaller backend if available.
3. Document that `pelt "native"` belongs to issue 008.

## Pack Output Contract

The packed artifact must:

- Run the selected entry `.imm`.
- Include imported `.imm` modules needed by `use`.
- Include standard libraries implemented inside the interpreter.
- Preserve relative file behavior as much as possible.
- Support `store` files next to the running artifact unless configured later.

## Acceptance Criteria

- `imm pack examples/hello.imm --crate dist/hello.pyz --pelt python` produces an
  executable artifact.
- The artifact runs and prints the same output as `imm run examples/hello.imm`.
- Missing entry file produces a clear error.
- Unsupported pelt produces a clear error.
- `pack { ... }` config can be parsed by `imm pack`.

## Test Plan

- Pack hello world.
- Pack module import example.
- Pack object/store example with generated `.immstore` outside the artifact.
- Verify bad entry path error.
- Verify bad pelt error.

## Risks

- Bundling arbitrary local modules can become complex. Start with source files
  reachable from the entry directory.
- The zipapp baseline is not a true native binary. Be explicit in docs.
- PyInstaller may require network installation if not already present, so it
  should be optional.

