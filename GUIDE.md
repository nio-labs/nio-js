# The Complete Guide to NioJS

> **The Hybrid, Agent-Native Worker Runtime**  
> Fast startup · Native Rust offloading · In-process Python AI · Portable capsules

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
   - [Multi-Worker Execution](#multi-worker-execution)
6. [The Hybrid Engine (Zig, Go, Raw C & Python)](#6-the-hybrid-engine-zig-go-raw-c--python)
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
- **Hybrid Compute**: Lightweight JS controls routing and business logic, while computationally intensive tasks escape seamlessly to **native Rust**, dynamically compiled **Zig** & **Raw C** via FFI, **Go** cloud APIs, or **in-process Python** AI models.
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

`/** @native */` compiles eligible numeric functions to machine code in the Rust host at worker startup. It also supports pure JSON generation through Rust. The original JavaScript remains available for calls that cannot use the native path. The example below uses supported numeric loops and branches:
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

Other supported functions can use local arithmetic, bitwise operations, `if`, `for`, `while`, `break`, `continue`, and selected `Math` operations. Pure object and array expressions can use bounded `Array.from` generation. The [native guide](docs/native.html) lists the contract and limits. Rebuild old capsules to get the corrected behavior.

### Multi-Worker Execution

Run isolated JavaScript runtimes on worker threads. Set the worker count with `--workers`:

```bash
# Run with 8 JavaScript workers
nio-js run app.ts --workers 8 --port 3000
```

Each worker runs its own QuickJS runtime and shares the server's HTTP listener.

---

## 6. The Hybrid Engine (Zig, Go, Raw C & Python)

Native imports need their language compiler during preparation and must follow the restricted ABI in [NATIVE_FFI.md](NATIVE_FFI.md). C uses the system compiler. Capsules embed native libraries and must match the build runtime version, OS, and CPU architecture. Python requires `--features python` and the deployment Python shared library. Native libraries execute with process privileges.

Need Python for machine learning? Zig for cryptography? Go for Kubernetes integrations? Raw C for legacy hardware parsing?  
`NioJS` integrates all of them **directly in-process**. No REST endpoints, no subprocess pipes, and zero network serialization latency.

### The Code (`server.ts`)

You can effortlessly mix and match all supported languages directly in your TypeScript business logic:

```typescript
import { get, reply } from 'nio.js';

// 1. Python AI (via PyO3)
import { classify_text } from './classifier.py';

// 2. Zig FFI (Dynamically compiled C-ABI)
import { verify_signature } from './crypto.zig';

// 3. Go Cloud-Native (via cgo shared library)
import { FetchClusterStatus } from './network.go';

// 4. C (Compiled with the system C compiler)
import { parse_legacy_sensor } from './legacy_parser.c';

get('/api/hybrid', () => {
  const prompt = 'App crashed on boot';
  const category = classify_text(prompt);
  const isValid = verify_signature(150.0, 89231.0);
  const cluster = JSON.parse(FetchClusterStatus());
  const voltage = parse_legacy_sensor(420.5, 1.05);

  return { category, isValid, cluster, voltage };
});
```

### Run Seamlessly
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

An `.njs` capsule is a single, self-contained, tamper-proof binary artifact containing:
1. All pre-compiled QuickJS bytecode and graph edge definitions.
2. SHA-256 integrity digests for every module and bundled asset.
3. Embedded static assets as raw bytes.
4. An optional Ed25519 publisher signature; verify with `--public-key` to establish a trusted publisher.

### Building & Verifying Capsules

```bash
# 1. Generate an Ed25519 publisher key
nio-js keys gen --email admin@example.com

# 2. Compile and sign a capsule
nio-js build app.ts --sign admin@example.com -o dist/app.njs

# 3. Inspect capsule contents and dependencies
nio-js inspect dist/app.njs

# 4. Cryptographically verify signature and integrity
nio-js verify dist/app.njs --public-key "$HOME/.nio/keys/admin@example.com.pub"

# 5. Run the capsule in production
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

Run pre-flight checks without starting the network listener:

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

### Q: How does the Hybrid Engine share memory between languages?
**A:** NioJS handles cross-boundary communication dynamically. HTTP payloads are routed via QuickJS. Native numeric imports use a restricted C ABI with zero to four double arguments and a double return value. Go also supports owned, zero-argument C string results. Python arguments and results use JSON through PyO3.

### Q: Is NioJS meant to replace Next.js or Nuxt?
**A:** No. NioJS is a backend worker runtime, optimized for high-throughput HTTP APIs, AI agent execution, and computationally heavy microservices. For full-stack apps, you should build your frontend (Vue, React, Svelte) statically and serve it alongside your NioJS backend API.

### Q: What exactly is a `.njs` capsule?
**A:** A capsule is a single-file deployment artifact with integrity checks and optional publisher signing. When you run `nio-js build`, it compiles your TypeScript/JavaScript into Ahead-of-Time (AOT) QuickJS bytecode and bundles it alongside downloaded ESM dependencies, FFI bindings, and asset files into one immutable binary payload. This binary runs without `node_modules` on the server. Publisher signing is optional; verify against a trusted public key to establish publisher identity.

### Q: How does `/** @native */` differ from Zig FFI?
**A:** `/** @native */` is a zero-config directive that tells the Rust host to dynamically compile your Javascript math loops into machine code under the hood. Zig FFI is used when you explicitly want to write raw C/Zig code and bind it manually to your project for things like cryptography.

### Q: Does NioJS support WebSockets?
**A:** WebSocket and SSE (Server-Sent Events) streaming are currently scheduled for our next major milestone (v0.4.0).

### Q: How do I test my service?
**A:** Use standard HTTP test clients or `curl`:
```bash
curl -i http://localhost:3000/health
```

---

*For upcoming roadmap milestones including native `nio-db` and `nio` integrations, check the [ROADMAP_V1.md on GitHub](https://github.com/nio-labs/nio-js/blob/main/ROADMAP_V1.md).*
