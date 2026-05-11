# RUST-003 - Lexer With Source Spans

Status: Done. Operationally closed by the Python-free Rust evaluator and verified by the native parity gate.

## Goal

Implement the Rust lexer/tokenizer with source spans and keyword parity with the
Python tokenizer.

## Dependencies

- RUST-001

## Scope

- UTF-8 source handling.
- LF/CRLF normalization.
- Single-line and block comments.
- Numeric literals.
- String literals and escapes.
- Identifiers.
- Symbols and operators.
- All current reserved words.
- Source spans for diagnostics.

## Required Keywords

```text
marmot insane dig let stash return if else for in while break continue
true false null matrix burrow use squeak sniff panic try catch tunnel
den hatch self init fur fang mask wear under
web fetch grab howl wait scatter nest nap tick pack crate pelt
probe law expect trace
```

`entry` remains contextual inside `pack`.

## Token Span Model

Each token should carry:

```text
file_id
byte_start
byte_end
line
column
```

Line and column are for display. Byte offsets are for robust slicing.

## Acceptance Criteria

- Rust lexer tokenizes all current examples.
- Rust lexer tokenizes all law files.
- Comments are discarded without disturbing line numbers.
- String escapes match Python behavior.
- Unterminated string and unterminated block comment produce syntax errors with
  line/column.

## Test Plan

- Snapshot token tests for representative programs.
- Keyword-vs-identifier tests.
- CRLF input test.
- String escape tests for `\n`, `\t`, `\"`, and `\\`.
- Error tests for bad string/comment.

## Notes

Do not use regex-only lexing for everything. IMM has block comments, strings,
and source spans; a small hand-written scanner is easier to control.
