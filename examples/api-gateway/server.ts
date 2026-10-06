import { get, post, reply } from 'nio.js';

// Fast in-memory datastore for active sessions and metrics
const state = {
  bootTime: Date.now(),
  requestCount: 0,
  users: [
    { id: 'usr_101', name: 'Alice Chen', role: 'admin', tier: 'enterprise' },
    { id: 'usr_102', name: 'Bob Smith', role: 'developer', tier: 'pro' },
    { id: 'usr_103', name: 'Carol Danvers', role: 'viewer', tier: 'free' }
  ]
};

// Health & liveness probe (sub-millisecond bare-metal response)
get('/health', () => {
  return {
    status: 'ok',
    uptime: Math.floor((Date.now() - state.bootTime) / 1000),
    workers: 'pinned-multi-core'
  };
});

// Dynamic route parameters with validation
get('/api/v1/users/:id', ({ params }) => {
  const user = state.users.find(u => u.id === params.id);
  if (!user) {
    return reply({ error: 'User not found', code: 'USER_NOT_FOUND' }, { status: 404 });
  }
  return { user };
});

// Query string parsing and filtering
get('/api/v1/users', ({ query }) => {
  state.requestCount++;
  let results = state.users;
  if (query.role) {
    results = results.filter(u => u.role === query.role);
  }
  if (query.tier) {
    results = results.filter(u => u.tier === query.tier);
  }
  return {
    total: results.length,
    users: results
  };
});

// JSON request validation, payload processing, and custom status
post('/api/v1/users', async ({ json }) => {
  const body = await json();
  if (!body.name || !body.role) {
    return reply(
      { error: 'Validation failed: "name" and "role" are required fields.' },
      { status: 400 }
    );
  }

  const newUser = {
    id: `usr_${Date.now().toString().slice(-4)}`,
    name: body.name,
    role: body.role,
    tier: body.tier || 'free'
  };

  state.users.push(newUser);
  state.requestCount++;

  return reply({ success: true, user: newUser }, { status: 201 });
});

// High-throughput CPU offloaded hashing / checksum benchmark endpoint
/** @native */
function computeChecksum(iterations: number): number {
  let acc = 0;
  for (let i = 0; i < iterations; i++) {
    acc = (acc * 31 + i) & 0x7fffffff;
  }
  return acc;
}

get('/api/v1/metrics/checksum', ({ query }) => {
  const rounds = Number(query.rounds || 50000);
  const start = Date.now();
  const checksum = computeChecksum(rounds);
  const elapsedMs = Date.now() - start;

  return {
    algorithm: 'hybrid-native-acc',
    rounds,
    checksum,
    elapsedMs,
    requestsProcessed: state.requestCount
  };
});
