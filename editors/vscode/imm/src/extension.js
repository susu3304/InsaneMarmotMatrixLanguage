"use strict";

const cp = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
let vscode;
try {
  vscode = require("vscode");
} catch {
  vscode = {
    Diagnostic: class {
      constructor(range, message, severity) {
        this.range = range;
        this.message = message;
        this.severity = severity;
      }
    },
    DiagnosticSeverity: { Error: 0 },
    Range: class {
      constructor(startLine, startCharacter, endLine, endCharacter) {
        this.start = { line: startLine, character: startCharacter };
        this.end = { line: endLine, character: endCharacter };
      }
    },
    SemanticTokensBuilder: class {
      constructor() {
        this.tokens = [];
      }
      push(line, startCharacter, length, tokenType, tokenModifiers) {
        this.tokens.push({ line, startCharacter, length, tokenType, tokenModifiers });
      }
      build() {
        return this.tokens;
      }
    },
    SemanticTokensLegend: class {
      constructor(tokenTypes, tokenModifiers) {
        this.tokenTypes = tokenTypes;
        this.tokenModifiers = tokenModifiers;
      }
    },
  };
}

const LANGUAGE_ID = "imm";
const SEMANTIC_TYPES = [
  "namespace",
  "type",
  "class",
  "interface",
  "function",
  "method",
  "parameter",
  "variable",
  "property",
  "keyword",
  "number",
  "string",
  "operator",
];
const SEMANTIC_MODIFIERS = ["declaration", "readonly", "defaultLibrary", "async"];
const SEMANTIC_LEGEND = new vscode.SemanticTokensLegend(SEMANTIC_TYPES, SEMANTIC_MODIFIERS);
const KEYWORDS = new Set([
  "marmot",
  "insane",
  "dig",
  "let",
  "stash",
  "return",
  "if",
  "else",
  "for",
  "in",
  "while",
  "break",
  "continue",
  "true",
  "false",
  "null",
  "matrix",
  "burrow",
  "use",
  "squeak",
  "sniff",
  "panic",
  "try",
  "catch",
  "tunnel",
  "choose",
  "den",
  "hatch",
  "self",
  "init",
  "fur",
  "fang",
  "mask",
  "wear",
  "under",
  "web",
  "fetch",
  "grab",
  "howl",
  "wait",
  "scatter",
  "nest",
  "nap",
  "tick",
  "pack",
  "crate",
  "pelt",
  "probe",
  "law",
  "expect",
  "trace",
]);
const TYPES = new Set([
  "Int",
  "Float",
  "Bool",
  "String",
  "Array",
  "Matrix",
  "Point",
  "Null",
  "Void",
  "Task",
  "TaskGroup",
  "Response",
  "Map",
]);
const MODULES = new Set(["core", "math", "path", "chaser", "store", "web", "tick"]);
const BUILTINS = new Set([
  "squeak",
  "sniff",
  "panic",
  "trace",
  "wait",
  "scatter",
  "nest",
  "nap",
  "expect",
  "map",
  "filter",
  "reduce",
  "len",
  "type",
  "str",
  "int",
  "float",
  "bool",
  "width",
  "height",
  "in_bounds",
  "points",
  "neighbors4",
  "neighbors8",
  "find",
  "find_all",
  "bfs",
  "astar",
  "direction",
  "step",
  "parse_field",
  "safe_moves",
  "random_move",
  "open",
  "save",
  "load",
  "all",
  "get",
  "delete",
  "count",
  "clear",
  "grab",
  "fetch",
  "now",
  "json",
  "text",
]);

let diagnostics;
let output;

