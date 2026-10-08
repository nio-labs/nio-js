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

results.push(measure('float math (iterations/s)', 1, 1, () => {
  let checksum = 0;
  for (let repeat = 0; repeat < 1; repeat++) {
    let value = 0.125;
    for (let i = 0; i < 250_000; i++) {
      value = value * 1.0000001 + 0.000001;
      if (value >= 1000) value -= 1000;
    }
    checksum += value;
  }
  return checksum;
}));

results.push(measure('string build (items/s)', 1_000, 100, () => {
  let checksum = 0;
  for (let repeat = 0; repeat < 100; repeat++) {
    const parts = [];
    for (let i = 0; i < 1_000; i++) parts.push(i.toString(36));
    const text = parts.join(':');
    checksum += text.length + text.charCodeAt(100);
  }
  return checksum;
}));

results.push(measure('array map/reduce (items/s)', 10_000, 200, () => {
  let checksum = 0;
  for (let repeat = 0; repeat < 200; repeat++) {
    const values = Array.from({ length: 10_000 }, (_, i) => i);
    const mapped = values.map((value) => (value * 3 + 7) % 997);
    checksum += mapped.reduce((sum, value) => sum + value, 0);
  }
  return checksum;
}));

results.push(measure('map insert/read (items/s)', 7_500, 200, () => {
  let checksum = 0;
  for (let repeat = 0; repeat < 200; repeat++) {
    const map = new Map();
    for (let i = 0; i < 2_500; i++) map.set(i, i * 2);
    for (let i = 0; i < 5_000; i++) checksum += map.get(i % 2_500) ?? 0;
  }
  return checksum;
}));

results.push(measure('JSON parse/stringify (ops/s)', 1, 1_500, () => {
  let checksum = 0;
  const payload = {
    id: 1,
    label: 'nio-js-benchmark',
    values: Array.from({ length: 512 }, (_, i) => ({ id: i, score: i * 1.25 })),
  };
  const text = JSON.stringify(payload);
  for (let repeat = 0; repeat < 1_500; repeat++) {
    checksum += JSON.parse(text).values[511].id;
    checksum += JSON.stringify(payload).length;
  }
  return checksum;
}));

results.push(measure('HTTP object build (ops/s)', 1, 2_000, () => {
  let checksum = 0;
  const request = {
    method: 'GET',
    url: '/api/benchmark?limit=512&offset=0',
    headers: { accept: 'application/json', connection: 'keep-alive' },
    body: null,
  };
  for (let repeat = 0; repeat < 2_000; repeat++) {
    const response = {
      status: 200,
      statusText: 'OK',
      headers: { 'content-type': 'application/json', date: 'Mon, 01 Jan 2024 00:00:00 GMT' },
      body: JSON.stringify({ ok: true, ids: Array.from({ length: 64 }, (_, i) => i) }),
    };
    checksum += response.body.length + response.status;
    checksum += request.url.length + Object.keys(request.headers).length;
  }
  return checksum;
}));

results.push(measure('SHA-like digest (ops/s)', 1, 400, () => {
  let checksum = 0;
  const data = new Uint8Array(64 * 1024);
  for (let i = 0; i < data.length; i++) data[i] = i & 0xff;
  const h0 = 0x6a09e667;
  const h1 = 0xbb67ae85;
  const h2 = 0x3c6ef372;
  const h3 = 0xa54ff53a;
  for (let repeat = 0; repeat < 400; repeat++) {
    let a = h0,
      b = h1,
      c = h2,
      d = h3;
    for (let offset = 0; offset < data.length; offset += 64) {
      const w0 = (data[offset] + (data[offset + 1] << 8) + (data[offset + 2] << 16) + (data[offset + 3] << 24)) | 0;
      const w1 = (data[offset + 4] + (data[offset + 5] << 8) + (data[offset + 6] << 16) + (data[offset + 7] << 24)) | 0;
      const x0 = ((a + ((b & c) | (~b & d)) + w0 + 0x428a2f98) << 7 | (a + ((b & c) | (~b & d)) + w0 + 0x428a2f98) >>> 25) + b;
      const x1 = ((d + ((a & b) | (~a & c)) + w1 + 0x71374491) << 12 | (d + ((a & b) | (~a & c)) + w1 + 0x71374491) >>> 20) + a;
      a = x0;
      b = x1;
      checksum += (a + b) >>> 0;
    }
  }
  return checksum;
}));

results.push(measure('regex scan (ops/s)', 10_000, 80, () => {
  let checksum = 0;
  const pattern = /id="item-\d+" score="[\d.]+"/g;
  for (let repeat = 0; repeat < 80; repeat++) {
    const lines = [];
    for (let i = 0; i < 10_000; i++) {
      lines.push(`<item id="item-${i}" score="${(i * 1.25).toFixed(2)}">`);
    }
    const text = lines.join('\n');
    let match;
    while ((match = pattern.exec(text))) {
      checksum += match[0].length;
    }
    pattern.lastIndex = 0;
  }
  return checksum;
}));

results.push(measure('buffer copy (ops/s)', 1, 120, () => {
  let checksum = 0;
  const src = new Uint8Array(256 * 1024);
  for (let i = 0; i < src.length; i++) src[i] = i & 0xff;
  for (let repeat = 0; repeat < 120; repeat++) {
    const dst = new Uint8Array(src.length);
    dst.set(src);
    checksum += dst[0] + dst[dst.length - 1];
  }
  return checksum;
}));


console.log(JSON.stringify({ results }));
