# Local benchmark results

Run: 2026-10-05T21:13:51.672233+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v22.21.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, aarch64-apple-darwin)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 8.15 | 8.72 |
| node | 59.92 | 46.84 |
| bun | 13.53 | 13.48 |
| deno | 22.35 | 34.58 |
| nio-source | 8.80 | 8.72 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 23,641 | 22,335 | 23,864 | 22,600 | 0.052 | 0.053 | 0.049 | 0.051 |
| /constant | 8 | 67,244 | 63,037 | 72,242 | 67,476 | 0.196 | 0.208 | 0.182 | 0.180 |
| /callback | 1 | 18,057 | 22,639 | 24,036 | 22,870 | 0.070 | 0.051 | 0.048 | 0.050 |
| /callback | 8 | 50,674 | 63,664 | 71,144 | 64,630 | 0.239 | 0.199 | 0.182 | 0.197 |
| /json | 1 | 14,056 | 20,349 | 22,218 | 19,858 | 0.088 | 0.056 | 0.055 | 0.061 |
| /json | 8 | 42,360 | 51,838 | 64,689 | 56,031 | 0.279 | 0.242 | 0.199 | 0.205 |
| /cpu | 1 | 174 | 6,778 | 7,119 | 7,031 | 6.375 | 0.156 | 0.157 | 0.155 |
| /cpu | 8 | 760 | 9,001 | 9,282 | 9,151 | 13.803 | 1.549 | 1.502 | 0.931 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 59,416–73,978 requests/s.
* node: 50,337–66,131 requests/s.
* bun: 63,185–73,419 requests/s.
* deno: 56,421–68,352 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
