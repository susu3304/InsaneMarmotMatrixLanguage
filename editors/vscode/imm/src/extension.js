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
  };
}

const LANGUAGE_ID = "imm";

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
  parseDiagnostics,
};
