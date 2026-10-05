'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { join, resolve } = require('node:path');
const { tmpdir } = require('node:os');
const { createHash } = require('node:crypto');
const { spawnSync } = require('node:child_process');
const installer = resolve(__dirname, '../install.sh');
function fixture(t, options = {}) {
  const dir = fs.mkdtempSync(join(tmpdir(), 'nio-js-install-test-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const tools = join(dir, 'tools');
  fs.mkdirSync(tools);
  const binary = '#!/bin/sh\necho "nio-js 0.1.0"\n';
  fs.writeFileSync(join(dir, 'binary'), binary);
  fs.writeFileSync(join(dir, 'checksum'), `${options.corrupt ? '0'.repeat(64) : createHash('sha256').update(binary).digest('hex')}  nio-js\n`);
  fs.writeFileSync(join(tools, 'uname'), '#!/bin/sh\ncase "$1" in -s) echo "$MOCK_OS";; -m) echo "$MOCK_ARCH";; -o) echo "$MOCK_KERNEL";; esac\n', { mode: 0o755 });
  fs.writeFileSync(join(tools, 'ldd'), '#!/bin/sh\necho "$MOCK_LIBC"\n', { mode: 0o755 });
  fs.writeFileSync(join(tools, 'curl'), '#!/bin/sh\nwhile [ "$#" -gt 0 ]; do\ncase "$1" in https:*) url=$1;; --output) shift; dest=$1;; esac\nshift\ndone\nprintf "%s\\n" "$url" >> "$FIXTURE/requests"\ncase "$url" in *.sha256) cp "$FIXTURE/checksum" "$dest";; *) cp "$FIXTURE/binary" "$dest";; esac\n', { mode: 0o755 });
  const home = join(dir, 'home');
  fs.mkdirSync(home);
  return {
    dir, home, binary,
    run(args = []) {
      return spawnSync('sh', [installer, ...args], {
        encoding: 'utf8',
        env: { ...process.env, PATH: `${tools}:${process.env.PATH}`, HOME: home,
          NIO_JS_VERSION: '', NIO_JS_INSTALL_DIR: '', ANDROID_ROOT: '', PREFIX: join(dir, 'termux'),
          FIXTURE: dir, MOCK_OS: options.os || 'Darwin', MOCK_ARCH: options.arch || 'arm64',
          MOCK_KERNEL: options.android ? 'Android' : 'GNU/Linux', MOCK_LIBC: options.libc || 'glibc 2.35' },
      });
    },
  };
}
const unix = { skip: process.platform === 'win32' };
test('installer selects macOS ARM64 latest and installs without Node', unix, t => {
  const f = fixture(t);
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(join(f.home, '.local/bin/nio-js'), 'utf8'), f.binary);
  assert.match(fs.readFileSync(join(f.dir, 'requests'), 'utf8'), /releases\/latest\/download\/nio-js-darwin-arm64/);
});
test('installer selects Linux x64, a pinned tag and a directory containing spaces', unix, t => {
  const f = fixture(t, { os: 'Linux', arch: 'x86_64' });
  const dest = join(f.dir, 'custom bin');
  const result = f.run(['--version', '0.2.0-rc.1', '--install-dir', dest]);
  assert.equal(result.status, 0, result.stderr);
  assert.ok(fs.existsSync(join(dest, 'nio-js')));
  assert.match(fs.readFileSync(join(f.dir, 'requests'), 'utf8'), /download\/v0.2.0-rc.1\/nio-js-linux-x64/);
});
test('installer selects Android ARM64 and Termux PREFIX', unix, t => {
  const f = fixture(t, { os: 'Linux', arch: 'aarch64', android: true });
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  assert.ok(fs.existsSync(join(f.dir, 'termux/bin/nio-js')));
  assert.match(fs.readFileSync(join(f.dir, 'requests'), 'utf8'), /nio-js-android-arm64/);
});
test('installer refuses corrupt downloads and preserves the existing executable', unix, t => {
  const f = fixture(t, { corrupt: true });
  const dest = join(f.dir, 'bin');
  fs.mkdirSync(dest);
  fs.writeFileSync(join(dest, 'nio-js'), 'previous runtime');
  const result = f.run(['--install-dir', dest]);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Checksum mismatch/);
  assert.equal(fs.readFileSync(join(dest, 'nio-js'), 'utf8'), 'previous runtime');
});
test('installer rejects unsupported architectures, musl and malformed versions before downloading', unix, t => {
  for (const options of [{ arch: 'armv7l' }, { os: 'Linux', libc: 'musl libc' }]) {
    const f = fixture(t, options);
    assert.notEqual(f.run().status, 0);
    assert.ok(!fs.existsSync(join(f.dir, 'requests')));
  }
  const f = fixture(t);
  assert.notEqual(f.run(['--version', '../anything']).status, 0);
  assert.ok(!fs.existsSync(join(f.dir, 'requests')));
});
