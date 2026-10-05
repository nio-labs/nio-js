# nio-js

A working preview of a Rust-hosted QuickJS service runtime. Write JavaScript or TypeScript, import compatible ESM by URL, and package code and assets into a portable `.njs` capsule.

```typescript
import { get } from 'nio.js'
get('/', 'Hello World')
```

The engine is embedded through `rquickjs` 0.14, which currently bundles the QuickJS-NG fork. Node and npm are not needed to build or execute this project. Building requires Rust 1.96+ and a C toolchain.

## Run

After a release is published, install the standalone CLI (macOS, Linux, or Termux):

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio-js/main/install.sh | sh
nio-js run app.ts
```

The installer detects your platform, downloads the latest stable release, verifies its SHA-256 checksum, and installs to `$HOME/.local/bin` (or `$PREFIX/bin` in Termux). It requires curl or wget and sha256sum or shasum. Add the printed directory to PATH if needed. Node and root access are unnecessary.

For a specific version or directory, download the script and pass options:

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio-js/main/install.sh -o install.sh
sh install.sh --version v0.1.0 --install-dir "$HOME/.local/bin"
```

Linux releases require GNU libc (built on Ubuntu 22.04); musl distributions are not supported. Windows and npm users can use `npm install -g @nio-labs/nio-js` or `npx @nio-labs/nio-js run app.ts`. The npm launcher requires Node 18+. Standalone binaries are also available from [GitHub Releases](https://github.com/nio-labs/nio-js/releases). Installation downloads become available once the first release is published.

Maintainers: pushing a `v*` version tag publishes to npm and GitHub after validation. To publish an existing tag manually, use [Actions → Publish to npm](https://github.com/nio-labs/nio-js/actions/workflows/publish.yml). Configure the repository secret `NPM_TOKEN` first, or configure npm trusted publishers for both `release.yml` and `publish.yml` on all seven packages.

Android ARM64 binaries are included for a Termux preview. Run `pkg install curl coreutils`, then use the installer above; it installs into `$PREFIX/bin`. Real ARM64 device validation is still required.

To build from source, run from this directory and install the compiled executable:

```bash
cargo build --release --locked
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/nio-js "$HOME/.local/bin/nio-js"
export PATH="$HOME/.local/bin:$PATH"
nio-js run examples/server.ts --port 3000
```

Visit `http://localhost:3000/`. The server binds to `127.0.0.1` by default. Use `--host 0.0.0.0` to listen on other interfaces.

The sample includes JSON, query parameters, parameterized routes, POST JSON, and multipart uploads. Scripts without registered routes execute and exit.

```bash
curl 'http://localhost:3000/greeting?name=Nio'
curl -X POST http://localhost:3000/echo \
  -H 'Content-Type: application/json' -d '{"hello":"world"}'
curl -F file=@examples/assets/hello.txt http://localhost:3000/upload
```

TypeScript is transformed with Oxc. This is not type checking. Include [types/nio.d.ts](types/nio.d.ts) in your editor's TypeScript project; DOM declarations describe the familiar web object types, but runtime support is a subset.

## Production deployment

Build an application capsule and run it on your server:

```sh
nio-js build app.ts -o app.njs
nio-js verify app.njs
nio-js run app.njs --host 127.0.0.1 --port 3000
```

Pin the runtime version and dependency URLs, commit `nio.lock`, and use `--frozen` for subsequent builds (`--offline --frozen` when dependencies are cached). Deploy the verified capsule to your server and run it under a process supervisor such as systemd. Keep the service bound to loopback behind an HTTPS reverse proxy, check an application health route after deployments, and retain the previous capsule and runtime version for rollback. Capsule execution requires neither source files nor the dependency cache.

## Capsules

```bash
nio-js build examples/server.ts -o server.njs
nio-js inspect server.njs
nio-js verify server.njs
nio-js run server.njs --port 3000
```

Capsules are versioned JSON documents containing JavaScript modules, dependency edges, per-object SHA-256 digests, source maps, assets, and required network destinations. They contain portable source, not engine bytecode. Execution requires neither the original source nor the dependency cache and performs no dependency downloads.

`verify` checks structure, graph completeness, compatibility, and object digests. It does not authenticate the publisher: artifact distribution still needs a trusted digest or signature. Source maps include original source content; do not place secrets in source or packaged assets.

## Dependencies from esm.sh and UNPKG

```typescript
import { get } from 'nio.js'
import { z } from 'https://esm.sh/zod@3.23.8?target=es2022'
get('/', () => z.string().parse('Hello World'))
```

Raw ESM works too:

```typescript
import { z } from 'https://unpkg.com/zod@3.23.8/lib/index.mjs'
```

```bash
nio-js build examples/cdn.ts -o cdn.njs
nio-js build examples/unpkg.ts --update -o unpkg.njs
nio-js build examples/cdn.ts --offline --frozen -o cdn.njs
```

Preparation allows `esm.sh`, `unpkg.com`, and `esm.unpkg.com` by default. Add another exact hostname with `--allow-import example.com`. Redirect destinations must also be allowed. Private/reserved network addresses are denied for module downloads.

A first successful preparation creates `nio.lock` beside the entrypoint. Existing locked objects are verified and reused. New remote dependencies require `--update` when a lock already exists; updating intentionally accepts newly fetched bytes. `--frozen` requires an existing lock. `--offline` requires every remote source in the verified cache.

The shared source cache defaults to `$HOME/.cache/nio-js`, with `NIO_JS_CACHE` or `--cache PATH` overrides. Emitted JavaScript and source maps are prepared during each build; a persistent transformed-code cache is future work.

Bare imports other than the reserved `'nio.js'` require an explicit import map:

```json
{
  "imports": {
    "zod": "https://esm.sh/zod@3.23.8?target=es2022"
  }
}
```

```bash
nio-js build server.ts --import-map imports.json -o server.njs
```

Local imports are confined to the entrypoint's directory after canonicalization, including symlink resolution. HTTPS relative imports resolve against the final redirected URL. Literal dynamic imports are prepared; computed dynamic imports, import attributes, Node built-ins, native addons, and automatic CommonJS execution are rejected.

## Application API

```typescript
import { get, post, reply } from 'nio.js'

get('/users/:id', async ({ params }) => ({ id: params.id }))
post('/echo', async ({ json }) => reply(await json(), {
  status: 201,
  headers: { 'x-service': 'nio-js' },
}))
```

- `get` and `post` register exact or `:parameter` routes before startup.
- Strings return text. Plain objects/arrays return JSON. Blob/File returns bytes. Response is accepted.
- `reply` controls status and headers. Unsupported return values fail clearly.
- Constant string, Blob, and explicit reply handlers are captured at startup and served by Rust.
- Unmatched paths return 404; unmatched methods return 405 with Allow.
- `query` exposes the first value per key. `searchParams.getAll()` exposes repeated values.
- `text()`, `json()`, `blob()`, and `formData()` consume the request body once.
- Handler errors return generic responses with request IDs; source-mapped details go to stderr.

JSON parse errors currently become handler errors unless the handler catches them. An uncaught handler failure or deadline recycles the runtime, resetting in-memory application state. Constant routes remain available while a callback is executing.

## Packaged assets

```typescript
import { get, asset, redirect } from 'nio.js'
get('/', asset('hello.txt'))
get('/docs', redirect('https://example.com/docs'))
```

```bash
nio-js build examples/assets.ts \
  --asset hello.txt=examples/assets/hello.txt -o assets.njs
nio-js run assets.njs
```

Assets are read only during preparation and embedded with digests. Applications have no general filesystem API. Native directory mounts are deferred.

## File API subset

The preview supplies Blob/File text, binary, slicing, and chunk-reader operations, plus FormData and multipart parsing. Browser uploads send bytes and metadata; server-side File objects are constructed separately.

Uploads, outbound responses, and HTTP responses are bounded and buffered in this preview. `Blob.stream()` offers a chunk-reader/async-iterator subset over buffered bytes; it is not a complete WHATWG ReadableStream or an end-to-end streaming transport. Large disk-backed uploads and full stream backpressure are future work.

Selected interoperability APIs include UTF-8 TextEncoder/TextDecoder, read-oriented URL/URLSearchParams/Headers, Response, console, basic timers, and outbound GET fetch. These are intentionally incomplete web API implementations. Timer and host completion jobs are driven during module evaluation and awaited handler execution; detached background tasks are not supported. No DOM, browser file picker, FileReader, AbortController, or full browser compatibility is claimed.

## Permissions and limits

Application networking is denied by default, separately from module preparation:

```bash
nio-js run server.njs --allow-net api.example.com
```

A hostname grant permits that host's HTTP/HTTPS ports; `host:port` restricts it to that port. Every redirect is checked. DNS addresses are checked and pinned for each connection; private/reserved destinations require both an explicit destination grant and `--allow-private-network`. Proxy environment variables are ignored.

Outbound fetch currently supports GET with no custom headers, body, or cancellation signal. Each response uses the configured byte limit. Outstanding operations are bounded; a host operation may finish after JavaScript times out, especially during native DNS resolution.

Declare required hosts in a capsule:

```bash
nio-js build server.ts --require-net api.example.com -o server.njs
nio-js run server.njs --allow-net api.example.com
```

Startup fails when declared destinations lack operator grants. Declared requirements are explicit; they are not inferred from arbitrary code.

Defaults:

| Limit | Default |
|---|---|
| QuickJS allocation budget | 64 MiB (`--memory-mb`) |
| JavaScript evaluation/handler deadline | 1 second (`--timeout-ms`) |
| Request, response, and outbound body | 1 MiB (`--max-body`) |
| Active HTTP requests | 8 |
| Queued callback jobs | 8 |
| Pending host operations per runtime | 8 |
| Active outbound operations across runtime recycling | 8 |
| Multipart fields | 32 |
| Module count | 256 |
| Individual source or asset | 4 MiB |
| Serialized capsule | 32 MiB |

Callbacks execute serially on one dedicated QuickJS thread, with two threads for HTTP I/O. Synchronous handlers avoid promise scheduling; asynchronous handlers and thenables drive the job queue. Both finish a microtask checkpoint within the invocation deadline. Request headers and search parameters are constructed when first accessed. Text/JSON response strings and binary buffers pass directly to Rust without a base64/JSON response envelope. Exact routes are indexed, and constant response headers and bytes are cached.

Queue waiting has a separate bounded timeout. Request-body reads have a five-second deadline. Shutdown stops accepting requests and drains within a bounded window.

The QuickJS budget does not include every host allocation. Host buffers have explicit limits, but this preview is not a hardened sandbox for hostile code. Multi-worker execution, invocation identity, per-principal authorization, whole-process accounting, and OS isolation are future work.

## Validation

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
```

Run the optional live-CDN test with public network access:

```bash
cargo test --locked --test service cdn_imports_execute_then_rebuild_offline -- --ignored
```

Tests exercise HTTP behavior, uploads, source-to-capsule execution after deleting sources, tamper detection, capabilities, limits, failure recovery, source maps, and live esm.sh/UNPKG dependencies with offline rebuilding.

Only macOS ARM64 has been exercised in this workspace. Linux, Windows, Termux, and experimental iSH require separate builds and device validation. A local Node/Bun comparison and reproducible benchmark suite are in [benchmarks/RESULTS.md](benchmarks/RESULTS.md) and [benchmarks/README.md](benchmarks/README.md). Those measurements cover one machine and four small workloads.

## Scope

Nio and nio-db are unchanged. The service launcher, database bindings, AI tools, durable workflows, incident replay, permission diffs, native filesystem mounts, and full standards compliance are deferred. The wider design is in [NIOJS.md](NIOJS.md).
