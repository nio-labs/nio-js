# Roadmap to v1.0.0: The Agent-Native Worker Runtime

> **Target:** Stable General Availability (GA) v1.0.0  
> **Mission:** Establish `nio-js` as the ultra-fast, sub-millisecond polyglot worker runtime natively integrated into the Nio ecosystem (`nio` + `nio-db` + `nio-js`).

---

## 🏛️ Ecosystem Architecture

```
                             ┌───────────────────────────────────┐
                             │        nio (Intelligence)         │
                             │  Agent Engine • Tool Registry     │
                             │  Natural Language Query Planning  │
                             └─────────────────┬─────────────────┘
                                               │ Tool Protocol / MCP
                                               ▼
┌─────────────────────────────────┐    ┌───────────────────────────────────┐
│        nio-db (Storage)         │◄───┤         nio-js (Runtime)          │
│  Durable Journal • SQL Engine   │IPC/│  Multi-Worker Rust Engine         │
│  SSE Events • Buckets & Records │HTTP│  QuickJS + PyO3 In-Process Python │
└─────────────────────────────────┘    └───────────────────────────────────┘
```

- **`nio`** ([repo](file:///home/mn/nio-labs/nio)): Terminal AI coding agent, autonomous tool dispatch, natural-language workflow planner.
- **`nio-db`** ([repo](file:///home/mn/nio-labs/nio-db)): Standalone Rust storage server with durable append-only journal, AlaSQL compiler, SSE event broadcast, and bucket storage.
- **`nio-js`** ([repo](file:///home/mn/nio-labs/nio-js)): High-performance worker runtime (<1ms boot, 68k+ req/sec, hybrid Rust/JS engine, in-process CPython bridge, MCP server).

---

## 🗺️ Milestone Breakdown

### 📦 Milestone 1: Streaming & Agent Protocols (`v0.3.0`)
**Theme:** Real-time token streaming and bidirectional agent communication.

- [ ] **Chunked Response & Server-Sent Events (SSE):**
  - Expose streaming responses in capsules for LLM token-by-token generation:
    ```javascript
    import { route } from 'nio.js';

    route.get('/stream', (req, res) => {
      res.sse((stream) => {
        stream.send({ token: 'Hello' });
        stream.send({ token: ' World' });
        stream.close();
      });
    });
    ```
- [ ] **WebSocket Server Support:**
  - Native Tokio-tungstenite WebSocket upgrade support on capsule routes.
  - Interactive agent chat sessions and bidirectional tool streaming.
- [ ] **Tracing & Observability:**
  - W3C Trace Context and OpenTelemetry header propagation across incoming requests and sub-agent invocations.
  - Diagnostics JSON output in `--agent-json` format for streaming trace analysis.

---

### 🗄️ Milestone 2: `nio-db` Native Integration (`v0.4.0`)
**Theme:** Zero-latency persistence and durable reactive data streams.

- [ ] **First-Class `nio-db` Client (`import { db } from 'nio.js'`):**
  - Seamless auto-discovery of local or remote `nio-db` instances via `NIODB_URL` and `NIODB_TOKEN`:
    ```javascript
    import { db, route } from 'nio.js';

    route.post('/records', async (req) => {
      const doc = await db.collection('artifacts').insert(req.body);
      return { ok: true, doc };
    });

    route.get('/query', async (req) => {
      return await db.sql('SELECT * FROM artifacts WHERE status = ?', ['active']);
    });
    ```
- [ ] **Reactive Event Consumer (SSE Bridge):**
  - Native listener for `nio-db` live record mutations:
    ```javascript
    import { db } from 'nio.js';

    db.on('records.created', async (record) => {
      console.log('New record persisted in nio-db:', record.id);
    });
    ```
- [ ] **Zero-Network In-Memory Cache:**
  - Rust-level in-process cache layer for `nio-db` queries, eliminating IPC round-trips for high-frequency agent lookups.
- [ ] **Storage Capability Flags:**
  - Enforce permission boundaries via `--allow-db[=scope]` sandboxing.

---

### ⚡ Milestone 3: `nio` Agent & Event Bus (`v0.5.0`)
**Theme:** Event-driven worker execution and unified tool bridging.

- [ ] **Event-Driven Worker Model (`import { on } from 'nio.js'`):**
  - Support non-HTTP micro-workers triggered by `nio` events, cron schedules, or background tasks:
    ```javascript
    import { on } from 'nio.js';

    on('git.commit', async (event) => {
      console.log('Automated agent review triggered for commit:', event.sha);
    });
    ```
- [ ] **Dual-Target Tool Registry (`tool()` API):**
  - Unified tool definition exported simultaneously to **stdio MCP server** (Cursor, Claude Desktop, Zed) and **`nio` CLI plugin catalog**:
    ```javascript
    import { tool, db } from 'nio.js';

    export const findArtifact = tool({
      name: 'find_artifact',
      description: 'Search records in nio-db',
      parameters: {
        type: 'object',
        properties: { query: { type: 'string' } },
        required: ['query'],
      },
      handler: async ({ query }) => {
        return await db.collection('artifacts').find({ query });
      },
    });
    ```
- [ ] **Scoped Agent Permissions:**
  - Unify permissions with `nio` security policies (`--allow-net`, `--allow-db`, `--allow-python`, `--allow-fs`).

---

### 🛡️ Milestone 4: Production Hardening & High Availability (`v0.6.0` – `v0.7.0`)
**Theme:** Enterprise-grade reliability, hot reloading, and resource guarantees.

- [ ] **Zero-Downtime Hot Reload (`nio-js dev` / SIGHUP):**
  - Dynamic capsule re-evaluation without dropping active TCP connections or aborting running agent requests.
- [ ] **Sandboxed Isolation & Resource Limits:**
  - Worker memory caps (`--max-memory=64M`).
  - Execution timeouts (`--timeout=5000ms`).
  - Crash isolation: Poison-pill capsules safely terminate worker threads without taking down the server.
- [ ] **High-Throughput Static Assets:**
  - Kernel-offloaded static file serving (`route.static('/public')`) backed by Tokio `fs` and byte streaming.
- [ ] **Single-File Bundling & Compression:**
  - Enhanced `.njs` capsule bundler with embedded bytecode caching and asset packing.
- [ ] **Interactive Ecosystem Scaffolding (`nio-js init`):**
  - **`nio-js init web`**: Generates a full-stack Nio ecosystem monorepo. Interactive prompts for:
    - Frontend: Vanilla JS (zero-build), Lit, React, Vue, Eleventy.
    - Database: `nio-db` or None.
    - Intelligence: NioAI (Python tools/agents) or None.
  - **`nio-js init app`**: Cross-platform mobile generation using Vue + Capacitor + NioJS backend API.
  - **Unified DevEx**: All scaffolded projects include a pre-configured `package.json` with unified scripts (`npm run dev`, `npm run prod`) to orchestrate both the UI and Nio backend seamlessly.
  - **Ready-to-Deploy**: Generates an optimized `Dockerfile` and `docker-compose.yml` for instant production deployment of the full stack.

---

### 🌍 Milestone 5: WebAssembly & Polyglot Engine (`v0.8.0`)
**Theme:** Universal execution and secure sandboxing for C, C++, Rust, Go, and Zig.

- [ ] **WebAssembly (Wasm) Runtime Core:**
  - Leverage QuickJS native Wasm support or a dedicated Wasm engine to run precompiled `.wasm` binaries.
  - Near-native execution speeds for heavy math, parsing, and cryptographic operations.
- [ ] **WASI (WebAssembly System Interface) Support:**
  - Secure, capability-based access to file system, environment variables, and system clocks for backend Wasm modules.
  - Strict sandboxing: Wasm capsules cannot access host resources unless explicitly granted via capability flags.
- [ ] **TypeScript Polyglot API (`import { wasm } from 'nio.js'`):**
  - Instant instantiation of Go, Rust, and C modules directly from TypeScript:
    ```javascript
    import { wasm } from 'nio.js';

    // Loads and caches a compiled Go/C/Rust module
    const parser = await wasm.load('heavy_parser.wasm');
    const result = parser.exports.parseData(myPayload);
    ```

---

### 🚀 Milestone 6: `v1.0.0` General Availability (GA)
**Theme:** Stable contract, unified ecosystem testing, and global distribution.

- [ ] **Frozen Capsule Specification v1.0:**
  - Strict semantic versioning and guaranteed backward compatibility across `route`, `tool`, `db`, `on`, and `py`.
- [ ] **Unified Multi-Service Test Suite:**
  - Automated end-to-end integration test runner validating `nio` (agent) ➔ `nio-js` (worker) ➔ `nio-db` (storage) end-to-end.
- [ ] **Complete TypeScript Definitions:**
  - Comprehensive, standalone `d.ts` definitions bundled directly with the runtime.
- [ ] **Turnkey Deployment Presets:**
  - Ready-to-run deployment profiles: Docker, Koyeb, Railway, and systemd units.
  - Multi-platform standalone binaries verified across all 6 target platforms:
    - Linux (`x86_64`, `aarch64`)
    - macOS (`x86_64`, `aarch64`)
    - Windows (`x86_64`)
    - Android Termux (`aarch64`)

---

## 📈 Version Trajectory Summary

| Release | Focus | Key Deliverables |
|---|---|---|
| **v0.2.0** *(Current)* | Hybrid Engine & Performance | Bare-metal Rust offloading, in-process PyO3 Python AI, stdio MCP server, 68k+ req/sec |
| **v0.3.0** | Protocols & Streaming | SSE token streaming, WebSockets, W3C OpenTelemetry tracing |
| **v0.4.0** | `nio-db` Storage Tier | Native `db` client, reactive SSE mutation listener, in-memory query cache |
| **v0.5.0** | `nio` Agent & Event Bus | Event workers (`on()`), dual MCP + `nio` tool registry (`tool()`), unified sandbox |
| **v0.6.0** | Resilience & DevEx | Zero-downtime hot reload, worker resource limits, memory boundaries |
| **v0.7.0** | Packaging, Static Assets & Scaffolding | Static asset serving, compiled `.njs` bytecode cache, optimized capsule distribution, nio-js init scaffolding |
| **v0.8.0** | WebAssembly & Polyglot Engine | Wasm runtime core, WASI support, safe execution for C/C++/Rust/Go capsules |
| **v1.0.0** | **General Availability (GA)** | Frozen Capsule v1.0 spec, cross-ecosystem test suite, production deployment presets |
