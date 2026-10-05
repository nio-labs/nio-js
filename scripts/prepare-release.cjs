'use strict';
const { readFileSync, writeFileSync } = require('node:fs');
const { createHash } = require('node:crypto');
const { resolve, join, basename } = require('node:path');
const { checkVersion, targets } = require('./check-version.cjs');

function prepareRelease(directory) {
  directory = resolve(directory);
  const { version, npmTag } = checkVersion();
  const packages = [];
  const sums = [];
  const entries = [...targets.map(([os, arch]) => ({ name: `@nio-labs/nio-js-${os}-${arch}`, metadata: `pack-${os}-${arch}.json`, os, arch })),
    { name: '@nio-labs/nio-js', metadata: 'pack-launcher.json' }];
  for (const entry of entries) {
    if (entry.os) {
      const asset = `nio-js-${entry.os}-${entry.arch}${entry.os === 'win32' ? '.exe' : ''}`;
      const digest = createHash('sha256').update(readFileSync(join(directory, 'release', asset))).digest('hex');
      const checksum = `${digest}  ${asset}\n`;
      if (readFileSync(join(directory, 'release', `${asset}.sha256`), 'utf8') !== checksum) throw new Error(`Binary checksum mismatch: ${asset}`);
      sums.push(checksum);
    }
    const records = JSON.parse(readFileSync(join(directory, 'npm', entry.metadata), 'utf8'));
    if (!Array.isArray(records) || records.length !== 1) throw new Error(`Expected one npm package: ${entry.metadata}`);
    const record = records[0];
    if (record.name !== entry.name || record.version !== version || typeof record.filename !== 'string' || basename(record.filename) !== record.filename || !record.filename.endsWith('.tgz')) throw new Error(`Wrong package/version: ${entry.metadata}`);
    const tarball = join(directory, 'npm', record.filename);
    const integrity = `sha512-${createHash('sha512').update(readFileSync(tarball)).digest('base64')}`;
    if (record.integrity !== integrity) throw new Error(`Tarball checksum mismatch: ${record.filename}`);
    packages.push({ name: record.name, version, tarball, integrity });
  }
  writeFileSync(join(directory, 'release', 'SHA256SUMS'), sums.sort().join(''));
  return { version, npmTag, packages };
}
module.exports = { prepareRelease };
if (require.main === module) {
  try {
    const plan = prepareRelease(process.argv[2] || 'dist/artifacts');
    writeFileSync(process.argv[3] || 'dist/publish-plan.json', JSON.stringify(plan, null, 2) + '\n');
    console.log(`Verified ${plan.packages.length} npm tarballs and ${targets.length} native binaries.`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
