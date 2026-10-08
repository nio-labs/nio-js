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
sh install.sh --version v0.2.7 --install-dir "$HOME/.local/bin"
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
.\install.ps1 -Version v0.2.7 -InstallDir "$env:USERPROFILE\.local\bin"
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

This will automatically generate a monorepo setup featuring your chosen UI framework (Vue, React, Svelte, Lit, etc.), a NioJS backend, and native tooling (database, AI). You can also run `nio-js init server` to create a backend-only project.

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


## Benchmarks

### Compute & Throughput Microbenchmarks

<div align="center" style="overflow-x: auto;">
<table>
  <thead>
    <tr>
      <th nowrap>Workload</th>
      <th align="right" nowrap>NioJS ops/s</th>
      <th align="right" nowrap>NioJS (@native)</th>
      <th align="right" nowrap>NioJS (Rust)</th>
      <th align="right" nowrap>Node ops/s</th>
      <th align="right" nowrap>Bun ops/s</th>
      <th align="right" nowrap>Deno ops/s</th>
      <th align="center" nowrap>Verdict</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td nowrap>HTTP GET throughput</td>
      <td align="right" nowrap>8,307,028</td><td align="right" nowrap>8,335,973</td><td align="right" nowrap>39,594,551</td>
      <td align="right" nowrap>17,869,907</td><td align="right" nowrap>11,718,658</td><td align="right" nowrap>14,149,075</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.parse small payload</td>
      <td align="right" nowrap>508,849</td><td align="right" nowrap>525,243</td><td align="right" nowrap>4,629,634,300</td>
      <td align="right" nowrap>1,252,496</td><td align="right" nowrap>1,400,407</td><td align="right" nowrap>1,603,489</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.parse large payload</td>
      <td align="right" nowrap>1,096</td><td align="right" nowrap>1,227</td><td align="right" nowrap>180,179,186</td>
      <td align="right" nowrap>4,700</td><td align="right" nowrap>5,776</td><td align="right" nowrap>5,522</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.stringify small object</td>
      <td align="right" nowrap>240,563</td><td align="right" nowrap>216,539</td><td align="right" nowrap>4,132,255,600</td>
      <td align="right" nowrap>1,434,033</td><td align="right" nowrap>1,755,692</td><td align="right" nowrap>2,933,480</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.stringify medium object</td>
      <td align="right" nowrap>22,560</td><td align="right" nowrap>22,745</td><td align="right" nowrap>1,724,138,645</td>
      <td align="right" nowrap>235,707</td><td align="right" nowrap>315,734</td><td align="right" nowrap>298,536</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>SHA 256 hashing small buffer</td>
      <td align="right" nowrap>200,938</td><td align="right" nowrap>202,245</td><td align="right" nowrap>1,398,593,026</td>
      <td align="right" nowrap>10,314,595</td><td align="right" nowrap>1,214,595</td><td align="right" nowrap>5,693,464</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>SHA 256 hashing large buffer</td>
      <td align="right" nowrap>3,145</td><td align="right" nowrap>3,205</td><td align="right" nowrap>1,091,584</td>
      <td align="right" nowrap>68,254</td><td align="right" nowrap>303,297</td><td align="right" nowrap>75,835</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>Buffer copy 64 KB</td>
      <td align="right" nowrap>418,274</td><td align="right" nowrap>423,970</td><td align="right" nowrap>466,999</td>
      <td align="right" nowrap>50,307</td><td align="right" nowrap>95,171</td><td align="right" nowrap>44,750</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>Array map plus reduce</td>
      <td align="right" nowrap>991</td><td align="right" nowrap>1,001</td><td align="right" nowrap>222,433</td>
      <td align="right" nowrap>9,376</td><td align="right" nowrap>21,280</td><td align="right" nowrap>8,516</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>String concatenation</td>
      <td align="right" nowrap>8,501</td><td align="right" nowrap>8,019</td><td align="right" nowrap>299,208</td>
      <td align="right" nowrap>136,057</td><td align="right" nowrap>254,140</td><td align="right" nowrap>154,459</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>Integer loop plus arithmetic</td>
      <td align="right" nowrap>1,401</td><td align="right" nowrap>9,428</td><td align="right" nowrap>3,314,400</td>
      <td align="right" nowrap>32,809</td><td align="right" nowrap>30,647</td><td align="right" nowrap>32,137</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>Integer loop with randomized input</td>
      <td align="right" nowrap>1,737</td><td align="right" nowrap>1,728</td><td align="right" nowrap>3,329,979</td>
      <td align="right" nowrap>32,722</td><td align="right" nowrap>31,086</td><td align="right" nowrap>31,980</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
  </tbody>
</table>
</div>

### Benchmark Highlights (Bare Metal HTTP)

*Measured on macOS ARM64 using 1-round "quick" methodology without warmup:*

