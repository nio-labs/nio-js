'use strict';
const { readFileSync, appendFileSync } = require('node:fs');
const { resolve, join } = require('node:path');
const root = resolve(__dirname, '..');
const targets = [['android', 'arm64'], ['darwin', 'arm64'], ['darwin', 'x64'], ['linux', 'arm64'], ['linux', 'x64'], ['win32', 'x64']];
function checkVersion(directory = root, tag) {
  const pkg = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
  const block = readFileSync(join(directory, 'Cargo.toml'), 'utf8').split('[package]')[1]?.split(/^\[/m)[0];
  const cargo = block?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  const lock = readFileSync(join(directory, 'Cargo.lock'), 'utf8').match(/\[\[package\]\]\s+name = "nio-js"\s+version = "([^"]+)"/)?.[1];
  const version = pkg.version;
  const valid = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.test(version)
    && !version.split('-').slice(1).join('-').split('.').some(part => /^0\d+$/.test(part));
  if (!valid || pkg.name !== '@nio-labs/nio-js' || cargo !== version || lock !== version) throw new Error('package.json, Cargo.toml, and Cargo.lock must have the same valid release version.');
  const names = targets.map(([os, arch]) => `@nio-labs/nio-js-${os}-${arch}`);
  if (Object.keys(pkg.optionalDependencies || {}).length !== names.length || names.some(name => pkg.optionalDependencies[name] !== version)) {
    throw new Error('All supported platform packages must use the exact release version.');
  }
  if (tag !== undefined && tag !== `v${version}`) throw new Error(`Expected existing release tag v${version}; received ${tag}.`);
  return { version, npmTag: version.includes('-') ? 'next' : 'latest', prerelease: version.includes('-') };
}
module.exports = { checkVersion, targets };
if (require.main === module) {
  try {
    const result = checkVersion(root, process.argv[2]);
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `version=${result.version}\nnpm_tag=${result.npmTag}\nprerelease=${result.prerelease}\n`);
    console.log(`Validated nio-js ${result.version}`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
