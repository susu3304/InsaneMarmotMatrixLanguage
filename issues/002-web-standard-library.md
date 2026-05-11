# 002 - Web Standard Library

## Goal

Add a built-in `web` standard library for HTTP without requiring users to install
or call external tools.

## Public API

```imm
use web

let sync_res = web.grab("https://example.com")

howl marmot main {
    let async_res = wait web.fetch("https://example.com")
}
```

### `web.grab`

```text
web.grab(url: String) -> Response
web.grab(options: Map) -> Response
```

Synchronous HTTP request. Intended for normal `marmot main`.

### `web.fetch`

```text
web.fetch(url: String) -> Task<Response>
web.fetch(options: Map) -> Task<Response>
```

Asynchronous HTTP request. Intended for `howl` contexts.

### Options

```imm
let res = web.grab({
    "method": "POST",
    "url": "https://example.com/api",
    "headers": {
        "Content-Type": "application/json"
    },
    "body": "{\"name\":\"marmot\"}",
    "timeout_ms": 3000
})
```

Supported initial fields:

| Field | Type | Default |
| --- | --- | --- |
| `method` | String | `"GET"` |
| `url` | String | required |
| `headers` | Map/String object | `{}` |
| `body` | String or Null | `null` |
| `timeout_ms` | Int | `10000` |

## Response Object

`Response` is a host-backed object with public members:

| Member | Type |
| --- | --- |
| `status` | Int |
| `headers` | Map |
| `body` | String |
| `url` | String |
| `ok` | Bool |

Methods:

```text
json() -> Any
text() -> String
```

`json()` should parse JSON using the host runtime and convert values to IMM
values. In the current Python interpreter this can use the Python `json` module.

## Error Behavior

- Invalid URL: runtime error.
- Timeout: runtime error with a clear message.
- Network failure: runtime error with host detail sanitized.
- HTTP 4xx/5xx: not an exception. Users inspect `res.status` or `res.ok`.
- Invalid JSON in `res.json()`: runtime error.

## Implementation Plan

1. Add `web` to built-in module loading.
2. Add `Response` host object support, ideally using the existing object/member
   access path instead of a special-case syntax.
3. Implement `web.grab` using Python standard library first.
4. Add option object conversion from IMM map-like data.
5. Implement response header normalization.
6. Implement `web.fetch` as a `Task<Response>` after issue 003 introduces the
   task runtime.

## Security And Runtime Policy

- Default timeout is `10000` ms.
- No implicit file downloads.
- Redirect behavior should follow host defaults initially, then be documented.
- TLS verification should stay enabled.
- Environment proxy behavior should be documented if inherited from the host.

## Acceptance Criteria

- `web.grab("https://example.com")` returns a `Response`.
- `web.grab({ "url": "...", "timeout_ms": 3000 })` honors timeout.
- `res.status`, `res.body`, `res.headers`, `res.ok`, and `res.json()` work.
- `web.fetch(...)` returns a task once the howl runtime exists.
- HTTP error status does not throw by itself.

## Test Plan

- Unit test option parsing without network.
- Unit test `Response` field/method access.
- Integration tests should use a local HTTP server, not public internet.
- Tests for timeout, invalid URL, JSON parsing, and 404 status.

## Notes For Binary Packaging

The `web` implementation must avoid dependencies that make `imm pack` fragile.
Prefer the Python standard library for the Python pelt, and a mature Rust HTTP
client for the native pelt.

