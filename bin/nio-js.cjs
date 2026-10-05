#!/usr/bin/env node
'use strict';
const { spawn } = require('node:child_process');
const { readFileSync, existsSync } = require('node:fs');
const { createHash } = require('node:crypto');
const { join, dirname, resolve } = require('node:path');
const { constants } = require('node:os');
const root = resolve(__dirname, '..');

function executable({ platform = process.platform, arch = process.arch, override = process.env.NIO_JS_BIN, resolvePackage = require.resolve, directory = root } = {}) {
  if (override) return resolve(override);
  const manifest = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
  const name = `@nio-labs/nio-js-${platform}-${arch}`;
  if (!manifest.optionalDependencies[name]) throw new Error(`Unsupported platform ${platform}/${arch}. Build from source and set NIO_JS_BIN.`);
  const filename = platform === 'win32' ? 'nio-js.exe' : 'nio-js';
  let metadataPath;
  try { metadataPath = resolvePackage(`${name}/package.json`); }
  catch (error) {
    if (error.code !== 'MODULE_NOT_FOUND') throw error;
    // This fallback is available only in a source checkout, not the npm tarball.
    const local = join(directory, 'target', 'release', filename);
    if (existsSync(join(directory, 'Cargo.toml')) && existsSync(local)) return local;
    throw new Error(`Missing ${name}@${manifest.version}. Install with optional dependencies enabled, or build from source and set NIO_JS_BIN.`);
  }
  const metadata = JSON.parse(readFileSync(metadataPath, 'utf8'));
  if (metadata.name !== name || metadata.version !== manifest.version || metadata.os?.[0] !== platform || metadata.cpu?.[0] !== arch) {
    throw new Error('nio-js platform package does not match this launcher. Reinstall the package.');
  }
  const binary = join(dirname(metadataPath), filename);
  const digest = createHash('sha256').update(readFileSync(binary)).digest('hex');
  if (digest !== metadata.niojsSha256) throw new Error('nio-js executable checksum failed. Reinstall the package.');
  return binary;
}

function main() {
  let binary;
  try { binary = executable(); }
  catch (error) { console.error(`nio-js: ${error.message}`); process.exitCode = 1; return; }
  const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit', shell: false });
  const forward = signal => { if (child.exitCode === null && child.signalCode === null) child.kill(signal); };
  const interrupt = () => forward('SIGINT');
  const terminate = () => forward('SIGTERM');
  process.on('SIGINT', interrupt);
  process.on('SIGTERM', terminate);
  const cleanup = () => { process.removeListener('SIGINT', interrupt); process.removeListener('SIGTERM', terminate); };
  child.on('error', error => { cleanup(); console.error(`nio-js: ${error.message}`); process.exitCode = 1; });
  child.on('exit', (code, signal) => {
    cleanup();
    if (signal) {
      try { process.kill(process.pid, signal); }
      catch { process.exitCode = 128 + (constants.signals[signal] || 1); }
    } else process.exitCode = code ?? 1;
  });
}
module.exports = { executable };
if (require.main === module) main();
