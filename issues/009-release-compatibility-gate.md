# 009 - Release And Compatibility Gate

## Goal

Define the gate for shipping the networking, async, testing, debug, and packaging
features as a coherent IMM release.

## Release Candidate Requirements

- All current Python tests pass.
- `imm law` passes in the Python interpreter.
- `web.grab`, `web.fetch`, `howl`, `wait`, `scatter`, `nest`, `nap`, `tick`,
  `probe`, `expect`, and `trace` are documented.
- `imm pack --pelt python` works on the development platform.
- Native runtime has either:
  - a working preview with documented gaps, or
  - a clear milestone plan and law-driven parity matrix.

## Compatibility Policy

### Source Compatibility

Existing IMM programs should continue to run unless they used one of the newly
reserved words as an identifier.

### Store Compatibility

`.immstore` format should remain readable. If it changes, introduce a format
version migration path.

### Law Compatibility

Every new feature must add law tests unless it is platform-specific packaging
behavior. Packaging behavior should have CLI smoke tests instead.

## Documentation Checklist

- Update `docs/spec-v0.1.md` with new reserved words.
- Add or update `docs/compliance.md`.
- Add `docs/web-spec.md`.
- Add `docs/howl-spec.md`.
- Add `docs/pack-spec.md`.
- Add examples:
  - `examples/web_grab.imm`
  - `examples/howl_fetch.imm`
  - `examples/nest_nap.imm`
  - `examples/probe_sample.imm`

## Final Acceptance Criteria

- A user can read the docs and write:

```imm
use web

howl marmot main {
    let a = scatter web.fetch("https://example.com")
    let b = scatter nap(100)

    squeak (wait a).status
    wait b
}
```

- A user can run:

```bash
imm probe
imm law
imm pack examples/hello.imm --crate dist/hello --pelt python
```

- The repository has a clear next step for `--pelt native`.

## Release Notes Draft

```text
IMM adds web networking, howl async tasks, probe/law testing, trace debugging,
and pack packaging. This release keeps the Python interpreter as the reference
runtime while opening the path to a native single-binary runtime.
```

