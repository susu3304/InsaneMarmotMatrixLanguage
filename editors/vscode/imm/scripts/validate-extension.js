"use strict";

const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");

for (const file of [
  "package.json",
  "language-configuration.json",
  "syntaxes/imm.tmLanguage.json",
  "snippets/imm.json",
]) {
  JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
}

const extension = require("../src/extension");
const fakeDocument = {
  fileName: "/tmp/sample.imm",
  lineCount: 3,
  lineAt(line) {
    return { text: ["marmot main {", "    @", "}"][line] || "" };
  },
};

const diagnostics = extension.parseDiagnostics(
  "/tmp/sample.imm: syntax error at 2:6: expected point after @",
  fakeDocument
);

if (diagnostics.length !== 1) {
  throw new Error("expected one parsed diagnostic");
}
if (diagnostics[0].range.start.line !== 1 || diagnostics[0].range.start.character !== 5) {
  throw new Error("expected Python diagnostic span to be converted to zero-based range");
}

const nativeDiagnostics = extension.parseDiagnostics(
  "0:2:6: syntax error: expected point after @",
  fakeDocument
);

if (nativeDiagnostics.length !== 1) {
  throw new Error("expected one native diagnostic");
}

console.log("extension metadata ok");