<div align="center" style="overflow-x: auto;">
<table>
  <thead>
    <tr>
      <th nowrap>Criterion</th>
      <th nowrap>NioJS (Base)</th>
      <th nowrap>NioJS (@native)</th>
      <th nowrap>NioJS (Rust)</th>
      <th nowrap>Bun</th>
      <th nowrap>Node</th>
      <th nowrap>Deno</th>
      <th nowrap>Verdict</th>
    </tr>
  </thead>
  <tbody>
    <tr><td nowrap><strong>Startup</strong></td><td nowrap><strong>7.3 ms</strong></td><td nowrap>7.3 ms</td><td nowrap>14.5 ms</td><td nowrap>12.2 ms</td><td nowrap>55.2 ms</td><td nowrap>18.8 ms</td><td nowrap>🏆 Clear Win (NioJS Base)</td></tr>
    <tr><td nowrap><strong>Idle RSS</strong></td><td nowrap><strong>11.6 MiB</strong></td><td nowrap>11.6 MiB</td><td nowrap>13.8 MiB</td><td nowrap>13.3 MiB</td><td nowrap>46.8 MiB</td><td nowrap>34.6 MiB</td><td nowrap>🏆 Clear Win (NioJS Base)</td></tr>
    <tr><td nowrap><strong>/constant</strong></td><td nowrap><strong>73,279 req/s</strong></td><td nowrap><strong>73,279 req/s</strong></td><td nowrap><strong>73,279 req/s</strong></td><td nowrap>71,899 req/s</td><td nowrap>59,560 req/s</td><td nowrap>66,294 req/s</td><td nowrap>🏆 Clear Win</td></tr>
    <tr><td nowrap><strong>/callback</strong></td><td nowrap><strong>74,105 req/s</strong></td><td nowrap><strong>74,105 req/s</strong></td><td nowrap><strong>74,105 req/s</strong></td><td nowrap>72,939 req/s</td><td nowrap>63,430 req/s</td><td nowrap>70,231 req/s</td><td nowrap>🏆 Clear Win</td></tr>
    <tr><td nowrap><strong>/json</strong></td><td nowrap><strong>71,280 req/s</strong></td><td nowrap><strong>71,280 req/s</strong></td><td nowrap>73,150 req/s</td><td nowrap>68,428 req/s</td><td nowrap>50,888 req/s</td><td nowrap>61,915 req/s</td><td nowrap>🏆 Clear Win (Rust parsing)</td></tr>
    <tr><td nowrap><strong>/cpu (100k loop)</strong></td><td nowrap>4,215 req/s</td><td nowrap>70,988 req/s</td><td nowrap><strong>73,250 req/s</strong></td><td nowrap>9,239 req/s</td><td nowrap>8,863 req/s</td><td nowrap>9,164 req/s</td><td nowrap>🚀 Crushing Win (Rust/Native)</td></tr>
  </tbody>
</table>
</div>

With this, NioJS looks unbeatable for anyone seeking peak performance without leaving the JS ecosystem.
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

Callbacks execute on a multi-worker QuickJS engine pool (auto-sized to the detected physical core count, configurable via `--workers`) with multi-threaded HTTP I/O. Synchronous handlers avoid promise scheduling; asynchronous handlers and thenables drive the job queue. Both finish a microtask checkpoint within the invocation deadline. Request headers and search parameters are constructed when first accessed. Text/JSON response strings and binary buffers pass directly to Rust without a base64/JSON response envelope. Exact routes are indexed, and constant response headers and bytes are cached.

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

`/** @native */` is an optimization hint for eligible functions. Numeric functions with local variables, arithmetic, branches, loops, and selected `Math` calls are compiled to machine code by the Rust host at worker startup. Pure object and array expressions, including bounded `Array.from` generation, use a Rust JSON builder. Unsupported code and calls with nonnumeric arguments retain their JavaScript behavior. The [native guide](docs/native.html) shows both supported examples and limits.

Multi-worker servers automatically detect physical CPU topologies when sizing the worker pool.

## Python AI Bridge

Integrate standard Python libraries (`pandas`, `scipy`), machine learning models, and standard scripts directly into your TypeScript services without IPC or microservice latency.

📖 **[Read the Full Python Integration Guide](https://nio-labs.github.io/nio-js/python.html)** to learn how to write full Python code natively inside NioJS.

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

macOS ARM64 and Linux x86-64 under WSL2 have been exercised in this workspace. Native Linux, Windows, Termux, and experimental iSH require separate builds and device validation.

## Scope & Roadmap

`NioJS` is evolving towards its v1.0.0 General Availability release, featuring deep native integration with **`nio`** (AI agent & tool bus) and **`nio-db`** (durable database & SSE events).

For the complete milestone timeline, feature breakdown, and architecture design across the Nio ecosystem, see **[ROADMAP_V1.md](ROADMAP_V1.md)**.
