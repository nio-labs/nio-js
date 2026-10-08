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

results.push(measure('object mutation (ops/s)', 1, 800, () => {
  let checksum = 0;
  const base = Array.from({ length: 256 }, (_, i) => ({ id: i, active: true, tags: new Array(8).fill(i) }));
  for (let repeat = 0; repeat < 800; repeat++) {
    const items = base.map((item) => ({ ...item, active: item.id % 3 === 0 }));
    for (let i = 0; i < items.length; i++) {
      if (items[i].active) checksum += items[i].tags.reduce((sum, tag) => sum + tag, 0);
    }
  }
  return checksum;
}));

results.push(measure('tree traversal (ops/s)', 1, 300, () => {
  let checksum = 0;
  function build(depth, width, prefix) {
    if (depth === 0) return { leaf: true, value: prefix };
    return { leaf: false, value: prefix, children: Array.from({ length: width }, (_, i) => build(depth - 1, width, prefix * 10 + i)) };
  }
  const root = build(3, 3, 1);
  function walk(node) {
    checksum += node.value;
    if (node.children) for (const child of node.children) walk(child);
  }
  for (let repeat = 0; repeat < 300; repeat++) {
    checksum = 0;
    walk(root);
  }
  return checksum;
}));

results.push(measure('stream chunks (ops/s)', 1, 150, () => {
  let checksum = 0;
  const chunks = Array.from({ length: 128 }, (_, i) => ({ offset: i * 4096, length: 4096, data: new Uint8Array(4096).fill(i & 0xff) }));
  for (let repeat = 0; repeat < 150; repeat++) {
    let collected = 0;
    for (const chunk of chunks) {
      collected += chunk.data[0] + chunk.data[chunk.data.length - 1];
    }
    checksum += collected;
  }
  return checksum;
}));

results.push(measure('text parsing (ops/s)', 1, 120, () => {
  let checksum = 0;
  const log = Array.from({ length: 5_000 }, (_, i) => `row-${i} latency=${(i * 0.37).toFixed(2)} status=${i % 7 === 0 ? 500 : 200}`).join('\n');
  for (let repeat = 0; repeat < 120; repeat++) {
    const lines = log.split('\n');
    for (const line of lines) {
      const number = Number(line.split('latency=')[1]?.split(' ')[0] ?? 0);
      checksum += Math.round(number);
    }
  }
  return checksum;
}));

results.push(measure('date parsing (ops/s)', 2_000, 80, () => {
  let checksum = 0;
  const timestamps = Array.from({ length: 2_000 }, (_, i) => new Date(Date.UTC(2024, 0, 1, i % 24, i % 60, i % 60)));
  for (let repeat = 0; repeat < 80; repeat++) {
    for (const ts of timestamps) {
      checksum += ts.getUTCFullYear() + ts.getUTCMonth() + ts.getUTCDate();
    }
  }
  return checksum;
}));

results.push(measure('JSON large parse (ops/s)', 1, 25, () => {
  let checksum = 0;
  const largePayload = JSON.stringify({
    id: 1,
    label: 'large-payload',
    values: Array.from({ length: 8_192 }, (_, i) => ({ id: i, score: i * 1.25 })),
  });
  for (let repeat = 0; repeat < 25; repeat++) {
    checksum += JSON.parse(largePayload).values[8191].id;
  }
  return checksum;
}));

results.push(measure('JSON medium stringify (ops/s)', 1, 800, () => {
  let checksum = 0;
  const mediumPayload = {
    id: 1,
    label: 'medium-payload',
    values: Array.from({ length: 512 }, (_, i) => ({ id: i, score: i * 1.25 })),
  };
  for (let repeat = 0; repeat < 800; repeat++) {
    checksum += JSON.stringify(mediumPayload).length;
  }
  return checksum;
}));

results.push(measure('JSON small stringify (ops/s)', 1, 5_000, () => {
  let checksum = 0;
  const smallPayload = { id: 1, label: 'small-payload', values: [1, 2, 3, 4, 5] };
  for (let repeat = 0; repeat < 5_000; repeat++) {
    checksum += JSON.stringify(smallPayload).length;
  }
  return checksum;
}));

results.push(measure('log format (ops/s)', 1, 4_000, () => {
  let checksum = 0;
  const template = `method=%s url=%s status=%d latency=%.2fms size=%d`;
  for (let repeat = 0; repeat < 4_000; repeat++) {
    checksum += template
      .replace('%s', 'GET')
      .replace('%s', '/benchmark')
      .replace('%d', '200')
      .replace('%.2f', '1.25')
      .replace('%d', '128')
      .length;
  }
  return checksum;
}));

results.push(measure('random ints (ops/s)', 5_000, 200, () => {
  let checksum = 0;
  for (let repeat = 0; repeat < 200; repeat++) {
    const nums = Array.from({ length: 5_000 }, () => Math.floor(Math.random() * 10_000));
    checksum += nums.reduce((sum, n) => sum + n, 0);
  }
  return checksum;
}));

console.log(JSON.stringify({ results }));

console.log(JSON.stringify({ results }));
