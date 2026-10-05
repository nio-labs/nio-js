export const get = (path, handler) => globalThis.__nioRegister('GET', path, handler);
export const post = (path, handler) => globalThis.__nioRegister('POST', path, handler);
export const reply = (body, options = {}) => ({ __nioReply: true, body, options });
export const asset = name => {
  const item = JSON.parse(globalThis.__nioAsset(String(name)));
  return new Blob([new Uint8Array(globalThis.__nioUnbase64(item.body))], { type: item.media_type });
};
export const redirect = (location, status = 302) => reply('', { status, headers: { location } });
