# Local benchmark results

Run: 2026-10-06T03:02:59.073643+00:00

Machine: {'os': 'GNU/Linux 6.18.33.2-microsoft-standard-WSL2', 'architecture': 'x86_64', 'cpu': 'Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H Intel(R) Core(TM) Ultra 5 135H', 'logical_cpus': 18, 'ram_gib': 7391.16015625}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v24.18.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, x86_64-unknown-linux-gnu)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 23.64 | 13.75 |
| node | 50.34 | 60.35 |
| bun | 19.97 | 19.27 |
| deno | 23.80 | 47.10 |
| nio-source | 23.83 | 13.68 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 2,942 | 2,873 | 2,226 | 2,308 | 0.571 | 0.495 | 0.647 | 0.664 |
| /constant | 8 | 11,464 | 10,611 | 7,771 | 7,521 | 0.923 | 1.023 | 1.411 | 1.446 |
| /callback | 1 | 2,712 | 2,809 | 2,171 | 2,129 | 0.547 | 0.578 | 0.684 | 0.668 |
| /callback | 8 | 10,001 | 9,560 | 6,591 | 8,336 | 1.042 | 1.131 | 1.680 | 1.312 |
| /json | 1 | 2,315 | 2,482 | 2,158 | 1,288 | 0.621 | 0.589 | 0.642 | 1.131 |
| /json | 8 | 9,122 | 8,692 | 8,968 | 7,528 | 1.139 | 1.256 | 1.178 | 1.480 |
| /cpu | 1 | 2,507 | 1,816 | 1,679 | 1,504 | 0.565 | 0.743 | 0.855 | 1.038 |
| /cpu | 8 | 8,738 | 4,952 | 3,676 | 6,014 | 1.190 | 2.566 | 3.347 | 1.599 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 11,190–15,722 requests/s.
* node: 5,690–13,778 requests/s.
* bun: 6,284–14,811 requests/s.
* deno: 7,359–10,847 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
