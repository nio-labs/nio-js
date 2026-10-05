'use strict';
const { copyFileSync, mkdirSync, readFileSync, writeFileSync, chmodSync, statSync } = require('node:fs');
const { createHash } = require('node:crypto');
const { resolve, join } = require('node:path');
const { checkVersion, targets } = require('./check-version.cjs');
const root = resolve(__dirname, '..');
function packagePlatform(binary, platform, arch, output = join(root, 'dist')) {
  if (!targets.some(([os, cpu]) => os === platform && cpu === arch)) throw new Error(`Unsupported platform ${platform}/${arch}`);
  const { version } = checkVersion();
  binary = resolve(binary);
  output = resolve(output);
  if (!statSync(binary).isFile() || statSync(binary).size === 0) throw new Error('Expected a nonempty compiled nio-js executable.');
  const filename = platform === 'win32' ? 'nio-js.exe' : 'nio-js';
  const asset = `nio-js-${platform}-${arch}${platform === 'win32' ? '.exe' : ''}`;
  const directory = join(output, `nio-js-${platform}-${arch}`);
  const release = join(output, 'release');
  mkdirSync(directory, { recursive: true });
  mkdirSync(release, { recursive: true });
  const digest = createHash('sha256').update(readFileSync(binary)).digest('hex');
  copyFileSync(binary, join(directory, filename));
  chmodSync(join(directory, filename), 0o755);
  copyFileSync(binary, join(release, asset));
  chmodSync(join(release, asset), 0o755);
  writeFileSync(join(release, `${asset}.sha256`), `${digest}  ${asset}\n`);
  copyFileSync(join(root, 'LICENSE'), join(directory, 'LICENSE'));
  writeFileSync(join(directory, 'README.md'), `# @nio-labs/nio-js-${platform}-${arch}\n\nNative executable for nio-js ${version}. Install \`@nio-labs/nio-js\` to use the CLI.\n`);
  writeFileSync(join(directory, 'package.json'), JSON.stringify({
    name: `@nio-labs/nio-js-${platform}-${arch}`, version,
    description: `Native nio-js executable for ${platform}/${arch}`, license: 'MIT',
    repository: { type: 'git', url: 'git+https://github.com/nio-labs/nio-js.git' },
    os: [platform], cpu: [arch], files: [filename, 'LICENSE', 'README.md'],
    niojsSha256: digest, publishConfig: { access: 'public' },
  }, null, 2) + '\n');
  return directory;
}
module.exports = { packagePlatform };
if (require.main === module) {
  try {
    const [binary = join(root, 'target', 'release', process.platform === 'win32' ? 'nio-js.exe' : 'nio-js'), platform = process.platform, arch = process.arch, output = join(root, 'dist')] = process.argv.slice(2);
    console.log(packagePlatform(binary, platform, arch, output));
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
