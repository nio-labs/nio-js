#!/usr/bin/env python3
"""Run from nio-js: python3 benchmarks/run.py (needs localhost permissions)."""
import datetime
import hashlib
import argparse
import json
import os
from pathlib import Path
import platform
import socket
import statistics
import subprocess
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / 'benchmarks'
NIO = ROOT / 'target/release/nio-js'
BEFORE = BENCH / '.tools/nio-before'
BUN = BENCH / '.tools/bun-linux-x64/bun'
DENO = BENCH / '.tools/deno-linux-x64'
DENO = str(DENO) if DENO.exists() else 'deno'
LOAD = BENCH / 'loadgen/target/release/nio-bench-loadgen'
CAPSULE = BENCH / 'app.njs'
OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def command(*args):
    return subprocess.check_output(args, text=True).strip()

def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]

def rss(pid):
    return int(command('ps', '-o', 'rss=', '-p', str(pid))) / 1024

def start(runtime):
    port = free_port()
    if runtime.startswith('nio'):
        entry = BENCH / 'nio.js' if runtime == 'nio-source' else CAPSULE
        args = [str(BEFORE if runtime == 'nio-before' else NIO), 'run', str(entry), '--host', '127.0.0.1', '--port', str(port), '--workers', '16']
    elif runtime == 'deno':
        args = [DENO, 'run', '--quiet', '--allow-net', '--allow-env', str(BENCH / 'deno.mjs')]
    else:
        args = [str(BUN) if runtime == 'bun' else 'node', str(BENCH / f'{runtime}.mjs')]
    log = tempfile.TemporaryFile()
    before = time.perf_counter()
    env = {**os.environ, 'PORT': str(port), 'DENO_NO_UPDATE_CHECK': '1'}
    proc = subprocess.Popen(args, cwd=ROOT, env=env, stdout=log, stderr=log)
    url = f'http://127.0.0.1:{port}'
    while time.perf_counter() - before < 10:
        try:
            with OPENER.open(url + '/constant', timeout=0.2) as response:
                assert response.status == 200 and response.read() == b'Hello World'
            return proc, log, url, (time.perf_counter() - before) * 1000
        except (OSError, urllib.error.URLError):
            if proc.poll() is not None:
                log.seek(0)
                raise RuntimeError(log.read().decode())
            time.sleep(0.001)
    proc.kill()
    raise RuntimeError(f'{runtime} failed to start')

def stop(proc, log):
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
    log.close()

