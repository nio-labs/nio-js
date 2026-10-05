import { get } from 'nio.js';
import { hello, json, cpu } from './workload.js';
get('/constant', hello);
get('/callback', () => hello);
get('/json', json);
get('/cpu', cpu);
