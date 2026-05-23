# insane marmot matrix for VS Code

VS Code language support for IMM (`.imm`) files.

## Features

- `.imm` language registration.
- Syntax highlighting for IMM keywords, types, strings, comments, numbers, declarations, built-in functions, and the IMM-style web server API.
- Comment, bracket, folding, and indentation rules.
- Snippets for `marmot main`, `howl marmot main`, `dig`, `probe`, `den`, `mask`, loops, matrices, and `web.den` servers.
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

By default, `imm.commandPath` is `auto`. The extension searches for a nearby
`imm` executable from the current `.imm` file and workspace root, then falls
back to `imm` on `PATH`.

If save-time diagnostics report that the CLI cannot be found, set an absolute
path:

```json
{
  "imm.commandPath": "/path/to/imm"
}
```

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