function activate(context) {
  diagnostics = vscode.languages.createDiagnosticCollection("imm");
  output = vscode.window.createOutputChannel("IMM");

  context.subscriptions.push(
    diagnostics,
    output,
    vscode.commands.registerCommand("imm.checkFile", () => checkActiveDocument(true)),
    vscode.commands.registerCommand("imm.runFile", () => runActiveFile(false)),
    vscode.commands.registerCommand("imm.runFileWithTrace", () => runActiveFile(true)),
    vscode.commands.registerCommand("imm.formatFile", formatActiveDocument),
    vscode.commands.registerCommand("imm.probe", () => runWorkspaceCommand(["probe"], "IMM Probe")),
    vscode.commands.registerCommand("imm.law", () => runWorkspaceCommand(["law"], "IMM Law")),
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (document.languageId === LANGUAGE_ID && getConfig().get("checkOnSave")) {
        checkDocument(document, false);
      }
    }),
    vscode.workspace.onDidCloseTextDocument((document) => diagnostics.delete(document.uri)),
    vscode.languages.registerDocumentSemanticTokensProvider(
      { language: LANGUAGE_ID },
      {
        provideDocumentSemanticTokens(document) {
          return buildSemanticTokens(document.getText());
        },
      },
      SEMANTIC_LEGEND
    ),
    vscode.languages.registerDocumentFormattingEditProvider(LANGUAGE_ID, {
      provideDocumentFormattingEdits(document) {
        return provideFormatEdits(document);
      },
    })
  );
}

function deactivate() {}

async function checkActiveDocument(showSuccess) {
  const document = activeImmDocument();
  if (!document) {
    return;
  }
  await checkDocument(document, showSuccess);
}

async function checkDocument(document, showSuccess) {
  if (document.isUntitled) {
    vscode.window.showWarningMessage("Save this IMM file before checking it.");
    return;
  }

  const result = await runCli(["check", document.fileName], document);
  const problems = parseDiagnostics(result.stderr || result.stdout, document);
  diagnostics.set(document.uri, problems);

  if (result.code === 0) {
    if (showSuccess) {
      vscode.window.showInformationMessage("IMM check passed.");
    }
    return;
  }

  output.clear();
  output.append(result.stdout);
  output.append(result.stderr);
  output.show(true);
  vscode.window.showErrorMessage("IMM check failed.");
}

async function runActiveFile(trace) {
  const document = activeImmDocument();
  if (!document) {
    return;
  }
  if (document.isDirty) {
    await document.save();
  }
  const args = trace ? ["run", document.fileName, "--trace"] : ["run", document.fileName];
  await runWorkspaceCommand(args, trace ? "IMM Run Trace" : "IMM Run", document);
}

async function formatActiveDocument() {
  const editor = vscode.window.activeTextEditor;
  const document = activeImmDocument();
  if (!document || !editor) {
    return;
  }

  const edits = await provideFormatEdits(document);
  if (edits.length === 0) {
    return;
  }

  const workspaceEdit = new vscode.WorkspaceEdit();
  for (const edit of edits) {
    workspaceEdit.replace(document.uri, edit.range, edit.newText);
  }
  await vscode.workspace.applyEdit(workspaceEdit);
  await document.save();
}

async function provideFormatEdits(document) {
  const workspace = getWorkspaceFolder(document);
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "imm-format-"));
  const tempFile = path.join(tempDir, path.basename(document.fileName || "document.imm"));

  try {
    fs.writeFileSync(tempFile, document.getText(), "utf8");
    const result = await runCli(["fmt", tempFile], document, workspace);
    if (result.code !== 0) {
      output.clear();
      output.append(result.stdout);
      output.append(result.stderr);
      output.show(true);
      vscode.window.showErrorMessage("IMM format failed.");
      return [];
    }
    const formatted = fs.readFileSync(tempFile, "utf8");
    const fullRange = new vscode.Range(
      document.positionAt(0),
      document.positionAt(document.getText().length)
    );
    return [vscode.TextEdit.replace(fullRange, formatted)];
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

async function runWorkspaceCommand(args, title, document) {
  const workspace = getWorkspaceFolder(document);
  const cli = resolveCli(workspace);

  if (getConfig().get("runInTerminal")) {
    const terminal = vscode.window.createTerminal({
      name: title,
      cwd: workspace ? workspace.uri.fsPath : undefined,
    });
    terminal.show();
    terminal.sendText([shellQuote(cli), ...args.map(shellQuote)].join(" "));
    return;
  }

  const result = await runCli(args, document, workspace);
  output.clear();
  output.append(result.stdout);
  output.append(result.stderr);
  output.show(true);
}

function runCli(args, document, workspaceOverride) {
  const workspace = workspaceOverride || getWorkspaceFolder(document);
  const cli = resolveCli(workspace);

  return new Promise((resolve) => {
    const child = cp.spawn(cli, args, {
      cwd: workspace ? workspace.uri.fsPath : undefined,
      shell: false,
      windowsHide: true,
    });

    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", (error) => {
      resolve({ code: 127, stdout, stderr: `${stderr}${error.message}\n` });
    });
    child.on("close", (code) => {
      resolve({ code: code || 0, stdout, stderr });
    });
  });
}

