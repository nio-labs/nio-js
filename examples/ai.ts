import { get } from 'nio.js';
import { predict, calculate } from './model.py';

get('/ai/classify', (req) => {
  const prompt = req.query.prompt || 'nio-js is very fast and great';
  return predict(prompt);
});

get('/ai/compute', (req) => {
  const x = Number(req.query.x || 10);
  const y = Number(req.query.y || 20);
  return { score: calculate(x, y) };
});
