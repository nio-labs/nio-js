# nio-js: The Hybrid, Agent-Native Worker Runtime

> **Vision:** TypeScript control plane. Native Rust speed. Python AI. Zero-config and agent-native.

---

## 1. Executive Summary

`nio-js` is a lightweight, high-performance edge worker runtime designed to beat existing runtimes (Bun, Node, Deno) across every benchmark criterion, while delivering the first truly agent-native developer experience.

Instead of fighting V8 or JavaScriptCore on general JIT compilation, `nio-js` employs a **hybrid architecture**:
- **TypeScript/JavaScript** handles API routing, HTTP framing, middleware, and request/response orchestration.
- **Native Rust** accelerates CPU-intensive algorithms (loops, cryptography, parsing, numerical crunching), outperforming V8/JSC JIT by 3x–4x.
- **Python** (modular) enables in-process AI/ML inference (PyTorch, NumPy, Hugging Face) without separate microservice overhead.
- **Agent-Native Tooling** provides instant 8ms test feedback, zero-config single-file execution, and machine-readable JSON diagnostics for AI coding agents.

```
                             [ Developer or Coding Agent ]
                                           │
                                     writes code in
                                           ▼
                             ┌───────────────────────────┐
                             │       TypeScript          │
                             │   (APIs, Routes, Logic)   │
                             └─────────────┬─────────────┘
                                           │
                   ┌───────────────────────┴───────────────────────┐
                   ▼                                               ▼
     ┌──────────────────────────┐                    ┌──────────────────────────┐
     │       Native Rust        │                    │        Python AI         │
     │   (Compute, Crypto,      │                    │   (PyTorch, NumPy,       │
     │    100k+ Loops, SIMD)    │                    │    Inference Models)     │
     └─────────────┬────────────┘                    └─────────────┬────────────┘
                   │                                               │
                   └───────────────────────┬───────────────────────┘
                                           ▼
                        ┌─────────────────────────────────────┐
                        │          nio-js Host Core           │
                        │    (Tokio, Axum, QuickJS, Wasm)     │
                        │                                     │
                        │  ⚡ 8ms Cold Start   💾 8.7MB RAM    │
                        │  🤖 Agent-JSON       🔒 Sandbox     │
                        │  📦 .njs Capsule     🚀 3x vs Bun   │
                        └─────────────────────────────────────┘
```

---

## 2. Competitive Benchmark Targets

| Metric | nio-js (Current) | **nio-js (Hybrid Target)** | Bun | Node | Deno | Strategic Advantage |
|---|---|---|---|---|---|---|
| **Startup** | **8.15 ms** | **~8.15 ms** | 13.53 ms | 59.92 ms | 22.35 ms | 🏆 Dominates via QuickJS instant init |
| **Idle Memory** | **8.72 MiB** | **~8.75 MiB** | 13.48 MiB | 46.84 MiB | 34.58 MiB | 🏆 Dominates (Node is 5x heavier) |
| **/constant** | 67,244 req/s | **~68,000 req/s** | 72,242 req/s | 63,037 req/s | 67,476 req/s | ⚖️ On par via Rust route caching |
| **/callback** | 50,674 req/s | **~50,000 req/s** | 71,144 req/s | 63,664 req/s | 64,630 req/s | 🥈 Competitive |
| **/json** | 42,360 req/s | **~42,000 req/s** | 64,689 req/s | 51,838 req/s | 56,031 req/s | 🥈 Competitive |
| **/cpu (100k loop)** | 760 req/s | **25,000–40,000+ req/s** | 9,282 req/s | 9,001 req/s | 9,151 req/s | 🚀 **3x–4x faster than Bun and Node** |

---

## 3. Step-by-Step Implementation Plan

### Step 1: Core Native Acceleration & Server Worker Pinning (Phase 1)
*Objective: Eliminate the `/cpu` benchmark gap and maximize multi-threaded host throughput.*

- [ ] **1.1 Physical Core Detection & CPU Pinning (`src/server.rs`):**
  - Use physical CPU core counts (`num_cpus::get_physical()`) instead of logical cores for the default worker count to avoid SMT/hyperthreading cache thrashing.
  - Pin each worker thread to a dedicated physical core (using `core_affinity` or `pthread_setaffinity_np` on Linux).
  - Keep `per_worker_queue` compact (16–32) for compute workloads to prevent queue pile-up behind slow workers.

