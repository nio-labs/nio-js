# Local benchmark results

Run: 2026-10-05T17:11:54.633237+00:00

Machine: {'os': '26.3.1', 'architecture': 'arm64', 'cpu': 'Apple M1', 'logical_cpus': 8, 'ram_gib': 8.0}. Versions: {'nio': 'nio-js 0.1.0', 'node': 'v22.21.1', 'bun': '1.4.2'}.

## Startup and idle memory

| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |
|---|---:|---:|
| nio | 11.33 | 5.95 |
| nio-source | 11.24 | 5.98 |
| node | 58.60 | 46.76 |
| bun | 17.36 | 13.30 |

## HTTP throughput and latency

Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.

| Route | Concurrency | nio-js req/s | Node req/s | Bun req/s | nio-js p95 ms | Node p95 ms | Bun p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| /constant | 1 | 18,561 | 19,968 | 19,256 | 0.071 | 0.062 | 0.067 |
| /constant | 8 | 42,208 | 44,211 | 41,395 | 0.296 | 0.309 | 0.318 |
| /callback | 1 | 10,759 | 20,113 | 19,895 | 0.126 | 0.066 | 0.064 |
| /callback | 8 | 20,109 | 45,955 | 47,620 | 0.581 | 0.305 | 0.286 |
| /json | 1 | 5,441 | 17,395 | 18,482 | 0.211 | 0.075 | 0.072 |
| /json | 8 | 6,290 | 36,879 | 42,867 | 1.782 | 0.360 | 0.304 |
| /cpu | 1 | 134 | 6,193 | 6,948 | 7.716 | 0.186 | 0.165 |
| /cpu | 8 | 134 | 7,930 | 8,661 | 67.417 | 1.787 | 1.640 |

## Variation and interpretation

Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:

* nio: 33,253–57,653 requests/s.
* node: 27,654–51,770 requests/s.
* bun: 26,578–56,029 requests/s.

These ranges overlap. Small differences in the median fixed-response rates do not establish a throughput winner. Later rounds were generally slower; this run does not identify the cause. Startup and idle memory favor nio-js in this test, while callback, JSON, and CPU throughput favor Node and Bun. The native constant-response path avoids JavaScript execution per request.

Code inspection suggests the callback bridge is a useful profiling target: nio-js serializes request metadata and converts response bytes through arrays, base64, and JSON before passing them back to Rust. This is an optimization hypothesis, not a measured attribution of the performance gap.

Total measured errors: 0. Every successful response matched the reference bytes.

Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [baseline.json](baseline.json).
