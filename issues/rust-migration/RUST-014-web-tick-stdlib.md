# RUST-014 - Web And Tick Stdlib

## Goal

Port `web` and `tick` to Rust with behavior matching the Python reference.

## Dependencies

- RUST-007
- RUST-010
- RUST-015 can be parallel for async pieces, but `web.grab` can land first.

## Scope

### `tick`

```text
tick.now() -> Int
```

Returns UNIX milliseconds.

### `web`

```text
web.grab(url: String) -> Response
web.grab(options: Map) -> Response
web.fetch(url: String) -> Task<Response>
web.fetch(options: Map) -> Task<Response>
```

### Response

- `status`
- `headers`
- `body`
- `url`
- `ok`
- `json()`
- `text()`

## HTTP Rules

- Default `timeout_ms = 10000`.
- HTTP 4xx/5xx returns a `Response`, not an exception.
- Invalid URL is an error.
- Timeout is an error.
- TLS verification remains enabled.
- Tests should use local HTTP servers or `data:` URLs where supported.

## Acceptance Criteria

- Native `web.grab` passes current web law.
- Native `web.fetch` returns `Task<Response>` after RUST-015.
- JSON conversion returns IMM values.
- Header map is accessible.

## Test Plan

- Data URL response if the chosen Rust client supports it. If not, implement a
  small data URL branch to keep law tests deterministic.
- Local HTTP GET.
- Local HTTP POST with headers/body.
- 404 response test.
- Timeout test.

## Risks

- `reqwest` does not handle `data:` URLs like Python `urllib` does. Add explicit
  `data:` handling in IMM stdlib to preserve deterministic tests.

