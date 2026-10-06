export const get = (path, handler) => globalThis.__nioRegister('GET', path, handler);
export const post = (path, handler) => globalThis.__nioRegister('POST', path, handler);
export const reply = (body, options = {}) => ({ __nioReply: true, body, options });
export const asset = name => {
  const item = JSON.parse(globalThis.__nioAsset(String(name)));
  return new Blob([new Uint8Array(globalThis.__nioUnbase64(item.body))], { type: item.media_type });
};
export const redirect = (location, status = 302) => reply('', { status, headers: { location } });
export const native = name => (...args) => globalThis.__nioNative(name, ...args);
export const python = (module, func) => (...args) => {
  const payload = JSON.stringify(args);
  const res = globalThis.__nioPython(String(module), String(func), payload);
  try { return JSON.parse(res); } catch { return res; }
};
export const pythonEval = code => globalThis.__nioPythonEval(String(code));
