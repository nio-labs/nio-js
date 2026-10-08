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

/** @native */
function testHttpGetThroughput(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    const req = "GET /api/data?id=" + repeat + " HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    const res = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}";
    checksum += req.length + res.length;
  }
  return checksum;
}
results.push(measure('HTTP GET throughput', 1, 500, testHttpGetThroughput));

const smallJsonStr = JSON.stringify({
  id: "1234567890", name: "Performance Test Small", description: "This is a small JSON payload for parsing and stringifying.", tags: ["benchmark", "json", "parse", "performance", "small"], metadata: { createdAt: "2026-01-01T00:00:00Z", active: true, count: 42 }, padding: "x".repeat(700)
});
/** @native */
function testJsonParseSmall(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    checksum += JSON.parse(smallJsonStr).padding.length;
  }
  return checksum;
}
results.push(measure('JSON.parse small payload', 1, 500, testJsonParseSmall));

const largeJsonStr = JSON.stringify({
  items: Array.from({ length: 1000 }, (_, i) => ({ id: i, name: "Item " + i, value: i * 1.5, padding: "x".repeat(50) }))
});
/** @native */
function testJsonParseLarge(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    checksum += JSON.parse(largeJsonStr).items[0].id;
  }
  return checksum;
}
results.push(measure('JSON.parse large payload', 1, 20, testJsonParseLarge));

const smallJsonObj = JSON.parse(smallJsonStr);
/** @native */
function testJsonStringifySmall(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    checksum += JSON.stringify(smallJsonObj).length;
  }
  return checksum;
}
results.push(measure('JSON.stringify small object', 1, 500, testJsonStringifySmall));

const mediumJsonObj = { items: Array.from({ length: 100 }, (_, i) => ({ id: i, active: i % 2 === 0 })) };
/** @native */
function testJsonStringifyMedium(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    checksum += JSON.stringify(mediumJsonObj).length;
  }
  return checksum;
}
results.push(measure('JSON.stringify medium object', 1, 200, testJsonStringifyMedium));

const buf1kb = new Uint8Array(1024);
for (let i = 0; i < buf1kb.length; i++) buf1kb[i] = i & 0xff;
/** @native */
function testSha256Small(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    let a = 0x6a09e667, b = 0xbb67ae85, c = 0x3c6ef372, d = 0xa54ff53a;
    for (let offset = 0; offset < buf1kb.length; offset += 64) {
      const w0 = (buf1kb[offset] + (buf1kb[offset + 1] << 8)) | 0;
      const w1 = (buf1kb[offset + 4] + (buf1kb[offset + 5] << 8)) | 0;
      const x0 = ((a + b) + w0 + 0x428a2f98) | 0;
      const x1 = ((c + d) + w1 + 0x71374491) | 0;
      a = x0; b = x1;
      checksum += (a + b) >>> 0;
    }
  }
  return checksum;
}
results.push(measure('SHA 256 hashing small buffer', 1, 200, testSha256Small));

const buf64kb = new Uint8Array(64 * 1024);
for (let i = 0; i < buf64kb.length; i++) buf64kb[i] = i & 0xff;
/** @native */
function testSha256Large(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    let a = 0x6a09e667, b = 0xbb67ae85, c = 0x3c6ef372, d = 0xa54ff53a;
    for (let offset = 0; offset < buf64kb.length; offset += 64) {
      const w0 = (buf64kb[offset] + (buf64kb[offset + 1] << 8)) | 0;
      const w1 = (buf64kb[offset + 4] + (buf64kb[offset + 5] << 8)) | 0;
      const x0 = ((a + b) + w0 + 0x428a2f98) | 0;
      const x1 = ((c + d) + w1 + 0x71374491) | 0;
      a = x0; b = x1;
      checksum += (a + b) >>> 0;
    }
  }
  return checksum;
}
results.push(measure('SHA 256 hashing large buffer', 1, 10, testSha256Large));

/** @native */
function testBufferCopy(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    const dst = new Uint8Array(buf64kb.length);
    dst.set(buf64kb);
    checksum += dst[0] + dst[dst.length - 1];
  }
  return checksum;
}
results.push(measure('Buffer copy 64 KB', 1, 200, testBufferCopy));

const arrMapReduce = Array.from({ length: 10000 }, (_, i) => i);
/** @native */
function testArrayMapReduce(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    const mapped = arrMapReduce.map(x => x * 3 + 7);
    checksum += mapped.reduce((acc, val) => acc + val, 0);
  }
  return checksum;
}
results.push(measure('Array map plus reduce', 1, 50, testArrayMapReduce));

/** @native */
function testStringConcat(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    let str = "";
    for (let i = 0; i < 1000; i++) {
      str += i.toString();
    }
    checksum += str.length;
  }
  return checksum;
}
results.push(measure('String concatenation', 1, 100, testStringConcat));

/** @native */
function testIntegerLoop(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    let value = 0;
    for (let i = 0; i < 10000; i++) {
      value = (value + i * 3) % 997;
    }
    checksum += value;
  }
  return checksum;
}
results.push(measure('Integer loop plus arithmetic', 1, 100, testIntegerLoop));

const randomInts = Array.from({ length: 10000 }, (_, i) => Math.floor(((i * 137 + 7) % 1000)));
/** @native */
function testIntegerLoopRandom(repeatLimit) {
  let checksum = 0;
  for (let repeat = 0; repeat < repeatLimit; repeat++) {
    let value = 0;
    for (let i = 0; i < 10000; i++) {
      value = (value + randomInts[i]) % 997;
    }
    checksum += value;
  }
  return checksum;
}
results.push(measure('Integer loop with randomized input', 1, 100, testIntegerLoopRandom));

console.log(JSON.stringify({ results }));
