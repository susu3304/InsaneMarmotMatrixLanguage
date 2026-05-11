# RUST-017 - Native Pack

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Enable `imm pack --pelt native` and produce a single executable that runs an IMM
program without Python.

## Dependencies

- RUST-016
- RUST-014
- RUST-015

## Pack Command

```bash
imm pack main.imm --crate dist/app --pelt native
```

Pack config:

```imm
pack {
    entry "main.imm"
    crate "dist/app"
    pelt "native"
}
```

## Output Contract

The artifact must contain:

- Native IMM runtime.
- Entry `.imm` source or compiled representation.
- Reachable local `.imm` modules.
- Standard library runtime code.
- Enough metadata for runtime error paths.

The artifact must not require:

- Python.
- Cargo.
- Source repository checkout.

## Recommended Implementation Options

### Option A: Source Embedding

Embed entry and local modules as bytes into a generated Rust wrapper, then build
that wrapper.

Pros:

- Simple.
- Preserves interpreter semantics.

Cons:

- Requires Rust toolchain at pack time.

### Option B: Runtime Binary + App Bundle Format

Append a source bundle to the native runtime executable.

Pros:

- Faster pack once implemented.
- No generated Rust project needed.

Cons:

- More custom binary work.

### Recommended First Native Pack

Use Option A first. Move to Option B after native runtime stabilizes.

## Acceptance Criteria

- `imm pack examples/hello.imm --pelt native --crate dist/hello` builds.
- The resulting executable prints the same output as `imm run examples/hello.imm`.
- Packed app can use local `use` modules.
- Packed app can use `web`, `store`, and `howl`.
- Bad `pelt` and missing entry errors are clear.

## Test Plan

- Pack hello.
- Pack module example.
- Pack store example.
- Pack howl/web example using deterministic data URL or local server.
- Run artifact with no Python in PATH if practical in CI.

## Risks

- Cross-compilation is out of scope for the first release. Build for host
  platform first.
