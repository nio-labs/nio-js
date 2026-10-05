import { hello, json, cpu } from './workload.js';
const cached = new TextEncoder().encode(hello);
let id = 0;
Deno.serve({
  hostname: '127.0.0.1',
  port: Number(Deno.env.get('PORT')),
  onListen() {},
  handler(req) {
    const path = new URL(req.url).pathname;
    const body = path === '/constant' ? cached : path === '/callback' ? hello : path === '/json' ? JSON.stringify(json()) : cpu();
    return new Response(body, {
      headers: {
        'content-type': path === '/json' ? 'application/json' : 'text/plain; charset=utf-8',
        'x-request-id': String(++id),
      },
    });
  },
});