function parseDiagnostics(text, document) {
  const lines = text.split(/\r?\n/).filter(Boolean);
  const parsed = [];

  for (const line of lines) {
    const diagnostic = parseDiagnosticLine(line, document);
    if (diagnostic) {
      parsed.push(diagnostic);
    }
  }

  if (parsed.length === 0 && text.trim()) {
    parsed.push(makeDiagnostic(0, 0, text.trim(), document));
  }

  return parsed;
}

function parseDiagnosticLine(line, document) {
  const pythonSpan = line.match(/^(.*?):\s+(.+?)\s+at\s+(\d+):(\d+):\s+(.+)$/);
  if (pythonSpan) {
    return makeDiagnostic(
      Number(pythonSpan[3]) - 1,
      Number(pythonSpan[4]) - 1,
      `${pythonSpan[2]}: ${pythonSpan[5]}`,
      document
    );
  }

  const nativeSpan = line.match(/^\d+:(\d+):(\d+):\s+(.+)$/);
  if (nativeSpan) {
    return makeDiagnostic(
      Number(nativeSpan[1]) - 1,
      Number(nativeSpan[2]) - 1,
      nativeSpan[3],
      document
    );
  }

  const pythonNoSpan = line.match(/^(.*?):\s+(.+)$/);
  if (pythonNoSpan) {
    const file = path.resolve(pythonNoSpan[1]);
    if (file === path.resolve(document.fileName)) {
      return makeDiagnostic(0, 0, pythonNoSpan[2], document);
    }
  }

  const nativeNoSpan = line.match(/^([a-z ]+ error):\s+(.+)$/i);
  if (nativeNoSpan) {
    return makeDiagnostic(0, 0, `${nativeNoSpan[1]}: ${nativeNoSpan[2]}`, document);
  }

  return null;
}

function makeDiagnostic(line, column, message, document) {
  const safeLine = Math.max(0, Math.min(line, document.lineCount - 1));
  const lineText = document.lineAt(safeLine).text;
  const safeColumn = Math.max(0, Math.min(column, lineText.length));
  const endColumn = Math.min(lineText.length, safeColumn + 1);
  const range = new vscode.Range(safeLine, safeColumn, safeLine, endColumn);
  const diagnostic = new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Error);
  diagnostic.source = "imm";
  return diagnostic;
}

function buildSemanticTokens(text) {
  const builder = new vscode.SemanticTokensBuilder(SEMANTIC_LEGEND);
  for (const token of classifySemanticTokens(text)) {
    builder.push(
      token.line,
      token.column,
      token.length,
      SEMANTIC_TYPES.indexOf(token.type),
      token.modifiers.reduce((bits, modifier) => {
        const index = SEMANTIC_MODIFIERS.indexOf(modifier);
        return index >= 0 ? bits | (1 << index) : bits;
      }, 0)
    );
  }
  return builder.build();
}

