# nio-js

The Hybrid, Agent-Native Worker Runtime using QuickJS, a multi-worker Rust host, native Rust offloading, and in-process Python AI.

**Write simple handlers. Import dependencies by URL. Offload compute to Rust & Python. Package a verified application into one `.njs` capsule.**

Status: design specification with full multi-worker server, agent tooling (MCP server, agent-json checks), native function acceleration, and modular Python AI execution documented in [README.md](README.md).

## Product direction

nio-js makes small services easy to write, inspect, deploy, and execute by autonomous AI agents. Its identity is a simple native application API, verified dependency packaging, multi-core worker pinning, and hybrid hybrid execution.

- Project and CLI: `nio-js`.
- Built-in application module: `'nio.js'`.
- Source: `.js`, `.ts`, and `.py`.
- Application capsule: `.njs`.
- Dependency lockfile: `nio.lock`.
- Engine: QuickJS embedded in a multi-worker Rust host with optional PyO3 CPython bridge.

There is no npm installation step or project-local `node_modules`. Initial dependency preparation may use the network; a prepared capsule executes without fetching dependencies. Application network operations remain subject to host permissions.

nio-js is a separate runtime. Keep the existing nio-db launcher, Node/AlaSQL query path, and Node event workers unchanged. Integration with Nio and nio-db is future work.

## Developer experience

### Hello World

```typescript
// server.ts
import { get } from 'nio.js'

get('/', 'Hello World')
get('/health', () => ({ status: 'ok' }))
```

```bash
nio-js run server.ts --port 3000
```

The host registers routes while evaluating the entry module, then starts listening. Registration closes before serving requests. An entry-module error prevents startup.

Constant responses execute through a native host path. Callback handlers execute in QuickJS. Async callbacks are supported.

### Requests and responses

```typescript
import { get, post, reply } from 'nio.js'

get('/greeting', ({ query }) => {
  const name: string = query.name ?? 'World'
  return `Hello ${name}`
})

post('/echo', async ({ json }) => {
  const body = await json()
  return reply(body, { status: 201 })
})
```

Proposed response rules:

- Strings produce UTF-8 text responses.
- JSON-compatible plain objects and arrays produce JSON responses.
- `Blob` and `File` produce binary responses using their declared media type, or `application/octet-stream`.
- `reply(body, { status, headers })` controls status and headers explicitly.
- Unsupported return values cause a handler error; they are not silently coerced.
- Unmatched routes return 404. Unsupported methods on a matched route return 405.

The request context exposes method, URL, headers, query parameters, route parameters, and body readers. Body parsing has explicit limits; the body can be consumed only once. Repeated query keys require an explicit multi-value accessor.

Runtime errors produce a generic server error with a request ID. Detailed diagnostics go to logs, with source locations where available. Client disconnects and shutdown propagate cancellation to supported host operations.

### TypeScript

Transform TypeScript into JavaScript before execution and cache the result. Provide declarations for `'nio.js'` to support editor completion and external type checking.

TypeScript transformation does not perform full type checking. Type checking is outside the initial runtime scope. Preserve source maps for diagnostics.

## URL dependencies

```typescript
import { get } from 'nio.js'
import { z } from 'https://esm.sh/zod@3.23.8?target=es2022'

get('/', () => z.string().parse('Hello World'))
```

Support compatible ESM from HTTPS providers:

| Provider | Usage |
|---|---|
| `esm.sh` | Transformed ESM with rewritten dependency imports |
| `unpkg.com` | Published files; select an actual ESM entry |
| `esm.unpkg.com` | UNPKG's transformed ESM service |
| Other HTTPS sources | Compatible ESM served directly |

An explicit raw ESM import can look like:

```typescript
import { z } from 'https://unpkg.com/zod@3.23.8/lib/index.mjs'
```

These examples illustrate the loader contract; compatibility must be verified against the selected engine and API implementation.

Resolver rules:

