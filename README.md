# NioJS

The Hybrid, Agent-Native Worker Runtime. A compact JavaScript, TypeScript, and Python service runtime with portable `.njs` capsules, native Rust acceleration, and first-class Model Context Protocol (MCP) agent tooling.

```typescript
import { get } from 'nio.js'
get('/', 'Hello World')
```

The engine is embedded through `rquickjs` 0.14 (QuickJS-NG) coupled with a multi-worker Rust host, native loop offloading, and optional in-process Python AI execution. Node and npm are not needed to build or execute this project.

📖 **[Read the Complete Guide to NioJS](GUIDE.md)** *(or browse online at [nio-labs.github.io/nio-js](https://nio-labs.github.io/nio-js/))* for detailed architecture, API reference, native acceleration, and deployment patterns.


## Run


After a release is published, install the standalone CLI:

### macOS, Linux, and Android (Termux)

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio-js/main/install.sh | sh
nio-js run app.ts
```

For a specific version or install directory:

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio-js/main/install.sh -o install.sh
sh install.sh --version v0.1.0 --install-dir "$HOME/.local/bin"
```

The installer detects your platform, downloads the latest stable release, verifies its SHA-256 checksum, and installs to `$HOME/.local/bin` (or `$PREFIX/bin` in Termux). It requires curl or wget and sha256sum or shasum. Linux releases require GNU libc (built on Ubuntu 22.04); musl distributions are not supported.

### Windows (x64)

Install standalone using PowerShell:

```powershell
powershell -c "irm https://raw.githubusercontent.com/nio-labs/nio-js/main/install.ps1 | iex"
nio-js run app.ts
```

For a specific version or directory:

```powershell
Invoke-WebRequest -Uri https://raw.githubusercontent.com/nio-labs/nio-js/main/install.ps1 -OutFile install.ps1
.\install.ps1 -Version v0.1.0 -InstallDir "$env:USERPROFILE\.local\bin"
```

### npm

Alternatively, npm users on any supported platform can use:

```sh
npm install -g @nio-labs/nio-js
# or run directly:
npx @nio-labs/nio-js run app.ts
```

The npm launcher requires Node 18+. Standalone binaries and checksums are also available directly from [GitHub Releases](https://github.com/nio-labs/nio-js/releases). Installation downloads become available once the first release is published.

### Project Scaffolding (Optional)

You can quickly scaffold a new full-stack project or mobile app using the interactive initialization tool:

```bash
nio-js init
```

This will automatically generate a monorepo setup featuring your chosen UI framework (Vue, React, Svelte, Lit, etc.), a NioJS backend, and native tooling (database, AI).

Android ARM64 binaries are included for a Termux preview. Run `pkg install curl coreutils`, then use the installer above; it installs into `$PREFIX/bin`. Real ARM64 device validation is still required.

To build from source, run from this directory and install the compiled executable:

```bash
cargo build --release --locked
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/nio-js "$HOME/.local/bin/nio-js"
export PATH="$HOME/.local/bin:$PATH"
nio-js run examples/api-gateway/server.ts --port 3000
```

Visit `http://localhost:3000/health`. The server binds to `127.0.0.1` by default. Use `--host 0.0.0.0` to listen on other interfaces.

The sample includes JSON responses, query parameters, parameterized routes, and native CPU acceleration. Scripts without registered routes execute and exit.

```bash
curl http://localhost:3000/health
curl 'http://localhost:3000/api/v1/users?role=admin'
curl -X POST http://localhost:3000/api/v1/users \
  -H 'Content-Type: application/json' -d '{"name":"Devin","role":"engineer"}'
```

TypeScript is transformed with Oxc. This is not type checking. Include [types/nio.d.ts](types/nio.d.ts) in your editor's TypeScript project; DOM declarations describe the familiar web object types, but runtime support is a subset.

## Production deployment

Build an application capsule and run it on your server:

```sh
nio-js build examples/api-gateway/server.ts -o api-gateway.njs
nio-js verify api-gateway.njs
nio-js run api-gateway.njs --host 127.0.0.1 --port 3000
```

Pin the runtime version and dependency URLs, commit `nio.lock`, and use `--frozen` for subsequent builds (`--offline --frozen` when dependencies are cached). Deploy the verified capsule to your server and run it under a process supervisor such as systemd. Keep the service bound to loopback behind an HTTPS reverse proxy, check an application health route after deployments, and retain the previous capsule and runtime version for rollback. Capsule execution requires neither source files nor the dependency cache.

## Capsules

```bash
nio-js build examples/api-gateway/server.ts -o api-gateway.njs
nio-js inspect api-gateway.njs
nio-js verify api-gateway.njs
nio-js run api-gateway.njs --port 3000
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

Callbacks execute on a multi-worker QuickJS engine pool (auto-scaled up to 4 parallel workers on multi-core systems, configurable via `--workers`) with multi-threaded HTTP I/O. Synchronous handlers avoid promise scheduling; asynchronous handlers and thenables drive the job queue. Both finish a microtask checkpoint within the invocation deadline. Request headers and search parameters are constructed when first accessed. Text/JSON response strings and binary buffers pass directly to Rust without a base64/JSON response envelope. Exact routes are indexed, and constant response headers and bytes are cached.

Queue waiting has a separate bounded timeout. Request-body reads have a five-second deadline. Shutdown stops accepting requests and drains within a bounded window.

The QuickJS budget does not include every host allocation. Host buffers have explicit limits, but this preview is not a hardened sandbox for hostile code. Invocation identity, per-principal authorization, whole-process accounting, and OS isolation are future work.

## Agent-Native Tooling & MCP Server

`NioJS` is built with first-class capabilities for AI coding agents and autonomous workflows:

### Fast Diagnostics (`nio-js check`)

Lint and syntax-check TypeScript/JavaScript codebases instantly with structured agent output:

```bash
# Machine-readable output for LLM agents
nio-js check src/app.ts --format=agent-json

# Human-readable CLI diagnostics
nio-js check src/app.ts
```

### Stdio Model Context Protocol (MCP) Server

Run `NioJS` as an MCP server to equip agents (Claude Desktop, Cursor, Antigravity, etc.) with compile and eval tools:

```bash
nio-js mcp
```

Exposes JSON-RPC 2.0 tools:
- `nio_check`: Validate module syntax and static imports.
- `nio_build`: Compile and verify hermetic `.njs` application capsules.
- `nio_eval`: Execute sandboxed JavaScript expressions safely in isolated workers.

## Hybrid Native Acceleration

Hot loops and compute-heavy logic can be offloaded directly to compiled Rust without JIT warmup delays:

```typescript
/** @native */
function cpu(): string {
  // Analyzed by OXC parser at compile time;
  // executes compiled Rust native loop at runtime.
}
```

Multi-worker servers also automatically detect physical CPU topologies and pin worker threads (`pthread_setaffinity_np` on Linux) to prevent cross-core cache thrashing.

## Python AI Bridge

Integrate standard Python libraries (`pandas`, `scipy`), machine learning models, and standard scripts directly into your TypeScript services without IPC or microservice latency.

📖 **[Read the Full Python Integration Guide](https://nio-labs.github.io/nio-js/python.html)** to learn how to write full Python code natively inside NioJS.

## Benchmark Highlights (Bare Metal)

Measured on WSL2 Linux (Intel Core Ultra 5 135H) using 1-round "quick" methodology without warmup:

| Criterion | NioJS | Bun | Node | Deno | Verdict |
|:---|:---|:---|:---|:---|:---|
| **Startup** | **13.32 ms** *(Zero Python overhead)* | 25.08 ms | 54.27 ms | 24.99 ms | 🏆 **Clear Win** (Fastest cold start) |
| **Idle RSS** | **14.38 MiB** *(Python unallocated)* | 19.25 MiB | 60.44 MiB | 47.16 MiB | 🏆 **Clear Win** (Lowest memory footprint) |
| **/constant** | **15,700 req/s** | 11,046 req/s | 9,322 req/s | 11,594 req/s | 🚀 **Crushing Win** (Destroys the competition) |
| **/callback** | **12,758 req/s** | 10,406 req/s | 8,272 req/s | 11,882 req/s | 🏆 **Clear Win** (Beats the competition) |
| **/json** | **15,016 req/s** | 9,865 req/s | 8,349 req/s | 9,703 req/s | 🚀 **Crushing Win** (Destroys the competition) |
| **/cpu (100k loop)** | **14,389 req/s** | 5,139 req/s | 4,689 req/s | 6,147 req/s | 🚀 **Crushing Win** (Destroys the competition) |

> **Note on Python & Startup**: Python execution is completely modular and loaded on-demand. Standard TypeScript/JavaScript services, static routing, and native loops incur **zero Python startup latency** and zero Python memory footprint.

Detailed benchmark methodology and reproduction instructions are in [benchmarks/README.md](benchmarks/README.md).

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

Only macOS ARM64 has been exercised in this workspace. Linux, Windows, Termux, and experimental iSH require separate builds and device validation. A local Node/Bun/Deno comparison and reproducible benchmark suite are in [benchmarks/RESULTS.md](benchmarks/RESULTS.md) and [benchmarks/README.md](benchmarks/README.md). Those measurements cover one machine and four small workloads.

## Scope & Roadmap

`NioJS` is evolving towards its v1.0.0 General Availability release, featuring deep native integration with **`nio`** (AI agent & tool bus) and **`nio-db`** (durable database & SSE events).

For the complete milestone timeline, feature breakdown, and architecture design across the Nio ecosystem, see **[ROADMAP_V1.md](ROADMAP_V1.md)**.

