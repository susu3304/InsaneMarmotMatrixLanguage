# RUST-005 - Diagnostics And Error Categories

## Goal

Define and implement native diagnostic infrastructure before runtime behavior
gets large.

## Dependencies

- RUST-003
- RUST-004

## Scope

- Error categories:
  - syntax
  - static
  - runtime
  - module
  - IO
  - network
  - pack
- Source-span rendering.
- Stable exit codes.
- Error comparison rules for parity tests.

## Diagnostic Shape

Recommended internal model:

```text
Diagnostic {
    category,
    message,
    primary_span,
    labels,
    notes,
}
```

## CLI Rendering

Initial display:

```text
main.imm:3:12: static error: wait can only be used inside howl context
```

Later display can add source snippets.

## Acceptance Criteria

- Lexer/parser errors show file, line, column, category, and message.
- Static/runtime errors can be compared by category in tests.
- Existing Python error messages do not need exact byte-for-byte parity, but
  category and broad message must match.

## Test Plan

- Syntax error location test.
- Static error category test.
- Runtime error category test.
- Module-not-found error test.

## Notes

Do not over-invest in pretty snippets before runtime parity. Precise spans and
stable categories matter more.