function classifySemanticTokens(text) {
  const tokens = scanImmTokens(text);
  const semantic = [];

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.kind === "number") {
      semantic.push(semanticToken(token, "number"));
      continue;
    }
    if (token.kind === "string") {
      semantic.push(semanticToken(token, "string"));
      continue;
    }
    if (token.kind === "operator") {
      semantic.push(semanticToken(token, "operator"));
      continue;
    }
    if (token.kind !== "identifier") {
      continue;
    }

    const prev = previousSemanticToken(tokens, index);
    const prev2 = previousSemanticToken(tokens, index, 2);
    const next = nextSemanticToken(tokens, index);
    const modifiers = [];

    if (prev && prev.value === "." && next && next.value === "(") {
      semantic.push(semanticToken(token, "method"));
      continue;
    }
    if (prev && prev.value === ".") {
      semantic.push(semanticToken(token, "property"));
      continue;
    }
    if (prev && prev.value === "dig") {
      modifiers.push("declaration");
      if (prev2 && prev2.value === "howl") {
        modifiers.push("async");
      }
      semantic.push(semanticToken(token, "function", modifiers));
      continue;
    }
    if (prev && prev.value === "marmot" && token.value === "main") {
      modifiers.push("declaration");
      if (prev2 && prev2.value === "howl") {
        modifiers.push("async");
      }
      semantic.push(semanticToken(token, "function", modifiers));
      continue;
    }
    if (prev && prev.value === "den") {
      semantic.push(semanticToken(token, "class", ["declaration"]));
      continue;
    }
    if (prev && prev.value === "mask") {
      semantic.push(semanticToken(token, "interface", ["declaration"]));
      continue;
    }
    if (prev && prev.value === "hatch") {
      semantic.push(semanticToken(token, "class"));
      continue;
    }
    if (prev && prev.value === "wear") {
      semantic.push(semanticToken(token, "interface"));
      continue;
    }
    if (prev && prev.value === "use") {
      semantic.push(semanticToken(token, "namespace"));
      continue;
    }
    if (prev && (prev.value === "let" || prev.value === "stash")) {
      modifiers.push("declaration");
      if (prev.value === "stash") {
        modifiers.push("readonly");
      }
      semantic.push(semanticToken(token, "variable", modifiers));
      continue;
    }
    if (next && next.value === ":" && !(prev && (prev.value === "let" || prev.value === "stash"))) {
      semantic.push(semanticToken(token, "parameter"));
      continue;
    }
    if (next && next.value === "=>") {
      semantic.push(semanticToken(token, "parameter"));
      continue;
    }
    if (TYPES.has(token.value)) {
      semantic.push(semanticToken(token, "type"));
      continue;
    }
    if (MODULES.has(token.value)) {
      semantic.push(semanticToken(token, "namespace", ["defaultLibrary"]));
      continue;
    }
    if (BUILTINS.has(token.value)) {
      semantic.push(semanticToken(token, "function", ["defaultLibrary"]));
      continue;
    }
    if (KEYWORDS.has(token.value)) {
      semantic.push(semanticToken(token, "keyword"));
      continue;
    }
    if (next && next.value === "(") {
      semantic.push(semanticToken(token, "function"));
      continue;
    }

    semantic.push(semanticToken(token, "variable"));
  }

  return semantic;
}

