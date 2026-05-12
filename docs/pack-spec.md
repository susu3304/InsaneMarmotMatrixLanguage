# IMM Pack

`imm pack` builds a runnable artifact for an IMM entry file.

```bash
imm-native pack examples/hello.imm --crate dist/hello-native --pelt native
```

The native pelt builds a small Rust wrapper that embeds the entry directory's
`.imm` sources and links the `imm-native` evaluator. The resulting artifact runs
without a source checkout.

Pack config can live in an IMM file:

```imm
pack {
    entry "examples/hello.imm"
    crate "dist/hello-native"
    pelt "native"
}
```

CLI flags override `crate` and `pelt`. Supported pelt values:

| Pelt | Status |
| --- | --- |
| `native` | implemented Rust executable |

Required release checks for the native pelt:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- law
```
