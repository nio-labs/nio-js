# Local benchmark results

Run: 2026-10-05T20:52:33.006499+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v22.21.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, aarch64-apple-darwin)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 8.17 | 7.29 |
| node | 59.71 | 46.87 |
| bun | 11.57 | 13.46 |
| deno | 23.30 | 34.60 |
| nio-source | 10.09 | 7.30 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 23,894 | 21,565 | 23,277 | 22,162 | 0.052 | 0.059 | 0.053 | 0.054 |
| /constant | 8 | 70,563 | 60,389 | 65,973 | 62,653 | 0.185 | 0.221 | 0.205 | 0.198 |
| /callback | 1 | 16,717 | 22,345 | 23,248 | 22,543 | 0.072 | 0.053 | 0.053 | 0.054 |
| /callback | 8 | 49,686 | 56,502 | 68,209 | 64,497 | 0.239 | 0.235 | 0.191 | 0.187 |
| /json | 1 | 13,510 | 19,832 | 22,237 | 19,957 | 0.086 | 0.060 | 0.056 | 0.059 |
| /json | 8 | 40,265 | 50,170 | 63,823 | 53,156 | 0.288 | 0.251 | 0.201 | 0.218 |
| /cpu | 1 | 187 | 6,696 | 7,210 | 6,996 | 5.740 | 0.166 | 0.153 | 0.161 |
| /cpu | 8 | 551 | 8,922 | 9,177 | 9,158 | 15.374 | 1.568 | 1.521 | 0.935 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 68,943–72,381 requests/s.
* node: 56,799–61,173 requests/s.
* bun: 64,433–69,239 requests/s.
* deno: 62,444–67,135 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
