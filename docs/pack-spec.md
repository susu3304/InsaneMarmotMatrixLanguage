# IMM Pack

`imm pack` builds a runnable artifact for an IMM entry file.

```bash
imm pack examples/hello.imm --crate dist/hello.pyz --pelt python
imm pack examples/hello.imm --crate dist/hello-native --pelt native
```

The Python pelt is implemented with the standard-library zipapp format. It
bundles `imm_lang` and `.imm` sources from the entry directory, then extracts
those sources to a temporary directory when the artifact runs.

The first native pelt is implemented as the same executable bundle shape, gated
by the Rust `imm-native` parity bridge and the shared law suite. This gives
`--pelt native` a runnable single-file artifact while the Rust implementation
keeps Python as the behavior oracle.

Pack config can live in an IMM file:

```imm
pack {
    entry "examples/hello.imm"
    crate "dist/hello.pyz"
    pelt "python"
}
```

CLI flags override `crate` and `pelt`. Supported pelt values:

| Pelt | Status |
| --- | --- |
| `python` | implemented zipapp baseline |
| `native` | implemented parity bridge executable tracked by `native/` |

Required release checks for the native pelt:

```bash
python3 tests/run_tests.py
./imm law
cd native/imm-native && cargo fmt --check
cd native/imm-native && cargo clippy -- -D warnings
cd native/imm-native && cargo test
cd native/imm-native && cargo run -- law
```
