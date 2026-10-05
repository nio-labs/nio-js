import http from 'node:http';
import { hello, json, cpu } from './workload.js';
const cached = Buffer.from(hello);
let id = 0;
http.createServer((req, res) => {
  const path = req.url;
  const body = path === '/constant' ? cached : path === '/callback' ? hello : path === '/json' ? JSON.stringify(json()) : cpu();
  res.setHeader('content-type', path === '/json' ? 'application/json' : 'text/plain; charset=utf-8');
  res.setHeader('x-request-id', String(++id));
  res.end(body);
}).listen(Number(process.env.PORT), '127.0.0.1');
