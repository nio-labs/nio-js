# Local benchmark results

Run: 2026-10-06T02:36:16.806696+00:00

Machine: {'os': 'GNU/Linux 6.18.33.2-microsoft-standard-WSL2', 'architecture': 'x86_64', 'cpu': 'Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H', 'logical_cpus': 18, 'ram_gib': 7391.16015625}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v24.18.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, x86_64-unknown-linux-gnu)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 23.70 | 11.80 |
| node | 49.88 | 60.39 |
| bun | 25.41 | 19.28 |
| deno | 22.57 | 47.20 |
| nio-source | 19.44 | 11.70 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 3,245 | 2,531 | 2,150 | 2,644 | 0.524 | 0.494 | 0.659 | 0.620 |
| /constant | 8 | 10,778 | 11,541 | 9,540 | 7,164 | 0.977 | 0.936 | 1.146 | 1.621 |
| /callback | 1 | 2,647 | 2,838 | 1,934 | 2,072 | 0.574 | 0.548 | 0.694 | 0.633 |
| /callback | 8 | 9,890 | 11,594 | 7,421 | 8,569 | 1.062 | 0.940 | 1.480 | 1.271 |
| /json | 1 | 2,257 | 2,662 | 1,478 | 2,052 | 0.679 | 0.574 | 0.877 | 0.650 |
| /json | 8 | 9,203 | 9,738 | 6,553 | 5,549 | 1.136 | 1.125 | 1.701 | 2.180 |
| /cpu | 1 | 2,409 | 2,164 | 1,577 | 1,785 | 0.650 | 0.675 | 0.870 | 0.763 |
| /cpu | 8 | 9,545 | 5,928 | 5,015 | 6,136 | 1.090 | 2.007 | 2.280 | 1.565 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 9,930–15,253 requests/s.
* node: 5,072–14,131 requests/s.
* bun: 7,110–14,860 requests/s.
* deno: 6,694–9,204 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
