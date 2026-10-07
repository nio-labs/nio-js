# Local benchmark results

Run: 2026-10-07T15:14:23.002813+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.2.7', 'node': 'v22.21.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, aarch64-apple-darwin)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 8.11 | 11.81 |
| node | 57.25 | 46.88 |
| bun | 11.52 | 13.31 |
| deno | 27.76 | 34.59 |
| nio-source | 12.66 | 11.80 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 8 | 70,371 | 56,150 | 67,408 | 66,450 | 0.187 | 0.258 | 0.203 | 0.188 |
| /callback | 8 | 53,125 | 59,346 | 68,449 | 68,222 | 0.231 | 0.232 | 0.203 | 0.187 |
| /json | 8 | 46,270 | 51,395 | 64,798 | 58,240 | 0.256 | 0.251 | 0.196 | 0.202 |
| /cpu | 8 | 2,575 | 8,841 | 9,195 | 9,148 | 4.396 | 1.578 | 1.516 | 0.949 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 70,371–70,371 requests/s.
* node: 56,150–56,150 requests/s.
* bun: 67,408–67,408 requests/s.
* deno: 66,450–66,450 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
