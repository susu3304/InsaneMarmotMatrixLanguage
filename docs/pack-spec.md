# IMM Pack

`imm pack` builds a runnable artifact for an IMM entry file.

```bash
imm pack examples/hello.imm --crate dist/hello.pyz --pelt python
imm pack examples/hello.imm --crate dist/hello-native --pelt native
```

The Python pelt is implemented with the standard-library zipapp format. It
bundles `imm_lang` and `.imm` sources from the entry directory, then extracts
those sources to a temporary directory when the artifact runs.

The native pelt builds a small Rust wrapper that embeds the entry directory's
`.imm` sources and links the `imm-native` evaluator. The resulting artifact runs
without Python or a source checkout.

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
| `native` | implemented Python-free Rust executable |

Required release checks for the native pelt:

```bash
python3 tests/run_tests.py
./imm law
cd native/imm-native && cargo fmt --check
cd native/imm-native && cargo clippy -- -D warnings
cd native/imm-native && cargo test
cd native/imm-native && cargo run -- law
```