function scanImmTokens(text) {
  const tokens = [];
  let index = 0;
  let line = 0;
  let column = 0;
  let inBlockComment = false;

  while (index < text.length) {
    const char = text[index];
    const next = text[index + 1] || "";

    if (char === "\r") {
      index += next === "\n" ? 2 : 1;
      line += 1;
      column = 0;
      continue;
    }
    if (char === "\n") {
      index += 1;
      line += 1;
      column = 0;
      continue;
    }
    if (inBlockComment) {
      if (char === "*" && next === "/") {
        inBlockComment = false;
        index += 2;
        column += 2;
      } else {
        index += 1;
        column += 1;
      }
      continue;
    }
    if (char === "/" && next === "*") {
      inBlockComment = true;
      index += 2;
      column += 2;
      continue;
    }
    if (char === "#") {
      while (index < text.length && text[index] !== "\n" && text[index] !== "\r") {
        index += 1;
        column += 1;
      }
      continue;
    }
    if (/\s/.test(char)) {
      index += 1;
      column += 1;
      continue;
    }
    if (char === "\"") {
      const startColumn = column;
      index += 1;
      column += 1;
      while (index < text.length) {
        const current = text[index];
        if (current === "\n" || current === "\r") {
          break;
        }
        index += 1;
        column += 1;
        if (current === "\\") {
          index += 1;
          column += 1;
          continue;
        }
        if (current === "\"") {
          break;
        }
      }
      tokens.push({ kind: "string", value: "", line, column: startColumn, length: column - startColumn });
      continue;
    }
    if (/[0-9]/.test(char)) {
      const start = index;
      const startColumn = column;
      while (/[0-9]/.test(text[index] || "")) {
        index += 1;
        column += 1;
      }
      if (text[index] === "." && /[0-9]/.test(text[index + 1] || "")) {
        index += 1;
        column += 1;
        while (/[0-9]/.test(text[index] || "")) {
          index += 1;
          column += 1;
        }
      }
      tokens.push({
        kind: "number",
        value: text.slice(start, index),
        line,
        column: startColumn,
        length: column - startColumn,
      });
      continue;
    }
    if (/[A-Za-z_]/.test(char)) {
      const start = index;
      const startColumn = column;
      index += 1;
      column += 1;
      while (/[A-Za-z0-9_]/.test(text[index] || "")) {
        index += 1;
        column += 1;
      }
      tokens.push({
        kind: "identifier",
        value: text.slice(start, index),
        line,
        column: startColumn,
        length: column - startColumn,
      });
      continue;
    }

    const twoChar = `${char}${next}`;
    if (["=>", "->", "..", "==", "!=", "<=", ">=", "&&", "||"].includes(twoChar)) {
      tokens.push({ kind: "operator", value: twoChar, line, column, length: 2 });
      index += 2;
      column += 2;
      continue;
    }
    tokens.push({ kind: "operator", value: char, line, column, length: 1 });
    index += 1;
    column += 1;
  }

  return tokens;
}

function previousSemanticToken(tokens, index, distance = 1) {
  let seen = 0;
  for (let i = index - 1; i >= 0; i -= 1) {
    seen += 1;
    if (seen === distance) {
      return tokens[i];
    }
  }
  return null;
}

function nextSemanticToken(tokens, index) {
  return tokens[index + 1] || null;
}

function semanticToken(token, type, modifiers = []) {
  return {
    line: token.line,
    column: token.column,
    length: token.length,
    type,
    modifiers,
    value: token.value,
  };
}

function activeImmDocument() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== LANGUAGE_ID) {
    vscode.window.showWarningMessage("Open an IMM (.imm) file first.");
    return null;
  }
  return editor.document;
}

function getConfig() {
  return vscode.workspace.getConfiguration("imm");
}

function getWorkspaceFolder(document) {
  if (document && document.uri) {
    const folder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (folder) {
      return folder;
    }
  }
  return vscode.workspace.workspaceFolders ? vscode.workspace.workspaceFolders[0] : undefined;
}

function resolveCli(workspace) {
  const config = getConfig();
  const workspacePath = workspace ? workspace.uri.fsPath : process.cwd();

  if (config.get("useNative")) {
    const nativePath = config.get("nativeCommandPath");
    if (nativePath) {
      return resolvePath(nativePath, workspacePath);
    }
    for (const candidate of [
      "native/imm-native/target/release/imm-native",
      "native/imm-native/target/debug/imm-native",
    ]) {
      const resolved = resolvePath(candidate, workspacePath);
      if (fs.existsSync(resolved)) {
        return resolved;
      }
    }
  }

  return resolvePath(config.get("commandPath") || "./imm", workspacePath);
}

function resolvePath(value, workspacePath) {
  if (path.isAbsolute(value)) {
    return value;
  }
  if (value.startsWith(".")) {
    return path.resolve(workspacePath, value);
  }
  if (value.includes("/") || value.includes("\\")) {
    return path.resolve(workspacePath, value);
  }
  return value;
}

function shellQuote(value) {
  if (process.platform === "win32") {
    return `"${value.replace(/"/g, '\\"')}"`;
  }
  return `'${value.replace(/'/g, "'\\''")}'`;
}

module.exports = {
  activate,
  deactivate,
  classifySemanticTokens,
  parseDiagnostics,
  scanImmTokens,
};