1. Reserve `'nio.js'` for the built-in module. Reject overrides.
2. Resolve local relative source imports within the permitted source root.
3. Resolve remote relative imports against the final URL after redirects.
4. Require explicit import mappings for other bare specifiers. Do not automatically choose package versions.
5. Resolve and lock the complete executable dependency graph.
6. Record requested and final URLs, integrity hashes, and transformation settings.
7. Set explicit CDN compilation targets and preserve relevant query parameters.
8. Verify fetched bytes against existing locks. Mismatches fail; they do not silently update the lock.
9. Do not silently switch between providers.
10. Reject unsupported imports and report the dependency chain leading to them.

Support dynamic imports only when the target is present in the prepared module graph. Literal imports can be discovered during preparation; computed targets require explicit inclusion. Capsule execution never downloads a missing module.

Module fetching and application networking have separate policies. Restrict preparation hosts and redirects; enforce host policy on every connection. Treat first-time locking as recording trust in fetched bytes, not proof that those bytes are safe.

An ESM CDN does not make every npm package compatible. Node built-ins, native addons, DOM APIs, and other unsupported host features remain unavailable.

References: [esm.sh documentation](https://esm.sh/), [UNPKG documentation](https://unpkg.com/).

## Runtime architecture

| Component | Responsibility |
|---|---|
| Rust host | HTTP transport, routing, I/O, permissions, scheduling, limits, shutdown |
| QuickJS | JavaScript modules, callbacks, promises, language execution |
| Module preparation | URL resolution, integrity verification, TypeScript transformation, caching |
| Capsule format | Executable module graph, assets, manifest, compatibility requirements |
| `'nio.js'` | Simple route registration and host-operation interfaces |

Use a dedicated execution thread per QuickJS runtime. The Rust I/O layer dispatches bounded work and schedules promise completion back onto the owning runtime thread. Do not assume one QuickJS runtime can execute JavaScript concurrently on several threads.

Reuse initialized application state. Bound queued and active requests. A CPU-bound handler can block its execution thread, so execution interruption and deadlines are required.

Expose selected web APIs for dependency interoperability and file handling, while retaining `'nio.js'` as the main application interface. Publish a supported-API matrix instead of claiming full browser or Node compatibility.

Initial interoperability targets: console, timers, encoders, URL handling, HTTP fetch, cancellation, Blob/File, multipart forms, and the stream functionality needed for bounded I/O. Add APIs deliberately rather than treating the entire web platform as an MVP requirement.

Do not expose QuickJS's unrestricted `std` or `os` modules to application code.

## Content-addressed cache and lockfile

Store individual source and emitted module objects by full cryptographic digest. Maintain manifests that reference them; do not make package names or truncated hashes the identity.

Source caches and transformed-code caches have different keys. Transformation keys include source integrity, transformer version, compiler options, and runtime target. Different provider output may produce different objects for the same package version.

Objects are immutable after insertion. Publish writes atomically and coordinate concurrent preparation. Explicit garbage collection may remove unreferenced objects; avoid promising a store that grows forever.

`nio.lock` records the complete graph and preparation settings. Runtime paths are derived locally rather than committed as machine-specific store paths.

Source execution may create a first lock. Changes to an existing dependency graph require an explicit update mode. Provide frozen preparation for CI. Deterministic serialization makes review easier, but does not eliminate merge conflicts.

Hashes verify byte identity. They do not certify authorship, benign behavior, or identical results across different machines and external services.

## Capsules

```bash
nio-js build server.ts -o server.njs
nio-js inspect server.njs
nio-js run server.njs --port 3000
```

A capsule contains:

- Entrypoint and complete executable module graph.
- Packaged assets and their integrity hashes.
- Source maps where included by build policy.
- Format version and runtime/API compatibility requirements.
- Declared application capabilities and resource requirements.
- Dependency and preparation metadata for inspection.

Secrets and live database state remain outside capsules. Declared permissions never grant themselves: operator policy decides what the host allows.

Use portable JavaScript as the canonical executable payload. Optional QuickJS bytecode is a local cache keyed by engine version, build configuration, and platform; do not treat it as a portable capsule format or load arbitrary untrusted bytecode.

Validate capsule structure, sizes, entry names, references, and hashes before execution. Prevent archive traversal if extraction is used. Local hash validation detects corruption relative to a trusted manifest; authenticating a distributed artifact requires a trusted digest or signature delivered through a separate trust mechanism.

An application with assets can load them from the capsule without granting arbitrary host filesystem access.

## Browser file API interoperability

Implement `Blob` and `File` in nio-js; QuickJS does not supply these host APIs.

```typescript
import { post, reply } from 'nio.js'

post('/upload', async ({ formData }) => {
  const form = await formData()
  const file = form.get('file')

  if (!(file instanceof File)) {
    return reply({ error: 'Choose a file' }, { status: 400 })
  }

  return {
    name: file.name,
    size: file.size,
    content: await file.text(),
  }
})
```

Browser-side upload:

```javascript
const file = input.files?.[0]
if (!file) throw new Error('Choose a file')

const form = new FormData()
form.append('file', file)

await fetch('/upload', { method: 'POST', body: form })
```

The browser sends bytes and metadata. nio-js constructs its own File object. Browser handles and object URLs do not automatically become usable server resources.

Enforce total upload, per-file, field-count, and parsing limits. Treat uploaded names and media types as untrusted metadata. `.text()` and `.arrayBuffer()` materialize content; provide bounded streaming for large bodies. Temporary disk-backed storage belongs to the host and needs quotas and cleanup.

File/Blob compatibility does not grant filesystem access. Browser DOM file pickers remain frontend functionality. Native directory mounts and writable handles are deferred.

Reference: [Browser File API](https://developer.mozilla.org/en-US/docs/Web/API/File_API).

## Permissions and resource control

Applications have no ambient filesystem, process, or network authority.

- Deny application outbound network access by default; grant specific destinations through host configuration.
- Apply destination policy across redirects and resolved addresses, including private-network access.
- Keep source loading, dependency preparation, and application I/O as distinct host operations.
- Bound JavaScript allocations, host buffers, request sizes, output sizes, concurrency, and queue depth.
- Enforce execution deadlines and cancellation of supported I/O.
- Keep secrets in explicit host bindings and exclude them from artifacts and routine logs.

QuickJS allocation limits are not whole-process memory limits. Host allocations and networking require separate accounting. Untrusted code may require additional OS isolation; do not claim engine controls alone establish a hardened sandbox.

Future filesystem access should use scoped capabilities with traversal and symlink boundaries enforced by the host. The principle is **no filesystem access by default**, rather than claiming the host has no filesystem.

## MVP scope

1. Execute JS modules, promises, and async handlers through QuickJS.
2. Transform TypeScript and provide `'nio.js'` type declarations.
3. Provide `get`, `post`, `reply`, constant responses, and predictable request/response conversion.
4. Load compatible HTTPS ESM, lock the graph, and cache verified bytes.
5. Build, inspect, validate, and execute `.njs` capsules without dependency downloads.
6. Implement basic File/Blob interoperability, multipart parsing, and bounded I/O.
7. Enforce initial network permissions, memory accounting, deadlines, and request limits.
8. Provide source-mapped diagnostics, request IDs, logs, graceful shutdown, and exit codes.
9. Check declared bindings and runtime compatibility before accepting requests.
10. Publish measured performance and a tested compatibility matrix.

Initial commands:

```bash
nio-js run server.ts --port 3000
nio-js build server.ts -o server.njs
nio-js inspect server.njs
nio-js run server.njs --port 3000
```

Before implementation, finalize configuration syntax, frozen/update flags, grant syntax, default limits, cache location, capsule layout, and HTTP request-context types. Examples above do not settle those contracts.

## Distinctive direction

The proposed execution contract makes application requirements inspectable and enforceable. One artifact carries verified dependencies, assets, declared capabilities, and compatibility requirements.

Build the following on that foundation:

| Feature | Stage | Value |
|---|---|---|
| Preflight and artifact inspection | MVP | Detect unmet declared requirements before serving |
| Permission and dependency diff | After MVP | Review how an upgrade changes application authority |
| Incident recording and replay | Later | Reproduce supported interactions without repeating external writes |
| Shared invocation context for routes, events, and AI tools | Future integration | Carry caller identity, capability scope, deadline, and trace ID |

These are proposed differentiators, not claims of global novelty or established superiority over other runtimes.

## Performance strategy

Optimize native routing, constant responses, warm runtimes, bounded asynchronous I/O, streaming, and reduced JS-to-Rust copying. Add isolated workers across CPU cores when measurements justify it.

Expected opportunity: small-service startup, modest memory, and selected request paths. Sustained CPU-heavy JavaScript may favor optimizing engines used by Node and Bun. Do not promise a universal speed multiplier.

Benchmark equivalent implementations on the same hardware and report:

- Process launch to HTTP readiness with dependencies cached.
- Idle and loaded resident memory, plus runtime and artifact size.
- HTTP throughput and p50/p95/p99 latency.
- JSON processing and CPU-heavy JavaScript.
- Concurrent outbound I/O, cancellation, overload, and error paths.

Compare constant-response paths with equivalent optimized competitor paths. Engine initialization timings are not complete application startup timings.

Reference: [QuickJS documentation](https://bellard.org/quickjs/quickjs.html).

## Platform targets

- Desktop/server: Linux, macOS, and Windows, validated per architecture.
- Termux: design for dedicated Android-compatible builds, initially ARM64. Promote to supported after device tests.
- iSH: experimental 32-bit x86 Linux/musl target. Engine and Rust dependency compatibility require validation inside iSH.

Portable capsule payloads allow preparation on a desktop and execution on a supported phone runtime. Compatibility metadata and host requirements still apply. Runtime portability does not imply support for native addons or identical external behavior.

References: [Termux execution environment](https://github.com/termux/termux-packages/wiki/Termux-execution-environment), [iSH repository](https://github.com/ish-app/ish).

## Future Nio and nio-db integration

Keep nio-js useful independently. Add Nio and nio-db as optional capabilities later.

Illustrative future API:

```typescript
import { on, tool, db } from 'nio.js'

on('orders.created', ({ record }) =>
  db.create('tasks', { orderId: record.id })
)

tool('pendingOrders', () =>
  db.list('orders', { status: 'pending' })
)
```

This requires explicit database bindings, tool descriptions and input schemas, authorization propagation, retry/idempotency rules, and event delivery contracts. A convenient `db` binding must enforce the active invocation's identity and permissions. A tool registration must not grant administrative database access to an agent.

Do not assume existing live events provide durable delivery or replay. Define those semantics before adding durable execution.

No existing launcher, query engine, event-worker runtime, or service implementation is replaced as part of the current scope.

## Deferred work

- General Node API compatibility, native addons, and arbitrary CommonJS execution.
- Native filesystem mounts and writable file handles.
- Built-in PM2-like supervision or an npm/npx replacement launcher.
- Database bindings, AI tools, scheduled jobs, and durable workflows.
- Incident replay, automatic failover, and application migration.
- Full browser APIs, DOM, and embedded browser UI.
- Full TypeScript type checking inside the runtime.

External supervisors such as PM2 or systemd can manage the native CLI once lifecycle behavior is implemented; do not assume Node-specific cluster mode applies.

## Acceptance criteria

- A `.ts` service exposes text, JSON, and async routes with source-mapped errors.
- A tested esm.sh dependency and a tested raw UNPKG ESM dependency execute correctly.
- Locked-byte mismatches fail, unresolved imports identify their dependency chain, and frozen preparation does not update the lock.
- A built capsule runs on another supported machine without Node, npm, a pre-existing module cache, or dependency-network access.
- Denied outbound destinations and redirected requests cannot bypass the configured policy.
- Oversized uploads, runaway JavaScript, excessive allocations, and overloaded queues fail within defined limits.
- File metadata and bytes behave as documented, and streaming cleanup works on cancellation.
- Shutdown stops accepting work, drains within a deadline, and exits predictably.
- Core behavior is tested on Termux before support is claimed; iSH remains experimental until validated.
- Benchmarks and documented limitations accompany performance claims.
