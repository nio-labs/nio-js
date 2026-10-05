'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { join, resolve } = require('node:path');
const { tmpdir } = require('node:os');
const { createHash } = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');
const { once } = require('node:events');
const { checkVersion, targets } = require('./check-version.cjs');
const { packagePlatform } = require('./package-platform.cjs');
const { prepareRelease } = require('./prepare-release.cjs');
const { publish } = require('./publish-npm.cjs');
const { executable } = require('../bin/nio-js.cjs');
const root = resolve(__dirname, '..');
function temporary(t) {
  const directory = fs.mkdtempSync(join(tmpdir(), 'nio-js-release-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  return directory;
}
function updateJson(path, mutate) {
  const value = JSON.parse(fs.readFileSync(path, 'utf8'));
  mutate(value);
  fs.writeFileSync(path, JSON.stringify(value));
}
function artifacts(t) {
  const directory = temporary(t);
  const binary = join(directory, 'input');
  fs.writeFileSync(binary, 'native executable fixture');
  fs.mkdirSync(join(directory, 'npm'));
  const version = checkVersion().version;
  for (const [platform, arch] of [...targets, [null, null]]) {
    if (platform) packagePlatform(binary, platform, arch, directory);
    const name = platform ? `@nio-labs/nio-js-${platform}-${arch}` : '@nio-labs/nio-js';
    const filename = `${name.slice(1).replace('/', '-')}-${version}.tgz`;
    const bytes = Buffer.from(`npm package fixture: ${name}`);
    fs.writeFileSync(join(directory, 'npm', filename), bytes);
    fs.writeFileSync(join(directory, 'npm', platform ? `pack-${platform}-${arch}.json` : 'pack-launcher.json'), JSON.stringify([{
      name, version, filename, integrity: `sha512-${createHash('sha512').update(bytes).digest('base64')}`,
    }]));
  }
  return directory;
}
test('release versions, exact platform versions and tags must agree', t => {
  const directory = temporary(t);
  for (const file of ['Cargo.toml', 'Cargo.lock', 'package.json']) fs.copyFileSync(join(root, file), join(directory, file));
  const version = checkVersion(directory).version;
  assert.equal(checkVersion(directory, `v${version}`).npmTag, 'latest');
  assert.throws(() => checkVersion(directory, 'v99.0.0'), /Expected existing release tag/);
  const prerelease = `${version}-rc.1`;
  updateJson(join(directory, 'package.json'), pkg => {
    pkg.version = prerelease;
    for (const key of Object.keys(pkg.optionalDependencies)) pkg.optionalDependencies[key] = prerelease;
  });
  assert.throws(() => checkVersion(directory), /same valid release version/);
  const cargo = join(directory, 'Cargo.toml');
  fs.writeFileSync(cargo, fs.readFileSync(cargo, 'utf8').replace(`version = "${version}"`, `version = "${prerelease}"`));
  const lock = join(directory, 'Cargo.lock');
  fs.writeFileSync(lock, fs.readFileSync(lock, 'utf8').replace(/(\[\[package\]\]\s+name = "nio-js"\s+version = ")[^"]+(")/, (_, prefix, suffix) => prefix + prerelease + suffix));
  assert.equal(checkVersion(directory, `v${prerelease}`).npmTag, 'next');
  updateJson(join(directory, 'package.json'), pkg => { pkg.optionalDependencies['@nio-labs/nio-js-linux-x64'] = '99.0.0'; });
  assert.throws(() => checkVersion(directory), /platform packages/);
});
test('platform launcher checks identity, version and binary checksum', t => {
  const directory = artifacts(t);
  const metadataPath = join(directory, 'nio-js-darwin-arm64', 'package.json');
  const options = { directory: root, platform: 'darwin', arch: 'arm64', override: '', resolvePackage: () => metadataPath };
  const binary = executable(options);
  assert.equal(fs.readFileSync(binary, 'utf8'), 'native executable fixture');
  fs.appendFileSync(binary, 'tampered');
  assert.throws(() => executable(options), /checksum failed/);
  updateJson(metadataPath, pkg => { pkg.version = '99.0.0'; });
  assert.throws(() => executable(options), /does not match/);
  assert.throws(() => executable({ ...options, arch: 'riscv64' }), /Unsupported platform/);
});
test('missing optional package gives actionable error; explicit binary override works', t => {
  const directory = temporary(t);
  fs.copyFileSync(join(root, 'package.json'), join(directory, 'package.json'));
  const options = { directory, platform: 'linux', arch: 'x64', override: '', resolvePackage: () => { throw Object.assign(new Error('missing'), { code: 'MODULE_NOT_FOUND' }); } };
  assert.throws(() => executable(options), /optional dependencies enabled/);
  assert.equal(executable({ ...options, override: './custom-binary' }), resolve('./custom-binary'));
});
test('Termux selects the Android ARM64 package', t => {
  const directory = artifacts(t);
  const metadata = join(directory, 'nio-js-android-arm64', 'package.json');
  const binary = executable({ directory: root, platform: 'android', arch: 'arm64', override: '', resolvePackage: name => {
    assert.equal(name, '@nio-labs/nio-js-android-arm64/package.json');
    return metadata;
  } });
  assert.equal(binary, join(directory, 'nio-js-android-arm64', 'nio-js'));
});
test('release preparation verifies all binaries; launcher is published last', t => {
  const directory = artifacts(t);
  const plan = prepareRelease(directory);
  assert.equal(plan.packages.length, 7);
  assert.equal(plan.packages.at(-1).name, '@nio-labs/nio-js');
  assert.equal(fs.readFileSync(join(directory, 'release', 'SHA256SUMS'), 'utf8').trim().split('\n').length, 6);
  fs.appendFileSync(join(directory, 'release', 'nio-js-linux-x64'), 'corrupted');
  assert.throws(() => prepareRelease(directory), /Binary checksum mismatch/);
});
test('release preparation rejects tampered tarballs and wrong package identities', t => {
  const directory = artifacts(t);
  const metadata = join(directory, 'npm', 'pack-darwin-arm64.json');
  const record = JSON.parse(fs.readFileSync(metadata))[0];
  fs.appendFileSync(join(directory, 'npm', record.filename), 'corrupted');
  assert.throws(() => prepareRelease(directory), /Tarball checksum mismatch/);
  updateJson(metadata, records => { records[0].name = '@someone/other'; });
  assert.throws(() => prepareRelease(directory), /Wrong package\/version/);
});
test('publish retries skip only identical packages and preserve publish order', t => {
  const plan = prepareRelease(artifacts(t));
  const published = [];
  let views = 0;
  publish(plan, args => {
    if (args[0] === 'view') {
      if (views++ === 0) return JSON.stringify(plan.packages[0].integrity);
      throw Object.assign(new Error('not found'), { stdout: JSON.stringify({ error: { code: 'E404' } }) });
    }
    published.push(args);
  });
  assert.equal(published.length, 6);
  assert.equal(published.at(-1)[1], plan.packages.at(-1).tarball);
  assert.ok(published.every(args => args.includes('--provenance') && args.includes('--ignore-scripts')));
  assert.throws(() => publish(plan, () => JSON.stringify('sha512-other')), /different contents/);
});
test('registry and authentication failures stop publishing', t => {
  const plan = prepareRelease(artifacts(t));
  let calls = 0;
  assert.throws(() => publish(plan, () => { calls++; throw new Error('registry unavailable'); }), /registry unavailable/);
  assert.equal(calls, 1);
  calls = 0;
  assert.throws(() => publish(plan, args => {
    calls++;
    if (args[0] === 'view') throw Object.assign(new Error('not found'), { stdout: JSON.stringify({ error: { code: 'E404' } }) });
    throw new Error('authentication failed');
  }), /authentication failed/);
  assert.equal(calls, 2);
});
test('launcher preserves argument bytes, working directory and exit code', t => {
  const directory = temporary(t);
  const script = join(directory, 'child.cjs');
  fs.writeFileSync(script, 'console.log(JSON.stringify({args:process.argv.slice(2),cwd:process.cwd()}));process.exit(17);');
  const result = spawnSync(process.execPath, [join(root, 'bin', 'nio-js.cjs'), script, 'literal ; $(echo bad)', 'space here'], {
    cwd: directory, env: { ...process.env, NIO_JS_BIN: process.execPath }, encoding: 'utf8',
  });
  assert.equal(result.status, 17, result.stderr);
  const output = JSON.parse(result.stdout);
  assert.deepEqual(output.args, ['literal ; $(echo bad)', 'space here']);
  assert.equal(output.cwd, fs.realpathSync(directory));
});
test('launcher forwards SIGTERM', { skip: process.platform === 'win32', timeout: 10000 }, async t => {
  const directory = temporary(t);
  const script = join(directory, 'child.cjs');
  fs.writeFileSync(script, "process.on('SIGTERM',()=>process.exit(23));setInterval(()=>{},1000);console.log('ready');");
  const child = spawn(process.execPath, [join(root, 'bin', 'nio-js.cjs'), script], { env: { ...process.env, NIO_JS_BIN: process.execPath } });
  t.after(() => { if (child.exitCode === null) child.kill('SIGKILL'); });
  await once(child.stdout, 'data');
  const exited = once(child, 'exit');
  child.kill('SIGTERM');
  assert.equal((await exited)[0], 23);
});
