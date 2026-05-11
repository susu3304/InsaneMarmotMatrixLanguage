# IMM Web Standard Library

`web` is a built-in namespace for HTTP and URL-backed responses.

```imm
use web

let res = web.grab("https://example.com")
```

## API

- `web.grab(url: String) -> Response`
- `web.grab(options: Map) -> Response`
- `web.fetch(url: String) -> Task<Response>`
- `web.fetch(options: Map) -> Task<Response>`

Options use a map with these fields:

| Field | Type | Default |
| --- | --- | --- |
| `method` | `String` | `"GET"` |
| `url` | `String` | required |
| `headers` | `Map<String>` | `{}` |
| `body` | `String` or `Null` | `null` |
| `timeout_ms` | `Int` | `10000` |

`Response` exposes `status`, `headers`, `body`, `url`, `ok`, `text()`, and
`json()`. HTTP 4xx/5xx responses return a `Response`; network failures,
timeouts, invalid URLs, and invalid JSON raise runtime errors.
