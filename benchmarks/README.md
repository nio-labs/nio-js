# nio-js / Node / Bun / Deno benchmarks

This suite measures the current preview on one machine. It compares native HTTP APIs without application frameworks, using the same response bytes and the same JavaScript workload functions.

## Reproduce

From the `nio-js` directory:

```sh
cargo build --release
cargo build --release --locked --manifest-path benchmarks/loadgen/Cargo.toml
mkdir -p benchmarks/.tools
# Download a matching Bun and Deno for your platform, e.g.
# curl -fL https://github.com/oven-sh/bun/releases/download/bun-v1.4.2/bun-linux-x64.zip -o benchmarks/.tools/bun.zip
# unzip -q benchmarks/.tools/bun.zip -d benchmarks/.tools && mv benchmarks/.tools/bun-linux-x64/bun benchmarks/.tools/bun-linux-x64/bun
# curl -fL https://github.com/denoland/deno/releases/download/v2.9.4/deno-x86_64-unknown-linux-gnu.zip -o benchmarks/.tools/deno.zip
# unzip -q benchmarks/.tools/deno.zip -d benchmarks/.tools && mv benchmarks/.tools/deno benchmarks/.tools/deno-linux-x64/deno
python3 benchmarks/run.py
```

Requires Node and Deno (or pre-downloaded into benchmarks/.tools), Python 3, a process-memory tool (Linux `ps` / macOS `ps`), and permission to bind localhost sockets. Bun stays inside the ignored tools directory. Pin Node and Deno to the versions recorded in `results.json` for a closer reproduction. nio-js uses a release build and the normal default limits, with multi-worker JavaScript dispatch.

Use `python3 benchmarks/run.py --no-baseline` for a four-runtime comparison even when a saved baseline binary exists. The optimization before/after comparison is retained in [OPTIMIZATION.md](OPTIMIZATION.md) and [optimization.json](optimization.json).

## Workloads

* `/constant`: cached Hello World bytes. nio-js uses its native constant-route path; Node uses `http.createServer`, Bun uses `Bun.serve`, and Deno uses `Deno.serve` with a cached byte buffer inside its handler. This compares these implementations, including nio-js's architectural shortcut. It does not compare pure JavaScript engine speed or Bun's optimized static `routes` API.
* `/callback`: Hello World returned by a JavaScript callback.
* `/json`: creates 20 objects and serializes them for each request.
* `/cpu`: the identical 100,000-iteration integer loop with `Math.imul`, followed by a text response. This intentionally emphasizes interpreted JavaScript versus JIT execution. It is one synthetic workload, not a general engine benchmark.

The Rust load generator has one blocking reqwest client per worker, HTTP keep-alive, and no proxy. Every measured request must return HTTP 200 and exactly match Node's reference body to count toward throughput. Failures count separately. Concurrent requests are closed-loop: each worker waits for a complete response before issuing another request. This does not model open-loop arrival rates, latency under overload, or independent remote clients.

For each workload and concurrency (1 and 8), each runtime gets one second of warmup and two seconds of measurement, repeated three times. Runtime order rotates one position each round. Runtimes run separately; the load generator runs on the same machine. The report uses median throughput and the median of each round's p95 latency. Raw p50/p95/p99 samples and throughput are in `results.json`. Short runs and shared-machine scheduling make small differences inconclusive. Higher-concurrency overload tests are excluded because nio-js's default admission limit is eight.

If `benchmarks/.tools/nio-before` exists, the runner also benchmarks that saved pre-optimization executable against the same capsule in the same run. Its hash is recorded in the results. To retain a baseline before changing the runtime, copy `target/release/nio-js` to that location before rebuilding. The original benchmark output is preserved in [BASELINE.md](BASELINE.md) and [baseline.json](baseline.json); its runtime order used a fixed random seed, as documented in that run. Compare the same-run before/after rows for the optimization effect rather than dividing numbers from separate sessions. The new default uses two HTTP I/O threads; the saved baseline used the host CPU count. Both use one JavaScript execution thread and the same request limits.

Three rounds do not fully balance four runtime positions when the optional baseline is included. Together with the observed variation, this limits conclusions about small differences. This is a local development benchmark, not a controlled capacity study.

Startup is measured ten times from process spawn to the first successful HTTP response, with warm filesystem caches. Each runtime gets one excluded launch, then samples are interleaved across runtimes. It includes readiness polling and local HTTP round-trip overhead. nio-js deployment startup uses a prebuilt `.njs` capsule; source startup is also shown. Capsule creation, runtime installation, dependency downloads, and empty-process startup are excluded. `python3 benchmarks/run.py --startup-only` refreshes these measurements in an existing results file and retains the initial startup samples separately.

Idle RSS is sampled 100 ms after that first response. Loaded RSS is sampled every 100 ms during each load test. These are process resident-memory observations in MiB, not precise peak allocation, total system memory cost, or a measurement of the load generator. RSS can grow after a longer workload or different GC timing.

Fast endpoints can be constrained by the generator, loopback networking, or client/server scheduling. Treat their numbers as observed end-to-end rates at the tested concurrency, not maximum server capacity. No TLS, external I/O, database, remote modules, filesystem operations, multiple server workers, production tuning, or browser APIs are measured. Response bodies are equivalent; HTTP implementations and header framing differ.

See [RESULTS.md](RESULTS.md) for the measured comparison.
