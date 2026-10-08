# NioPM Design Document

> **Status:** Draft  
> **Runtime:** Node.js >= 18 (ESM-only)  
> **Default mode:** CDN-only (zero disk writes for packages)  
> **Opt-in mode:** `--cache` for local offline cache at `~/.niopm/cache`

---

## 1. Problem

`npm install` is slow, creates bloated `node_modules`, and is unnecessary when CDNs can serve packages as ESM instantly.

## 2. Solution

A Node.js package runner that:
- Skips `node_modules` entirely
- Resolves bare imports (`import _ from 'lodash'`) to CDN URLs at runtime via a custom ESM Loader
- Pins exact versions in a lockfile for determinism
- Optionally caches fetched modules locally for offline use

## 3. File Structure

```
niopm/
├── package.json
├── README.md
├── src/
│   ├── cli.js          # CLI entrypoint (init, add, remove, run, lock, clean, doctor)
│   ├── loader.js       # Custom ESM loader hook (resolve/load)
│   ├── resolver.js     # CDN URL builder + fallback logic
│   ├── lockfile.js     # niopm.lock read/write/validate
│   └── cache.js        # Optional ~/.niopm/cache manager (disabled by default)
└── bin/
    └── niopm.js        # Shell wrapper
```

## 4. CLI Commands

| Command | Purpose |
|---|---|
| `niopm init` | Create `package.json` + `niopm.lock` |
| `niopm add <pkg>` | Add dependency, update lockfile |
| `niopm remove <pkg>` | Remove dependency and lockfile entry |
| `niopm run <script> [args]` | Execute with loader injected |
| `niopm lock` | Regenerate lockfile from `package.json` |
| `niopm clean` | Purge `~/.niopm/cache` |
| `niopm doctor` | Diagnose network/cache/compat issues |

### `niopm run` Flags

| Flag | Purpose |
|---|---|
| `--cache` | Enable local cache (`~/.niopm/cache`) |
| `--offline` | Disallow network, use cache only |
| `--silent` | Suppress network logs |
| `--cdns esm.sh,unpkg` | Override CDN priority |

## 5. Lockfile Format (`niopm.lock`)

```json
{
  "version": "1",
  "dependencies": {
    "lodash": {
      "version": "4.17.21",
      "resolved": "https://esm.sh/lodash@4.17.21",
      "integrity": "sha384-..."
    }
  }
}
```

Fields:
- `version` — lockfile format version
- `dependencies` — map of `name@version` to resolution metadata
- `resolved` — exact CDN URL
- `integrity` — SHA-256 of fetched content (populated when `--cache` is used)

## 6. Runtime Loader (`src/loader.js`)

### Resolve Hook
```js
resolve(specifier, context, nextResolve) {
  // 1. If already a URL, pass through
  if (specifier.startsWith('http')) return nextResolve(specifier);
  
  // 2. If relative or absolute path, pass through
  if (specifier.startsWith('.') || specifier.startsWith('/')) return nextResolve(specifier);
  
  // 3. Otherwise treat as bare package specifier
  const url = resolver.resolve(specifier);
  return nextResolve(url);
}
```

### Load Hook
```js
async load(url, context, nextLoad) {
  if (url.startsWith('http')) {
    // Fetch from CDN (or cache if --cache enabled)
    const source = await fetcher.get(url);
    return { format: 'module', source };
  }
  return nextLoad(url);
}
```

## 7. Resolver (`src/resolver.js`)

```js
const CDNS = ['https://esm.sh', 'https://unpkg.com'];

resolve(specifier) {
  // Parse "lodash" or "lodash@4.17.21"
  const [name, version] = parseSpecifier(specifier);
  
  // Check lockfile first
  const locked = lockfile.get(name, version);
  if (locked) return locked.resolved;
  
  // Build URL from preferred CDN
  return `${CDNS[0]}/${name}@${version || 'latest'}`;
}
```

Fallback chain:
1. Lockfile pinned URL
2. esm.sh
3. unpkg
4. Error with clear message

## 8. Cache Layer (`src/cache.js`, opt-in)

Activated only when `--cache` flag is passed.

### Cache Layout

```
~/.niopm/
├── cache/
│   └── <name>@<version>/
│       ├── package.json
│       ├── index.js
│       └ integrity.txt
└── config.json
```

### Cache Rules

- Key: `name@version`
- TTL: 7 days (configurable)
- Integrity: SHA-256 stored in `integrity.txt`
- Offline mode: `--offline` errors if cache miss
- Clean: `niopm clean` removes `~/.niopm/cache`

## 9. Error Handling

| Scenario | Behavior |
|---|---|
| CDN unreachable | Print error, suggest `--cache` or `npm install` fallback |
| Package not found | Print "Package X not found on esm.sh/unpkg" |
| Non-ESM package | esm.sh transpiles; if it fails, error with CJS note |
| Native addon | Fail fast: "Package X requires native compilation. Use `npm install`." |
| Lockfile mismatch | Warn and prompt to run `niopm lock` |

## 10. Implementation Phases

### Phase 1 — Core (v0.1)
- [ ] `package.json` scaffold
- [ ] `src/resolver.js` (esm.sh + unpkg URL builder)
- [ ] `src/lockfile.js` (read/write JSON lockfile)
- [ ] `src/loader.js` (bare specifier → CDN rewrite)
- [ ] `src/cli.js` (init, add, run)
- [ ] `README.md` quickstart

### Phase 2 — Polish (v0.2)
- [ ] `src/cache.js` (`--cache` opt-in)
- [ ] `src/cli.js` (remove, lock, clean, doctor)
- [ ] Integrity hashing in lockfile
- [ ] Proxy support (`HTTP_PROXY` / `HTTPS_PROXY`)
- [ ] Type definition fetching for IDE support

### Phase 3 — Hardening (v0.3)
- [ ] `--offline` mode
- [ ] Sub-dependency lockfile resolution
- [ ] Retry + backoff for network requests
- [ ] Supply-chain warning for mismatched integrity

## 11. Non-Goals

- CommonJS transpilation (esm.sh handles most cases)
- Native module compilation (`node-gyp`)
- Monorepo/workspace hoisting
- Windows `.exe` wrapper

---

## 12. Open Questions

1. **CDN URL format:** esm.sh default bundle vs `?bundle` flag?
2. **Version pinning:** `add lodash` → latest, or prompt for version?
3. **Lockfile format:** JSON vs TOML? (JSON is zero-dep; TOML is prettier)
4. **Scope packages:** support `@scope/pkg`?