- [ ] **1.2 Native Bridge Registry (`src/engine.rs`):**
  - Implement a `__nioNative` global bridge in QuickJS context initialization.
  - Expose a registry where native Rust closures can be invoked from JavaScript.
  - Add zero-copy passing for numeric primitives and `TypedArray` buffers (e.g., `Uint8Array`, `Int32Array`).

- [ ] **1.3 AST Native Directive & Transpiler Hook (`src/prepare.rs`):**
  - Add AST parsing in `prepare.rs` using `oxc_parser` to recognize native execution directives (`/** @native */` or `.rs` companion modules).
  - Transform calls to designated functions into `__nioNative.invoke(id, ...)` dispatch calls.

- [ ] **1.4 Benchmark Suite Update (`benchmarks/`):**
  - Update `benchmarks/workload.js` and `benchmarks/run.py` to evaluate the hybrid compute path.
  - Validate that the 100k-iteration integer loop reaches **>25,000 req/s** at concurrency 8.

---

### Step 2: Agent-Native Tooling & Sandboxed CLI (Phase 2)
*Objective: Make `nio-js` the fastest and most reliable runtime for AI coding agents (Claude Code, Cursor, Devin, Antigravity).*

- [ ] **2.1 Structured Agent JSON Diagnostics (`src/main.rs`):**
  - Add `--format=agent-json` flag to `nio-js check` and `nio-js build`.
  - Emit machine-readable diagnostics containing:
    ```json
    {
      "status": "error",
      "file": "app.ts",
      "line": 14,
      "col": 5,
      "code": "TS2345",
      "message": "Type mismatch: expected number, got string",
      "suggested_fix": "Number(id)"
    }
    ```
  - Enables agents to self-correct in a single LLM turn without parsing noisy terminal output.

- [ ] **2.2 Zero-Config Single-File Runner:**
  - Ensure `nio-js run <file.ts>` resolves URL modules (`https://esm.sh/...`), builds, and executes with zero project boilerplate (no `package.json` or `node_modules` required).

- [ ] **2.3 Deterministic Sandbox Hardening:**
  - Enforce `--memory-mb`, `--timeout-ms`, and `--allow-net` defaults so coding agents can execute generated code safely without container overhead.

- [ ] **2.4 Built-In MCP (Model Context Protocol) Server:**
  - Implement `nio-js mcp` command to expose evaluation, validation, and benchmarking tools directly to AI agent harnesses.

---

### Step 3: Modular Python AI Bridge (Phase 3)
*Objective: Allow TypeScript services to invoke Python AI/ML models in-process without microservice latency or base runtime bloat.*

- [ ] **3.1 Modular Feature Flag (`Cargo.toml`):**
  - Put Python embedding behind a Cargo feature flag:
    ```toml
    [features]
    default = []
    python = ["dep:pyo3"]
    ```
  - Ensures the base `nio-js` binary remains ultra-light (8 MiB idle RAM, 8 ms startup) and does not force Python dev headers on systems that only need TS/Rust.

- [ ] **3.2 In-Process Python Bridge (`src/python.rs` / `src/engine.rs`):**
  - Embed CPython via `pyo3` when enabled.
  - Expose a bridge allowing TS code to import `.py` files:
    ```ts
    import { runInference } from './model.py';
    ```
  - Manage Python GIL safely across worker threads.

- [ ] **3.3 Zero-Copy Buffer & Tensor Transfer:**
  - Enable direct buffer sharing between QuickJS `ArrayBuffer` and Python `memoryview` / `numpy.ndarray` for efficient vector and image passing.

---

## 4. Unified CLI Specification

All tooling and commands strictly use the **`nio-js`** binary:

```bash
# Run a TypeScript or Capsule worker
nio-js run app.ts
nio-js run app.njs --port 3000

# Build single-file verifiable capsule
nio-js build app.ts -o app.njs

# Machine-readable diagnostics for AI agents
nio-js check app.ts --format=agent-json

# Start MCP server for AI coding assistants
nio-js mcp

# Inspect and verify capsule integrity
nio-js inspect app.njs
nio-js verify app.njs
```

---

## 5. Execution Order & Milestones

1. **Milestone 1:** Step 1 (`__nioNative` bridge + worker core pinning) $\to$ Benchmark run against Bun/Node.
2. **Milestone 2:** Step 2 (Agent JSON diagnostics + sandbox runner) $\to$ Agent workflow testing.
3. **Milestone 3:** Step 3 (Feature-gated `pyo3` Python bridge) $\to$ In-process PyTorch/NumPy example.
