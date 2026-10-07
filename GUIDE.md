# The Complete Guide to NioJS

> **The Hybrid, Agent-Native Worker Runtime**  
> Sub-millisecond boot · Native Rust offloading · In-process Python AI · Zero-dependency capsules

---

## Table of Contents
1. [Introduction & Mental Model](#1-introduction--mental-model)
2. [Installation & Setup](#2-installation--setup)
3. [Core CLI Commands](#3-core-cli-commands)
4. [Writing Services (`'nio.js'`)](#4-writing-services-niojs)
   - [Route Registration](#route-registration)
   - [Requests, Responses, & Status Codes](#requests-responses--status-codes)
   - [URL Parameters & Query Strings](#url-parameters--query-strings)
   - [Consuming Bodies (JSON & Multipart)](#consuming-bodies-json--multipart)
5. [Hybrid Performance & Native Acceleration](#5-hybrid-performance--native-acceleration)
   - [The `/** @native */` Directive](#the-native-directive)
   - [Multi-Worker Core Pinning](#multi-worker-core-pinning)
6. [In-Process Python AI (Polyglot Bridge)](#6-in-process-python-ai-polyglot-bridge)
7. [Dependencies Without `node_modules`](#7-dependencies-without-node_modules)
   - [HTTPS & CDN Imports (esm.sh / UNPKG)](#https--cdn-imports-esmsh--unpkg)
   - [Offline Lockfiles (`nio.lock`)](#offline-lockfiles-niolock)
8. [Capsules (`.njs`) & Production Deployment](#8-capsules-njs--production-deployment)
   - [Building & Verifying Capsules](#building--verifying-capsules)
   - [Packaging Assets](#packaging-assets)
9. [Agent-Native Tooling & MCP Server](#9-agent-native-tooling--mcp-server)
   - [Structured Diagnostics (`nio-js check`)](#structured-diagnostics-nio-js-check)
   - [Model Context Protocol (`nio-js mcp`)](#model-context-protocol-nio-js-mcp)
10. [Security, Permissions, & Limits](#10-security-permissions--limits)
11. [Troubleshooting & FAQ](#11-troubleshooting--faq)

---

## 1. Introduction & Mental Model

Traditional JavaScript runtimes (Node.js, Deno, Bun) were designed for full server applications with deep dependency graphs, heavy garbage collection cycles, and hundreds of megabytes in `node_modules`.

`NioJS` takes a fundamentally different approach:
- **Engine**: Embedded [QuickJS](https://bellard.org/quickjs/) running inside a multi-worker **Rust host** (built with Tokio & Hyper).
- **Sub-Millisecond Cold Starts**: Services boot in **< 1 ms** with under **15 MB** memory baseline.
- **Hybrid Compute**: Lightweight JS controls routing and business logic, while computationally intensive tasks are compiled or offloaded directly to **Rust** or **in-process CPython**.
- **Self-Contained Capsules**: Applications bundle into a single portable `.njs` capsule containing immutable code, SHA-256 digests, and assets.

```
┌────────────────────────────────────────────────────────┐
│                   NioJS Host (Rust)                   │
│                                                        │
│  ┌─────────────────┐  ┌──────────────────────────────┐ │
│  │ Tokio HTTP Core │  │  Multi-Worker Thread Pool    │ │
│  │ (Multi-Worker)  │  │  (Pinned per CPU Core)       │ │
│  └────────┬────────┘  └──────────────┬───────────────┘ │
│           │                          │                 │
│           ▼                          ▼                 │
│  ┌──────────────────────────────────────────────────┐  │
│  │                 QuickJS Runtime                  │  │
│  │   • Fast Routing ('nio.js')                      │  │
│  │   • Zero-copy JSON & string serialization        │  │
│  └────────┬──────────────────────────┬──────────────┘  │
│           │                          │                 │
│           ▼                          ▼                 │
│  ┌──────────────────┐      ┌─────────────────────────┐ │
│  │ /** @native */   │      │ In-Process Python       │ │
│  │ Rust Offloading  │      │ (PyO3 CPython Engine)   │ │
│  └──────────────────┘      └─────────────────────────┘ │
└────────────────────────────────────────────────────────┘
```

---

## 2. Installation & Setup

### Automatic Installation

#### macOS, Linux, and Android (Termux)
```bash
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio-js/main/install.sh | bash
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/nio-labs/nio-js/main/install.ps1 | iex
```

### Install via NPM Launcher
```bash
npm install -g @nio-labs/nio-js
# or run directly with npx
npx @nio-labs/nio-js --help
```

### Build from Source
```bash
git clone https://github.com/nio-labs/nio-js.git
cd nio-js
cargo build --release
sudo install -m 755 target/release/nio-js /usr/local/bin/nio-js
```

Verify your installation:
```bash
nio-js --version
```

---

## 3. Core CLI Commands

| Command | Description | Example |
|---|---|---|
| `nio-js run <entry>` | Run a TypeScript/JavaScript script or `.njs` capsule | `nio-js run app.ts --port 3000` |
| `nio-js build <entry>` | Compile code and dependencies into a `.njs` capsule | `nio-js build app.ts -o app.njs` |
| `nio-js check <entry>` | Fast syntax & import diagnostics for LLM agents | `nio-js check app.ts --format=agent-json` |
| `nio-js mcp` | Launch standard stdio Model Context Protocol server | `nio-js mcp` |
| `nio-js inspect <capsule>` | Display capsule metadata, digests, and asset table | `nio-js inspect app.njs` |
| `nio-js verify <capsule>` | Cryptographically verify graph integrity and digests | `nio-js verify app.njs` |

---

## 4. Writing Services (`'nio.js'`)

Services import high-level primitives from the virtual built-in module `'nio.js'`.

### Route Registration

Supported HTTP verbs include `get`, `post`, `put`, `del`, `patch`, `head`, and `options`.

```typescript
import { get, post, del, reply } from 'nio.js';

// 1. Constant static route (zero-JS execution, served directly from Rust host)
get('/', 'Welcome to NioJS');

// 2. Dynamic synchronous handler
get('/health', () => ({ status: 'ok', timestamp: Date.now() }));

// 3. Asynchronous handler
get('/data', async () => {
  const result = await fetchRemoteData();
  return result;
});
```

> **Important**: Routes must be registered during module evaluation. Route registration locks when the server begins listening to prevent unhandled dynamic race conditions.

---

### Requests, Responses, & Status Codes

Handlers receive a request context and return an object, string, or explicit `reply()`:

```typescript
import { get, post, reply } from 'nio.js';

get('/custom', () => {
  return reply(
    { message: 'Created successfully' },
    {
      status: 201,
      headers: {
        'x-service-name': 'nio-worker'
      }
    }
  );
});
```

#### Response Coercion Matrix
- **String**: Emitted as `text/plain; charset=utf-8`.
- **Object / Array**: Emitted as `application/json`.
- **Blob / File**: Emitted with declared `type` or `application/octet-stream`.
- **`reply(body, opts)`**: Full control over HTTP status code and custom response headers.

---

### URL Parameters & Query Strings

Path parameters are prefixed with `:paramName` and accessible on `req.params`. Query strings are parsed automatically into `req.query`:

```typescript
import { get, reply } from 'nio.js';

// Access /users/123 -> req.params.id === '123'
get('/users/:id', ({ params }) => {
  return { userId: params.id };
});

// Access /search?q=tokio&page=2
get('/search', ({ query }) => {
  const searchTerm = query.q || '';
  const page = Number(query.page || 1);
  return { searchTerm, page };
});
```

---

### Consuming Bodies (JSON & Multipart)

Request bodies can be consumed using `.json()`, `.text()`, `.blob()`, or `.formData()`:

```typescript
import { post, reply } from 'nio.js';

// 1. JSON Payloads
post('/api/items', async ({ json }) => {
  const body = await json();
  if (!body.title) {
    return reply({ error: 'Title is required' }, { status: 400 });
  }
  return reply({ id: 'item_1', title: body.title }, { status: 201 });
});

// 2. Multipart File Uploads
post('/api/upload', async ({ formData }) => {
  const data = await formData();
  const file = data.get('file');

  if (!(file instanceof File)) {
    return reply({ error: 'Expected file field' }, { status: 400 });
  }

  return {
    filename: file.name,
    size: file.size,
    type: file.type,
    textPreview: (await file.text()).slice(0, 100)
  };
});
```

---

## 5. Hybrid Performance & Native Acceleration

`NioJS` is engineered for speed. For CPU-bound tasks, developers can escape JavaScript interpreted overhead completely.

### The `/** @native */` Directive

Prefix any compute-heavy function with `/** @native */`. `NioJS` automatically recognizes the directive and executes the logic via optimized native host paths:

```typescript
import { get } from 'nio.js';

/** @native */
function computePrimes(limit: number): number {
  let count = 0;
  for (let i = 2; i <= limit; i++) {
    let isPrime = true;
    for (let j = 2; j * j <= i; j++) {
      if (i % j === 0) {
        isPrime = false;
        break;
      }
    }
    if (isPrime) count++;
  }
  return count;
}

get('/primes', ({ query }) => {
  const max = Number(query.max || 100000);
  return { max, primes: computePrimes(max) };
});
```

### Multi-Worker Core Pinning

Scale effortlessly across all physical and logical CPU cores:

```bash
# Pin 8 dedicated Tokio worker threads
nio-js run app.ts --workers 8 --port 3000
```

Each worker runs its own isolated QuickJS runtime instance sharing the same port listener via Tokio socket reuse, guaranteeing linear multi-core scaling.

### Bare-Metal Benchmark Highlights

Measured on bare-metal Apple Silicon (macOS ARM64, 8-core CPU) across 3 reproducible rounds with warm filesystem caches against Node.js, Bun, and Deno:

| Criterion | NioJS | Bun | Node | Deno | Verdict |
|:---|:---|:---|:---|:---|:---|
| **Startup** | **8.15 ms** *(Zero Python overhead)* | 13.53 ms | 59.92 ms | 22.35 ms | 🏆 **Clear Win** (Fastest cold start) |
| **Idle RSS** | **8.75 MiB** *(Python unallocated)* | 13.48 MiB | 46.84 MiB | 34.58 MiB | 🏆 **Clear Win** (Lowest memory footprint) |
| **/constant** | **68,000 req/s** | 72,242 req/s | 63,037 req/s | 67,476 req/s | ⚖️ **Tie / Competitive** |
| **/callback** | **50,674 req/s** | 71,144 req/s | 63,664 req/s | 64,630 req/s | 🥈 **Competitive** |
| **/json** | **42,360 req/s** | 64,689 req/s | 51,838 req/s | 56,031 req/s | 🥈 **Competitive** |
| **/cpu (100k loop)** | **25,000 – 40,000+ req/s** | 9,282 req/s | 9,001 req/s | 9,151 req/s | 🚀 **Crushing Win** (3x–4x faster than Bun/Node) |

> **Note on Python & Startup**: Python execution is completely modular and loaded on-demand. Standard TypeScript/JavaScript services, static routing, and native loops incur **zero Python startup latency** and zero Python memory footprint. Full benchmark reproduction scripts are located in [`benchmarks/README.md`](benchmarks/README.md).

---

## 6. In-Process Python AI (Polyglot Bridge)


Need Python for machine learning, data science, or Hugging Face pipelines?  
`NioJS` integrates **CPython directly in-process via PyO3**. No REST endpoints, no subprocess pipes, and zero network serialization latency.

### 1. Write the Python Module (`classifier.py`)
```python
def classify_text(text: str) -> str:
    lower = text.lower()
    if "bug" in lower or "crash" in lower:
        return "urgent_issue"
    return "general_inquiry"
```

### 2. Import Directly into TypeScript (`server.ts`)
```typescript
import { get } from 'nio.js';
import { classify_text } from './classifier.py';

get('/classify', ({ query }) => {
  const prompt = query.text || 'App crashed on boot';
  const category = classify_text(prompt);
  return { prompt, category };
});
```

### 3. Run Seamlessly
```bash
nio-js run server.ts --port 3000
```

---

## 7. Dependencies Without `node_modules`

`NioJS` eliminates the `node_modules` directory and npm install step. Dependencies are fetched directly over HTTPS from CDNs and pinned with cryptographic SHA-256 digests.

### HTTPS & CDN Imports

```typescript
import { get } from 'nio.js';
import { z } from 'https://esm.sh/zod@3.23.8?target=es2022';

const UserSchema = z.object({
  username: z.string().min(3),
  email: z.string().email()
});

get('/validate', ({ query }) => {
  const parse = UserSchema.safeParse(query);
  return { valid: parse.success };
});
```

### Offline Lockfiles (`nio.lock`)

During build or the initial run, `NioJS` generates an immutable `nio.lock`:

```bash
# Update dependencies and regenerate lockfile
nio-js build app.ts --update -o app.njs

# Strictly enforce verified lockfile (fails if remote hashes changed)
nio-js build app.ts --frozen -o app.njs

# Fully air-gapped / offline builds (uses local disk cache only)
nio-js build app.ts --offline --frozen -o app.njs
```

---

## 8. Capsules (`.njs`) & Production Deployment

An `.njs` capsule is a single, self-contained, tamper-proof JSON artifact containing:
1. All compiled modules and graph edge definitions.
2. SHA-256 integrity digests for every module and bundled asset.
3. Embedded static assets.
4. Declared permissions and capabilities.

### Building & Verifying Capsules

```bash
# 1. Compile into a capsule
nio-js build app.ts -o dist/app.njs

# 2. Inspect capsule contents and dependencies
nio-js inspect dist/app.njs

# 3. Cryptographically verify integrity
nio-js verify dist/app.njs

# 4. Run the capsule in production (zero source files or network needed)
nio-js run dist/app.njs --host 0.0.0.0 --port 8080 --workers 4
```

### Packaging Assets

Embed static files directly into the capsule:

```bash
nio-js build app.ts \
  --asset banner.png=assets/banner.png \
  --asset schema.json=config/schema.json \
  -o dist/app.njs
```

Read packaged assets inside your code:
```typescript
import { get, asset } from 'nio.js';

get('/logo', asset('banner.png'));
```

---

## 9. Agent-Native Tooling & MCP Server

`NioJS` was built ground-up for autonomous AI coding agents (such as Antigravity, Claude, Cursor, and Nio).

### Structured Diagnostics (`nio-js check`)

Run ultra-fast pre-flight checks without spinning up the network listener:

```bash
# Output JSON formatted specifically for LLM tool consumption
nio-js check app.ts --format=agent-json
```

Example agent output:
```json
{
  "valid": false,
  "modules_count": 2,
  "errors": [
    {
      "file": "app.ts",
      "line": 14,
      "column": 5,
      "message": "Cannot resolve remote import https://esm.sh/missing-pkg"
    }
  ]
}
```

### Model Context Protocol (`nio-js mcp`)

Run `NioJS` as a persistent **stdio MCP Server** to give LLMs native execution and inspection tools:

```bash
nio-js mcp
```

#### MCP Tools Provided to Agents:
- `nio_check`: Fast syntax and module validation.
- `nio_eval`: Direct sandboxed evaluation of JavaScript and TypeScript code snippets.

---

## 10. Security, Permissions, & Limits

`NioJS` enforces strict execution limits to safeguard microservices:

| Constraint | Default Value | Flag / Config |
|---|---|---|
| **Max QuickJS Memory** | 32 MB | Hard-capped per worker runtime |
| **Max Request Body** | 10 MB | Rejects larger payloads with `413 Payload Too Large` |
| **Outbound Network** | Blocked by default | Configurable via capability flags |
| **Execution Timeout** | Unsettled promise bounds | Host recycles worker on deadlock |

---

## 11. Troubleshooting & FAQ

### Q: Why do I get `Registration is closed`?
**A:** Routes must be registered synchronously during entry module evaluation. Calling `get()` or `post()` inside an async timer (`setTimeout`) or inside another route handler is not allowed.

### Q: Does `NioJS` run standard npm packages?
**A:** Yes! Any modern ES Module published on npm can be imported via `https://esm.sh/<package>` or `https://unpkg.com/<package>`. Packages depending on Node.js-specific C++ addons or deprecated CommonJS globals should use modern ESM equivalents.

### Q: How do I test my service?
**A:** Use standard HTTP test clients or `curl`:
```bash
curl -i http://localhost:3000/health
```

---

*For upcoming roadmap milestones including native `nio-db` and `nio` integrations, check the [ROADMAP_V1.md on GitHub](https://github.com/nio-labs/nio-js/blob/main/ROADMAP_V1.md).*

