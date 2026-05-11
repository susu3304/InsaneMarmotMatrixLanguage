# insane marmot matrix for VS Code

VS Code language support for IMM (`.imm`) files.

## Features

- `.imm` language registration.
- Syntax highlighting for IMM keywords, types, strings, comments, numbers, declarations, and built-in functions.
- Comment, bracket, folding, and indentation rules.
- Snippets for `marmot main`, `howl marmot main`, `dig`, `probe`, `den`, `mask`, loops, and matrices.
- Commands:
  - `IMM: Check File`
  - `IMM: Run File`
  - `IMM: Run File With Trace`
  - `IMM: Format File`
  - `IMM: Run Probe`
  - `IMM: Run Law Suite`
- Save-time diagnostics through `imm check`.
- Document formatting through `imm fmt`.

## Workspace Setup

By default, the extension resolves `imm.commandPath` as `./imm` from the workspace root. In this repository that means commands work when VS Code is opened at the repo root.

To use the Rust native runtime, set:

```json
{
  "imm.useNative": true
}
```

When `imm.nativeCommandPath` is empty, native mode tries:

1. `native/imm-native/target/release/imm-native`
2. `native/imm-native/target/debug/imm-native`

## Development

```bash
cd editors/vscode/imm
npm test
npm run package
```

`npm run package` uses `@vscode/vsce` through `npx` and creates a `.vsix` package.
