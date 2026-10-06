export const hello = 'Hello World';
export function json() {
  return { message: hello, items: Array.from({ length: 20 }, (_, id) => ({ id, name: `item-${id}` })) };
}
/** @native */
export function cpu() {
  let value = 42;
  for (let i = 0; i < 100000; i++) value = (Math.imul(value, 1664525) + 1013904223) | 0;
  return String(value >>> 0);
}
