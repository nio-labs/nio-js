# Local benchmark results

Run: 2026-10-05T17:28:21.026632+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v22.21.1', 'bun': '1.4.2'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 10.31 | 5.75 |
| node | 73.27 | 46.87 |
| bun | 12.25 | 13.28 |
| nio-before | 8.57 | 5.94 |
| nio-source | 8.68 | 5.78 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 17,905 | 15,490 | 17,487 | 0.077 | 0.080 | 0.071 |
| /constant | 8 | 53,097 | 26,355 | 40,444 | 0.230 | 0.544 | 0.316 |
| /callback | 1 | 12,807 | 15,695 | 18,219 | 0.110 | 0.079 | 0.073 |
| /callback | 8 | 23,031 | 25,094 | 34,553 | 0.524 | 0.533 | 0.361 |
| /json | 1 | 9,985 | 13,149 | 16,369 | 0.131 | 0.089 | 0.076 |
| /json | 8 | 17,122 | 19,382 | 31,299 | 0.660 | 0.714 | 0.422 |
| /cpu | 1 | 138 | 5,504 | 6,172 | 7.596 | 0.197 | 0.182 |
| /cpu | 8 | 138 | 7,155 | 8,049 | 61.517 | 1.981 | 1.762 |

## Same-run nio-js before / after

| Route (concurrency 8) | Before req/s | After req/s | Change |
|---|---:|---:|---:|
| /constant | 32,525 | 53,097 | 1.63× |
| /callback | 15,585 | 23,031 | 1.48× |
| /json | 5,652 | 17,122 | 3.03× |
| /cpu | 125 | 138 | 1.11× |

Paired startup measurements: before 8.57 ms, after 10.31 ms. The HTTP changes do not improve every metric. The small CPU throughput difference should not be treated as an engine speedup: no JavaScript engine/compiler optimization was made, and observed performance varies across rounds.


## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 33,293–58,188 requests/s.
* node: 25,716–39,115 requests/s.
* bun: 31,682–45,187 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses two HTTP I/O threads. QuickJS still executes CPU work on one JavaScript thread. The same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [optimization.json](optimization.json).
