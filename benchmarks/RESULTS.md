# Local benchmark results

Run: 2026-10-07T14:38:02.781455+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.2.6', 'node': 'v22.21.1', 'bun': '1.4.2', 'deno': 'deno 2.9.4 (stable, release, aarch64-apple-darwin)'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 13.45 | 15.17 |
| node | 63.69 | 46.89 |
| bun | 11.67 | 13.44 |
| deno | 19.99 | 34.56 |
| nio-before | 11.64 | 15.16 |
| nio-source | 10.30 | 15.36 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | Deno req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms | Deno p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| /constant | 8 | 72,421 | 58,824 | 71,444 | 67,527 | 0.179 | 0.248 | 0.195 | 0.194 |
| /callback | 8 | 69,865 | 53,487 | 72,587 | 68,580 | 0.189 | 0.283 | 0.186 | 0.192 |
| /json | 8 | 70,622 | 52,744 | 68,286 | 61,626 | 0.185 | 0.254 | 0.193 | 0.203 |
| /cpu | 8 | 71,566 | 8,871 | 9,246 | 9,153 | 0.190 | 1.570 | 1.508 | 0.948 |

## Same-run nio-js before / after

| Route (concurrency 8) | Before req/s | After req/s | Change |
|---|---:|---:|---:|
| /constant | 70,992 | 72,421 | 1.02× |
| /callback | 55,016 | 69,865 | 1.27× |
| /json | 45,399 | 70,622 | 1.56× |
| /cpu | 6,392 | 71,566 | 11.20× |

Paired startup measurements: before 11.64 ms, after 13.45 ms. The HTTP changes do not improve every metric. The small CPU throughput difference should not be treated as an engine speedup: no JavaScript engine/compiler optimization was made, and observed performance varies across rounds.


## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 72,421–72,421 requests/s.
* node: 58,824–58,824 requests/s.
* bun: 71,444–71,444 requests/s.
* deno: 67,527–67,527 requests/s.

Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.

The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).
