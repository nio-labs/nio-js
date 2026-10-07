# Local benchmark results

Run: 2026-10-07T01:52:00.414601+00:00

Machine: {'os': 'GNU/Linux 6.18.33.2-microsoft-standard-WSL2', 'architecture': 'x86_64', 'cpu': 'Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H', 'logical_cpus': 18, 'ram_gib': 7391.16015625}. Versions: {'nio': 'nio-js 0.2.0', 'node': 'v24.18.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, x86_64-unknown-linux-gnu)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 13.32 | 14.38 |
| node | 54.27 | 60.44 |
| bun | 25.08 | 19.25 |
| deno | 24.99 | 47.16 |
| nio-source | 24.84 | 14.39 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 8 | 15,700 | 9,322 | 11,046 | 11,594 | 0.701 | 1.201 | 0.989 | 0.924 |
| /callback | 8 | 12,758 | 8,272 | 10,406 | 11,882 | 0.852 | 1.325 | 1.032 | 0.881 |
| /json | 8 | 15,016 | 8,349 | 9,865 | 9,703 | 0.732 | 1.337 | 1.090 | 1.122 |
| /cpu | 8 | 14,389 | 4,689 | 5,139 | 6,147 | 0.757 | 2.657 | 2.221 | 1.580 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 15,700–15,700 requests/s.
* node: 9,322–9,322 requests/s.
* bun: 11,046–11,046 requests/s.
* deno: 11,594–11,594 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
