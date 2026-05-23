# IMM Web Standard Library

`web` is a built-in namespace for HTTP clients, responses, and IMM-native web
servers.

```imm
use web

let res = web.grab("https://example.com")
```

## API

- `web.grab(url: String) -> Response`
- `web.grab(options: Map) -> Response`
- `web.fetch(url: String) -> Task<Response>`
- `web.fetch(options: Map) -> Task<Response>`
- `web.den() -> WebApp`
- `web.burrow() -> WebApp`
- `web.release(den: WebApp, options: Map) -> Null`
- `web.peek(den: WebApp, options: Map) -> Server`
- `web.listen(den: WebApp, options: Map) -> Server`
- `web.response(options: Map) -> Response`
- `web.squeak(body: String, status?: Int) -> Response`
- `web.html(body: String, status?: Int) -> Response`
- `web.shiny(value: Any, status?: Int) -> Response`
- `web.pelt(body: String | Array<Int>, status?: Int) -> Response`
- `web.scurry(url: String, status?: Int) -> Response`
- `web.null(status?: Int) -> Response`
- `web.lost(message?: String) -> Response`
- `web.panic(message?: String) -> Response`
- `web.trace() -> Middleware`
- `web.recover() -> Middleware`
- `web.cors(options?: Map) -> Middleware`
- `web.limit(bytes: Int) -> Middleware`

Plain aliases are also available: `web.app`, `web.router`, `web.text`,
`web.json`, `web.bytes`, `web.redirect`, and `web.empty`.

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

## Web Server

IMM names are the primary API:

```imm
use web

dig home(ctx) {
    return web.html("<h1>Hello insane marmot matrix</h1>")
}

dig show_user(ctx) {
    return web.shiny({
        "id": ctx.paws["id"],
        "trail": ctx.trail
    })
}

marmot main {
    let den = web.den()
    let api = web.burrow()

    den.wear(web.trace())
    den.wear(web.cors({ "origin": "*" }))

    den.sniff("/", home)
    api.sniff("/users/:id", show_user)
    den.dig("/api", api)

    web.release(den, {
        "host": "127.0.0.1",
        "port": 8080
    })
}
```

### WebApp / Burrow

- `den.sniff(path, handler)` handles `GET`.
- `den.stash(path, handler)` handles `POST`.
- `den.replace(path, handler)` handles `PUT`.
- `den.patch(path, handler)` handles `PATCH`.
- `den.erase(path, handler)` handles `DELETE`.
- `den.ask(path, handler)` handles `OPTIONS`.
- `den.nod(path, handler)` handles `HEAD`.
- `den.any(path, handler)` handles any HTTP method.
- `den.wear(middleware)` adds middleware.
- `den.dig(prefix, burrow)` mounts another `WebApp`.
- `den.hoard(prefix, directory)` serves static files.
- `den.lost(handler)` registers the 404 handler.
- `den.rescue(handler)` registers the error handler.

Plain aliases are available for routing: `get`, `post`, `put`, `delete`,
`use`, `mount`, `static`, `not_found`, and `error`.

Route patterns support exact segments, `:params`, and trailing `*wildcards`.

### Context

- `ctx.req` is the `Request`.
- `ctx.paws` / `ctx.params` contains path parameters.
- `ctx.trail` / `ctx.query` contains query parameters.
- `ctx.pouch` / `ctx.state` is per-request mutable state.
- `ctx.next()` returns `null`; middleware returning `null` continues.

### Request

- `req.method`, `req.path`, `req.headers`, `req.cookies`, and `req.remote`.
- `req.trail` / `req.query`.
- `req.sniff()` / `req.text()` returns the body as text.
- `req.crack()` / `req.json()` parses the body as JSON.
- `req.form()` parses URL-encoded form data.
- `req.pelt()` / `req.bytes()` returns `Array<Int>`.

### Server

`web.release` blocks and serves until the process is stopped. `web.peek` and
`web.listen` return a server handle for tests and embedded workflows:

```imm
let server = web.peek(den, { "port": 0 })
let res = web.grab(server.url + "/")
server.stop()
```

Server fields and methods:

- `server.host`, `server.port`, `server.url`, `server.running`.
- `server.stop() -> Bool`.
- `server.closed() -> Task<Null>`.

### Server Options

| Field | Type | Default |
| --- | --- | --- |
| `host` | `String` | `"127.0.0.1"` |
| `port` | `Int` | `8080` |
| `workers` | `Int` | `1` |
| `max_body_bytes` | `Int` | `1048576` |
| `request_timeout_ms` | `Int` | `10000` |

Current server support is HTTP/1.1 over TCP. TLS, WebSocket, and SSE names are
reserved for the web API but are not implemented yet.
