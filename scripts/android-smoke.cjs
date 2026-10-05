'use strict';
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { createServer } = require('node:net');
const { checkVersion } = require('./check-version.cjs');
const adb = (...args) => execFileSync('adb', args, { encoding: 'utf8', timeout: 30000 }).trim();
async function main() {
  const directory = fs.mkdtempSync(join(tmpdir(), 'nio-js-android-'));
  const remote = '/data/local/tmp/nio-js-smoke';
  let pid;
  let port;
  try {
    const socket = createServer();
    await new Promise((done, reject) => { socket.once('error', reject); socket.listen(0, '127.0.0.1', done); });
    port = socket.address().port;
    await new Promise(done => socket.close(done));
    adb('shell', 'mkdir', '-p', remote);
    adb('push', resolve(process.argv[2] || 'target/x86_64-linux-android/release/nio-js'), `${remote}/nio-js`);
    adb('shell', 'chmod', '700', `${remote}/nio-js`);
    assert.equal(adb('shell', `${remote}/nio-js`, '--version'), `nio-js ${checkVersion().version}`);
    fs.writeFileSync(join(directory, 'server.ts'), `import {get,post} from 'nio.js';
get('/', 'Hello World');
get('/query', ({query}) => ({name:query.name}));
get('/timer', async()=>{await new Promise(r=>setTimeout(r,5));return new File(['héllo'],'a.txt');});
post('/echo', async({json})=>await json());
`);
    adb('push', join(directory, 'server.ts'), `${remote}/server.ts`);
    adb('shell', `${remote}/nio-js`, 'build', `${remote}/server.ts`, '-o', `${remote}/server.njs`);
    adb('shell', `${remote}/nio-js`, 'verify', `${remote}/server.njs`);
    adb('forward', `tcp:${port}`, 'tcp:31415');
    for (const entry of ['server.ts', 'server.njs']) {
      if (entry.endsWith('.njs')) adb('shell', 'rm', `${remote}/server.ts`);
      pid = adb('shell', `nohup ${remote}/nio-js run ${remote}/${entry} --port 31415 > ${remote}/server.log 2>&1 < /dev/null & echo $!`);
      assert.match(pid, /^\d+$/);
      const url = `http://127.0.0.1:${port}`;
      let ready = false;
      for (let attempt = 0; attempt < 100; attempt++) {
        try {
          const response = await fetch(url, { signal: AbortSignal.timeout(1000) });
          assert.equal(response.status, 200);
          assert.equal(await response.text(), 'Hello World');
          ready = true;
          break;
        } catch { await new Promise(done => setTimeout(done, 100)); }
      }
      assert.ok(ready, 'Android HTTP server became ready');
      assert.deepEqual(await (await fetch(`${url}/query?name=Termux`)).json(), { name: 'Termux' });
      assert.equal(await (await fetch(`${url}/timer`)).text(), 'héllo');
      const response = await fetch(`${url}/echo`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{"answer":42}' });
      assert.equal(response.status, 200);
      assert.deepEqual(await response.json(), { answer: 42 });
      adb('shell', 'kill', '-TERM', pid);
      pid = undefined;
      await new Promise(done => setTimeout(done, 1000));
    }
    console.log('Android/Bionic smoke passed: version, TypeScript, HTTP, JSON, timers, files, and source-free capsules.');
  } catch (error) {
    try { console.error(adb('shell', 'cat', `${remote}/server.log`)); } catch {}
    throw error;
  } finally {
    if (pid && /^\d+$/.test(pid)) { try { adb('shell', 'kill', '-TERM', pid); } catch {} }
    if (port) { try { adb('forward', '--remove', `tcp:${port}`); } catch {} }
    fs.rmSync(directory, { recursive: true, force: true });
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
