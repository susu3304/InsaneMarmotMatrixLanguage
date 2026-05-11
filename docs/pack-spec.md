# IMM Pack

`imm pack` builds a runnable artifact for an IMM entry file.

```bash
imm pack examples/hello.imm --crate dist/hello.pyz --pelt python
```

The Python pelt is implemented with the standard-library zipapp format. It
bundles `imm_lang` and `.imm` sources from the entry directory, then extracts
those sources to a temporary directory when the artifact runs.

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
| `native` | parity-gated future target tracked by `native/` |
