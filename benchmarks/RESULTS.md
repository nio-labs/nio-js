# Local benchmark results

Run: 2026-10-08T04:54:10.500871+00:00

Machine: {'os': 'GNU/Linux 6.18.33.2-microsoft-standard-WSL2', 'architecture': 'x86_64', 'cpu': 'Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H', 'logical_cpus': 18, 'ram_gib': 7391.16796875}. Versions: {'nio': 'nio-js 0.2.7', 'node': 'v24.18.1', 'bun': '1.4.2', 'deno': 'deno 2.9.7 (stable, release, x86_64-unknown-linux-gnu)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 26.02 | 15.98 |
| node | 97.76 | 60.46 |
| bun | 24.54 | 21.76 |
| deno | 31.23 | 37.02 |
| nio-source | 25.53 | 16.18 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 8 | 7,828 | 5,499 | 6,877 | 8,110 | 1.486 | 2.213 | 1.613 | 1.332 |
| /callback | 8 | 5,813 | 5,639 | 7,270 | 7,716 | 1.861 | 2.130 | 1.549 | 1.406 |
| /json | 8 | 5,081 | 5,321 | 6,860 | 6,186 | 2.131 | 2.194 | 1.620 | 1.810 |
| /cpu | 8 | 4,216 | 2,933 | 3,667 | 4,069 | 2.511 | 4.418 | 3.173 | 2.406 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 7,828–7,828 requests/s.
* node: 5,499–5,499 requests/s.
* bun: 6,877–6,877 requests/s.
* deno: 8,110–8,110 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
