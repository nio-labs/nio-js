# Local benchmark results

Run: 2026-10-05T17:36:53.105490+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v22.21.1', 'bun': '1.4.2'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 9.19 | 5.75 |
| node | 72.04 | 46.78 |
| bun | 13.61 | 13.26 |
| nio-source | 8.76 | 5.78 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 17,431 | 14,955 | 16,737 | 0.070 | 0.082 | 0.074 |
| /constant | 8 | 34,421 | 24,498 | 30,622 | 0.360 | 0.547 | 0.411 |
| /callback | 1 | 11,981 | 15,238 | 16,465 | 0.120 | 0.082 | 0.075 |
| /callback | 8 | 22,785 | 24,471 | 32,369 | 0.520 | 0.559 | 0.400 |
| /json | 1 | 9,725 | 14,449 | 16,145 | 0.135 | 0.084 | 0.077 |
| /json | 8 | 14,270 | 19,582 | 31,829 | 0.779 | 0.714 | 0.410 |
| /cpu | 1 | 143 | 5,583 | 6,287 | 7.401 | 0.203 | 0.176 |
| /cpu | 8 | 137 | 6,700 | 7,868 | 59.745 | 2.140 | 1.845 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 31,886–60,631 requests/s.
* node: 23,988–41,081 requests/s.
* bun: 29,750–40,557 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses two HTTP I/O threads. QuickJS still executes CPU work on one JavaScript thread. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
