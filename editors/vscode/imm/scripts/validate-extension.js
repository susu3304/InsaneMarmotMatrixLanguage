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

const semantic = extension.classifySemanticTokens(`
insane marmot main {
    let a = 10
    squeak add("insane", a)
}

dig add(a: Int, b: Int) -> Int {
    return a + b
}

dig home(ctx: WebApp) {
    return web.shiny({ "ok": true })
}
`);

function hasToken(value, type) {
  return semantic.some((token) => token.value === value && token.type === type);
}

if (!hasToken("add", "function")) {
  throw new Error("expected add to receive function semantic color");
}
if (!hasToken("main", "function")) {
  throw new Error("expected main to receive entrypoint function semantic color");
}
if (!hasToken("Int", "type")) {
  throw new Error("expected Int to receive type semantic color");
}
if (!hasToken("a", "variable")) {
  throw new Error("expected a to receive variable semantic color");
}
if (!hasToken("squeak", "function")) {
  throw new Error("expected squeak to receive builtin function semantic color");
}
if (!hasToken("WebApp", "type")) {
  throw new Error("expected WebApp to receive type semantic color");
}
if (!hasToken("shiny", "method")) {
  throw new Error("expected shiny to receive member method semantic color");
}

console.log("extension metadata ok");