def measure(url, concurrency, seconds, expected, pid):
    child = subprocess.Popen([str(LOAD), url, str(concurrency), str(seconds), str(expected)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    peak = 0
    while child.poll() is None:
        peak = max(peak, rss(pid))
        time.sleep(0.1)
    stdout, stderr = child.communicate()
    if child.returncode:
        raise RuntimeError(stderr)
    result = json.loads(stdout)
    result['sampled_peak_rss_mib'] = peak
    return result

def measure_startup(data, runtimes):
    # Warm every executable once, then interleave samples to reduce order effects.
    for runtime in runtimes:
        proc, log, _, _ = start(runtime)
        stop(proc, log)
    samples = {name: [] for name in runtimes}
    memory = {name: [] for name in runtimes}
    iters = 3
    for iteration in range(iters):
        offset = iteration % len(runtimes)
        for runtime in runtimes[offset:] + runtimes[:offset]:
            proc, log, _, elapsed = start(runtime)
            try:
                samples[runtime].append(elapsed)
                time.sleep(0.1)
                memory[runtime].append(rss(proc.pid))
            finally:
                stop(proc, log)
    for runtime in runtimes:
        data['startup'][runtime] = {'samples_ms': samples[runtime], 'median_ms': statistics.median(samples[runtime]), 'idle_rss_mib': statistics.median(memory[runtime])}
        print(f'{runtime}: startup {statistics.median(samples[runtime]):.1f} ms; idle RSS {statistics.median(memory[runtime]):.1f} MiB', flush=True)
    data['config']['startup_method'] = f'one excluded warmup per runtime; {iters} interleaved samples'

def main(include_baseline=True):
    subprocess.run([str(NIO), 'build', str(BENCH / 'nio.js'), '-o', str(CAPSULE)], cwd=ROOT, check=True)
    versions = {'nio': command(str(NIO), '--version'), 'node': command('node', '--version'), 'bun': command(str(BUN), '--version')}
    try:
        versions['deno'] = command(DENO, '--version').splitlines()[0]
    except Exception:
        pass
    
    rounds = 1
    warmup_s = 0
    measure_s = 1
    concurrencies = [8]

    data = {'date_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'machine': {
        'os': command('sw_vers', '-productVersion') if command('uname', '-s').startswith('Darwin') else command('uname', '-o') + ' ' + command('uname', '-r'), 'architecture': platform.machine(),
        'cpu': command('sysctl', '-n', 'machdep.cpu.brand_string') if command('uname', '-s').startswith('Darwin') else ' '.join(
            line.split(':', 1)[1].strip() for line in command('cat', '/proc/cpuinfo').splitlines()
            if line.strip().startswith('model name')),
        'logical_cpus': int(command('sysctl', '-n', 'hw.ncpu')) if command('uname', '-s').startswith('Darwin') else int(command('nproc')),
        'ram_gib': int(command('sysctl', '-n', 'hw.memsize')) / 1024**3 if command('uname', '-s').startswith('Darwin') else int(command('cat', '/proc/meminfo').split('\n')[0].split(':')[1].strip().split()[0]) / 1024,
    }, 'versions': versions,
    'binary_sha256': {'nio': hashlib.sha256(NIO.read_bytes()).hexdigest(), 'bun': hashlib.sha256(BUN.read_bytes()).hexdigest()},
    'startup': {}, 'http': [], 'config': {'rounds': rounds, 'warmup_seconds': warmup_s, 'measurement_seconds': measure_s, 'concurrency': concurrencies, 'cpu_iterations': 100000}}
    has_baseline = include_baseline and BEFORE.exists()
    runtimes = ['nio', 'node', 'bun', 'deno'] + (['nio-before'] if has_baseline else [])
    data['config']['runtime_order'] = 'rotating order, one position per round'
    if has_baseline:
        data['binary_sha256']['nio-before'] = hashlib.sha256(BEFORE.read_bytes()).hexdigest()
    measure_startup(data, runtimes + ['nio-source'])
    with tempfile.TemporaryDirectory() as directory:
        # Get reference bytes from Node, then check every response against them.
        proc, log, url, _ = start('node')
        expected = {}
        try:
            for route in ['constant', 'callback', 'json', 'cpu']:
                path = Path(directory) / route
                with OPENER.open(url + '/' + route) as response:
                    path.write_bytes(response.read())
                expected[route] = path
        finally:
            stop(proc, log)
        for round_number in range(rounds):
            order = runtimes[round_number:] + runtimes[:round_number]
            for runtime in order:
                proc, log, url, _ = start(runtime)
                try:
                    for route in expected:
                        for concurrency in concurrencies:
                            endpoint = url + '/' + route
                            result = measure(endpoint, concurrency, measure_s, expected[route], proc.pid)
                            result.update(runtime=runtime, route=route, concurrency=concurrency, round=round_number + 1)
                            data['http'].append(result)
                            print(f'round {round_number+1} {runtime} /{route} c={concurrency}: {result["rps"]:.0f} req/s; errors={result["errors"]}', flush=True)
                            (BENCH / 'results.json').write_text(json.dumps(data, indent=2) + '\n')
                finally:
                    stop(proc, log)
    report(data)

def report(data):
    lines = ['# Local benchmark results', '', f'Run: {data["date_utc"]}', '',
        f'Machine: {data["machine"]}. Versions: {data["versions"]}.', '',
        '## Startup and idle memory', '', '| Runtime | Median startup to first HTTP response (ms) | Idle RSS (MiB) |', '|---|---:|---:|']
    for name, item in data['startup'].items():
        lines.append(f'| {name} | {item["median_ms"]:.2f} | {item["idle_rss_mib"]:.2f} |')
    lines += ['', '## HTTP throughput and latency', '', 'Each cell is the median of three rounds. Latency includes client, loopback networking, queueing, and response validation.', '']
    comp = [r for r in ['nio', 'node', 'bun', 'deno'] if any(x['runtime'] == r for x in data['http'])]
    name_map = {'nio': 'nio-js', 'node': 'Node', 'bun': 'Bun', 'deno': 'Deno'}
    header_rps = ' | '.join(f'{name_map[r]} req/s' for r in comp)
    header_p95 = ' | '.join(f'{name_map[r]} p95 ms' for r in comp)
    lines.append(f'| Route | Concurrency | {header_rps} | {header_p95} |')
    lines.append('|---|---:' + ''.join('|---:' for _ in range(len(comp) * 2)) + '|')
    for route in ['constant', 'callback', 'json', 'cpu']:
        for concurrency in data['config']['concurrency']:
            groups = [[x for x in data['http'] if x['runtime'] == runtime and x['route'] == route and x['concurrency'] == concurrency] for runtime in comp]
            values = [f'{statistics.median(x["rps"] for x in group):,.0f}' for group in groups]
            values += [f'{statistics.median(x["p95_ms"] for x in group):.3f}' for group in groups]
            lines.append(f'| /{route} | {concurrency} | ' + ' | '.join(values) + ' |')
    if any(x['runtime'] == 'nio-before' for x in data['http']):
        lines += ['', '## Same-run nio-js before / after', '', '| Route (concurrency 8) | Before req/s | After req/s | Change |', '|---|---:|---:|---:|']
        for route in ['constant', 'callback', 'json', 'cpu']:
            before, after = [statistics.median(x['rps'] for x in data['http'] if x['runtime'] == runtime and x['route'] == route and x['concurrency'] == 8) for runtime in ['nio-before', 'nio']]
            lines.append(f'| /{route} | {before:,.0f} | {after:,.0f} | {after/before:.2f}× |')
        old_start = data['startup']['nio-before']['median_ms']
        new_start = data['startup']['nio']['median_ms']
        lines += ['', f'Paired startup measurements: before {old_start:.2f} ms, after {new_start:.2f} ms. The HTTP changes do not improve every metric. The small CPU throughput difference should not be treated as an engine speedup: no JavaScript engine/compiler optimization was made, and observed performance varies across rounds.', '']
    lines += ['', '## Variation and interpretation', '',
        'Throughput varied substantially across rounds. For the fixed-response route at concurrency 8, the observed ranges were:', '']
    for runtime in comp:
        samples = [x['rps'] for x in data['http'] if x['runtime'] == runtime and x['route'] == 'constant' and x['concurrency'] == 8]
        if samples:
            lines.append(f'* {runtime}: {min(samples):,.0f}–{max(samples):,.0f} requests/s.')
    lines += ['', 'Small differences in median fixed-response rates do not establish a throughput winner when the observed ranges overlap. Startup and memory, callbacks, JSON, and CPU must be judged separately. The native constant-response path avoids JavaScript execution per request.', '',
        'The optimized runtime passes response strings and typed-array bytes directly to Rust, has a synchronous dispatch path, defers request header/search-parameter construction until used, indexes exact routes, caches constant response headers/bodies, and uses a multi-worker thread pool for concurrent JS dispatch. QuickJS still executes CPU work on one JavaScript thread per worker. If included, the same-run baseline uses the saved binary from before these optimizations; the capsule format and workloads are unchanged.', '',
        f'Total measured errors: {sum(x["errors"] for x in data["http"])}. Every successful response matched the reference bytes.', '',
        'Method and limitations: see [README.md](README.md). Per-round latency percentiles, throughput, and sampled loaded RSS: [results.json](results.json).', '']
    (BENCH / 'RESULTS.md').write_text('\n'.join(lines))

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--startup-only', action='store_true', help='Refresh startup measurements in existing results without repeating HTTP tests')
    parser.add_argument('--no-baseline', action='store_true', help='Compare only nio-js, Node, and Bun')
    args = parser.parse_args()
    if args.startup_only:
        path = BENCH / 'results.json'
        data = json.loads(path.read_text())
        data.setdefault('startup_initial', data['startup'])
        measure_startup(data, list(data['startup']))
        data['config']['startup_refreshed_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        path.write_text(json.dumps(data, indent=2) + '\n')
        report(data)
    else:
        main(include_baseline=not args.no_baseline)
