# Local benchmark results

Run: 2026-10-06T04:02:57.240936+00:00

Machine: {'os': 'GNU/Linux 6.18.33.2-microsoft-standard-WSL2', 'architecture': 'x86_64', 'cpu': 'Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H', 'logical_cpus': 18, 'ram_gib': 7391.16015625}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v24.18.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, x86_64-unknown-linux-gnu)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 23.49 | 11.77 |
| node | 48.82 | 60.43 |
| bun | 25.51 | 19.28 |
| deno | 25.55 | 47.36 |
| nio-source | 23.85 | 11.72 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 2,386 | 2,734 | 2,540 | 2,094 | 0.687 | 0.566 | 0.565 | 0.686 |
| /constant | 8 | 10,267 | 11,567 | 8,744 | 7,250 | 1.029 | 0.929 | 1.241 | 1.502 |
| /callback | 1 | 2,712 | 2,983 | 2,296 | 2,255 | 0.537 | 0.480 | 0.576 | 0.645 |
| /callback | 8 | 10,183 | 10,769 | 7,214 | 7,858 | 1.169 | 1.011 | 1.546 | 1.428 |
| /json | 1 | 2,695 | 2,246 | 1,548 | 1,758 | 0.565 | 0.774 | 0.980 | 0.802 |
| /json | 8 | 8,499 | 8,886 | 7,077 | 7,960 | 1.232 | 1.227 | 1.563 | 1.366 |
| /cpu | 1 | 2,275 | 1,687 | 1,468 | 1,479 | 0.642 | 0.825 | 0.932 | 0.902 |
| /cpu | 8 | 8,102 | 5,344 | 4,411 | 4,670 | 1.280 | 2.295 | 2.649 | 2.085 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 9,383–15,455 requests/s.
* node: 5,774–11,620 requests/s.
* bun: 6,153–9,582 requests/s.
* deno: 6,931–9,742 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
