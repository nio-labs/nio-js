import * as rs from './suite.zig';

const samples = 9;

if (typeof performance === 'undefined' || typeof performance.now !== 'function') {
  let startTime;
  const origin = Date.now();
  globalThis.performance = {
    now() {
      if (startTime === undefined) startTime = Date.now();
      return Date.now() - startTime;
    },
    timeOrigin: origin,
  };
}

function measure(name, units, batch, run) {
  for (let i = 0; i < 2; i++) run(batch);
  const times = [];
  let checksum;
  for (let i = 0; i < samples; i++) {
    const start = performance.now();
    checksum = run(batch);
    times.push(performance.now() - start);
  }
  times.sort((a, b) => a - b);
  const medianMs = times[Math.floor(times.length / 2)];
  return {
    name,
    opsPerSecond: Math.round((units * batch * 1000) / medianMs),
    medianMs,
    checksum: typeof checksum === 'number' ? checksum.toFixed(6) : String(checksum),
  };
}

const results = [];

results.push(measure('HTTP GET throughput', 1, 500, rs.testHttpGetThroughput));
results.push(measure('JSON.parse small payload', 1, 500, rs.testJsonParseSmall));
results.push(measure('JSON.parse large payload', 1, 20, rs.testJsonParseLarge));
results.push(measure('JSON.stringify small object', 1, 500, rs.testJsonStringifySmall));
results.push(measure('JSON.stringify medium object', 1, 200, rs.testJsonStringifyMedium));
results.push(measure('SHA 256 hashing small buffer', 1, 200, rs.testSha256Small));
results.push(measure('SHA 256 hashing large buffer', 1, 10, rs.testSha256Large));
results.push(measure('Buffer copy 64 KB', 1, 200, rs.testBufferCopy));
results.push(measure('Array map plus reduce', 1, 50, rs.testArrayMapReduce));
results.push(measure('String concatenation', 1, 100, rs.testStringConcat));
results.push(measure('Integer loop plus arithmetic', 1, 100, rs.testIntegerLoop));
results.push(measure('Integer loop with randomized input', 1, 100, rs.testIntegerLoopRandom));

console.log(JSON.stringify({ results }));
