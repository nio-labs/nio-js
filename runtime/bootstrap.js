(() => {
  const routes = [];
  const pending = new Map();
  let nextOperation = 0;
  let closed = false;
  const textBytes = s => new Uint8Array(__nioEncode(String(s)));
  const toBytes = value => {
    if (value instanceof Blob) return value._bytes.slice();
    if (value instanceof ArrayBuffer) return new Uint8Array(value).slice();
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength).slice();
    return textBytes(value);
  };
  class Blob {
    constructor(parts = [], options = {}) {
      const chunks = parts.map(toBytes);
      const length = chunks.reduce((n, c) => n + c.length, 0);
      if (length > __nioMaxBody) throw new RangeError('Blob exceeds byte limit');
      this._bytes = new Uint8Array(length);
      let offset = 0;
      for (const chunk of chunks) { this._bytes.set(chunk, offset); offset += chunk.length; }
      this.type = String(options.type ?? '').toLowerCase().replace(/[^\x20-\x7e]/g, '');
    }
    get size() { return this._bytes.length; }
    async text() { return __nioDecode(Array.from(this._bytes)); }
    async arrayBuffer() { return this._bytes.slice().buffer; }
    slice(start = 0, end = this.size, type = '') {
      start = start < 0 ? Math.max(this.size + start, 0) : Math.min(start, this.size);
      end = end < 0 ? Math.max(this.size + end, 0) : Math.min(end, this.size);
      return new Blob([this._bytes.slice(start, Math.max(start, end))], { type });
    }
    stream() {
      const bytes = this._bytes;
      let offset = 0, locked = false;
      return {
        getReader() {
          if (locked) throw new TypeError('Stream is locked');
          locked = true;
          return {
            async read() {
              if (offset >= bytes.length) return { done: true, value: undefined };
              const value = bytes.slice(offset, offset + 65536); offset += value.length;
              return { done: false, value };
            },
            async cancel() { offset = bytes.length; },
            releaseLock() { locked = false; },
          };
        },
        async *[Symbol.asyncIterator]() { const reader = this.getReader(); try { for (;;) { const r = await reader.read(); if (r.done) return; yield r.value; } } finally { reader.releaseLock(); } },
      };
    }
  }
  class File extends Blob {
    constructor(parts, name, options = {}) {
      super(parts, options);
      this.name = String(name).replaceAll('/', ':');
      this.lastModified = Number(options.lastModified ?? Date.now());
    }
  }
  class FormData {
    constructor() { this._entries = []; }
    append(name, value, filename) {
      if (value instanceof Blob) value = new File([value], filename ?? (value instanceof File ? value.name : 'blob'), { type: value.type });
      else value = String(value);
      this._entries.push([String(name), value]);
    }
    get(name) { return this._entries.find(([n]) => n === String(name))?.[1] ?? null; }
    getAll(name) { return this._entries.filter(([n]) => n === String(name)).map(([,v]) => v); }
    has(name) { return this._entries.some(([n]) => n === String(name)); }
    entries() { return this._entries[Symbol.iterator](); }
    [Symbol.iterator]() { return this.entries(); }
  }
  class Headers {
    constructor(init = {}) { this._values = new Map(); for (const [k,v] of (init instanceof Headers ? init.entries() : Array.isArray(init) ? init : Object.entries(init))) this.set(k,v); }
    set(name,value) { name = String(name).toLowerCase(); value = String(value); if (!/^[!#$%&'*+.^_`|~0-9a-z-]+$/.test(name) || /[\r\n]/.test(value)) throw new TypeError('Invalid header'); this._values.set(name,value); }
    get(name) { return this._values.get(String(name).toLowerCase()) ?? null; }
    has(name) { return this._values.has(String(name).toLowerCase()); }
    entries() { return this._values.entries(); }
    [Symbol.iterator]() { return this.entries(); }
  }
  class URLSearchParams {
    constructor(init = '') { this._pairs = Array.isArray(init) ? init : typeof init === 'object' ? Object.entries(init) : __nioQuery(String(init)); }
    get(name) { return this._pairs.find(([k]) => k === String(name))?.[1] ?? null; }
    getAll(name) { return this._pairs.filter(([k]) => k === String(name)).map(([,v]) => v); }
    has(name) { return this._pairs.some(([k]) => k === String(name)); }
    entries() { return this._pairs[Symbol.iterator](); }
    [Symbol.iterator]() { return this.entries(); }
    toString() { return __nioQueryEncode(JSON.stringify(this._pairs)); }
  }
  class URL {
    constructor(value, base) { Object.assign(this, JSON.parse(__nioUrl(String(value), base === undefined ? '' : String(base)))); this.searchParams = new URLSearchParams(this.search); }
    toString() { return this.href; }
    toJSON() { return this.href; }
  }
  class Response {
    constructor(body = '', options = {}) {
      this._blob = body instanceof Blob ? body : new Blob([body ?? '']);
      this.status = options.status ?? 200; this.ok = this.status >= 200 && this.status < 300;
      this.headers = new Headers(options.headers ?? {}); this.bodyUsed = false; this.body = this._blob.stream(); this.url = '';
      if (!this.headers.has('content-type') && this._blob.type) this.headers.set('content-type', this._blob.type);
    }
    _consume() { if (this.bodyUsed) throw new TypeError('Body already consumed'); this.bodyUsed = true; return this._blob; }
    async text() { return this._consume().text(); }
    async json() { return JSON.parse(await this.text()); }
    async arrayBuffer() { return this._consume().arrayBuffer(); }
    async blob() { return this._consume(); }
    static json(data, options = {}) { return new Response(JSON.stringify(data), { ...options, headers: { 'content-type': 'application/json; charset=utf-8', ...(options.headers ?? {}) } }); }
  }
  class TextEncoder { encode(value = '') { return textBytes(value); } }
  class TextDecoder { constructor(label = 'utf-8') { if (!['utf-8','utf8'].includes(label.toLowerCase())) throw new TypeError('Only UTF-8 supported'); } decode(value = new Uint8Array()) { return __nioDecode(Array.from(toBytes(value))); } }
  Object.assign(globalThis, { Blob, File, FormData, Headers, URL, URLSearchParams, Response, TextEncoder, TextDecoder });
  const operation = (kind, payload) => new Promise((resolve, reject) => {
    if (pending.size >= 8) return reject(new Error('Too many pending host operations'));
    const id = ++nextOperation; pending.set(id,{resolve,reject});
    try { __nioOperation(id,kind,JSON.stringify(payload)); } catch (e) { pending.delete(id); reject(e); }
  });
  globalThis.__nioComplete = (id, result) => {
    const p = pending.get(id); if (!p) return; pending.delete(id);
    const data = JSON.parse(result); data.error ? p.reject(new Error(data.error)) : p.resolve(data.value);
  };
  globalThis.fetch = async (address, options = {}) => {
    if (options.method && options.method.toUpperCase() !== 'GET') throw new Error('Only outbound GET is supported in this preview');
    if (options.body || options.headers || options.signal) throw new Error('Custom fetch bodies, headers, and signals are not supported in this preview');
    const result = await operation('fetch', { url: String(address) });
    const response = new Response(new Blob([new Uint8Array(__nioUnbase64(result.body))]), { status: result.status, headers: result.headers }); response.url = result.url;
    return response;
  };
  globalThis.setTimeout = (fn, ms = 0, ...args) => {
    if (typeof fn !== 'function') throw new TypeError('Timer callback must be a function');
    const promise = operation('sleep', { ms: Math.max(0, Math.min(Number(ms) || 0, 30000)) });
    promise.then(() => fn(...args)); return promise;
  };
  globalThis.console = Object.fromEntries(['log','info','warn','error','debug'].map(level => [level, (...args) => __nioLog(args.map(v => typeof v === 'string' ? v : JSON.stringify(v)).join(' '))]));
  globalThis.__nioRegister = (method, path, handler) => {
    if (closed) throw new Error('Route registration is closed');
    if (typeof path !== 'string' || !path.startsWith('/') || /[?#]/.test(path)) throw new TypeError('Invalid route path');
    if (typeof handler !== 'function' && typeof handler !== 'string' && !(handler instanceof Blob) && handler?.__nioReply !== true) throw new TypeError('Handler must be a function or supported constant response');
    if (routes.some(r => r.method === method && r.path === path)) throw new Error('Duplicate route');
    routes.push({ method, path, handler });
  };
  const encodeReply = result => {
    let options = {};
    if (result?.__nioReply === true) { options = result.options; result = result.body; }
    let body, type;
    if (result instanceof Response) {
      if (result.bodyUsed) throw new TypeError('Response body already consumed');
      options = { status: result.status, headers: Object.fromEntries(result.headers) }; result = result._blob;
    }
    if (typeof result === 'string') {
      body = result;
      if (options.headers === undefined && (options.status === undefined || options.status === 200)) {
        if (body.length > __nioMaxBody) throw new RangeError('Response exceeds byte limit');
        return [200, 1, body];
      }
      type = 'text/plain; charset=utf-8';
    } else if (result instanceof Blob) {
      body = result._bytes; type = result.type || 'application/octet-stream';
    } else if (result !== null && typeof result === 'object' && (Array.isArray(result) || Object.getPrototypeOf(result) === Object.prototype || Object.getPrototypeOf(result) === null)) {
      body = JSON.stringify(result);
      if (options.headers === undefined && (options.status === undefined || options.status === 200)) {
        if (body.length > __nioMaxBody) throw new RangeError('Response exceeds byte limit');
        return [200, 2, body];
      }
      type = 'application/json; charset=utf-8';
    } else throw new TypeError('Unsupported handler return value');
    if (body.length > __nioMaxBody) throw new RangeError('Response exceeds byte limit');
    let headers;
    if (options.headers === undefined) headers = [['content-type', type]];
    else {
      headers = new Headers(options.headers);
      if (!headers.has('content-type')) headers.set('content-type', type);
      headers = Array.from(headers);
    }
    const status = options.status ?? 200;
    if (!Number.isInteger(status) || status < 200 || status > 599) throw new TypeError('Invalid response status');
    return [status, 0, body, headers];
  };
  globalThis.__nioRoutes = () => {
    closed = true;
    return routes.map(r => ({
      method: r.method,
      path: r.path,
      constant: typeof r.handler !== 'function' ? encodeReply(r.handler) : null,
      arity: typeof r.handler === 'function' ? (r.handler.length || 0) : 0,
    }));
  };
  class LazyRequest {
    constructor(input) {
      this._input = input;
      this._parsed = null;
      this._headers = null;
      this._searchParams = null;
      this._query = null;
      this._consumed = false;
      this._boundText = null;
      this._boundJson = null;
      this._boundBlob = null;
      this._boundFormData = null;
    }
    _getParsed() {
      if (!this._parsed) this._parsed = typeof this._input === 'string' ? JSON.parse(this._input) : this._input;
      return this._parsed;
    }
    get method() { return this._getParsed().method; }
    get url() { return this._getParsed().url; }
    get search() { return this._getParsed().search || ''; }
    get params() { return this._getParsed().params || {}; }
    get requestId() { return this._getParsed().requestId; }
    get headers() {
      if (!this._headers) this._headers = new Headers(this._getParsed().headers || []);
      return this._headers;
    }
    set headers(v) { this._headers = v; }
    get searchParams() {
      if (!this._searchParams) this._searchParams = new URLSearchParams(this.search);
      return this._searchParams;
    }
    set searchParams(v) { this._searchParams = v; }
    get query() {
      if (!this._query) {
        this._query = {};
        for (const [k, v] of this.searchParams) {
          if (!(k in this._query)) this._query[k] = v;
        }
      }
      return this._query;
    }
    _getBytes() {
      if (this._consumed) throw new TypeError('Body already consumed');
      this._consumed = true;
      const b = this._getParsed().body;
      return b ? new Uint8Array(__nioUnbase64(b)) : new Uint8Array(0);
    }
    get text() {
      return this._boundText || (this._boundText = async () => __nioDecode(Array.from(this._getBytes())));
    }
    get json() {
      return this._boundJson || (this._boundJson = async () => JSON.parse(await this.text()));
    }
    get blob() {
      return this._boundBlob || (this._boundBlob = async () => new Blob([this._getBytes()], { type: this.headers.get('content-type') ?? '' }));
    }
    get formData() {
      return this._boundFormData || (this._boundFormData = async () => {
        this._getBytes();
        const rawForm = this._getParsed().form;
        if (!rawForm) throw new TypeError('Expected multipart/form-data or application/x-www-form-urlencoded');
        const form = new FormData();
        for (const field of rawForm) {
          if (field.filename !== null) form.append(field.name, new File([new Uint8Array(__nioUnbase64(field.body))], field.filename, { type: field.type ?? '' }));
          else form.append(field.name, __nioDecode(__nioUnbase64(field.body)));
        }
        return form;
      });
    }
  }
  const emptyReq = new LazyRequest({ method: 'GET', url: '', headers: [], query: {}, search: '', params: {}, body: '', form: null });
  globalThis.__nioDispatch = (route, input) => {
    const handler = routes[route]?.handler;
    if (typeof handler !== 'function') throw new Error('Invalid callback route');
    const req = handler.length === 0 ? emptyReq : new LazyRequest(input);
    const result = handler(req);
    return result !== null && (typeof result === 'object' || typeof result === 'function') && typeof result.then === 'function'
      ? Promise.resolve(result).then(encodeReply) : encodeReply(result);
  };
})();
