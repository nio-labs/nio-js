'use strict';
const { execFileSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
const { createHash } = require('node:crypto');
function npm(args) {
  return execFileSync('npm', args, { encoding: 'utf8', stdio: args[0] === 'publish' ? 'inherit' : ['ignore', 'pipe', 'pipe'] });
}
function publish(plan, run = npm) {
  for (const pkg of plan.packages) {
    const digest = `sha512-${createHash('sha512').update(readFileSync(pkg.tarball)).digest('base64')}`;
    if (digest !== pkg.integrity) throw new Error(`Tarball changed before publishing: ${pkg.name}`);
    let existing;
    try { existing = JSON.parse(run(['view', `${pkg.name}@${pkg.version}`, 'dist.integrity', '--json'])); }
    catch (error) {
      let code;
      try { code = JSON.parse(String(error.stdout)).error?.code; } catch {}
      if (code !== 'E404') throw error;
    }
    if (existing !== undefined) {
      if (existing !== pkg.integrity) throw new Error(`${pkg.name}@${pkg.version} already exists with different contents. Bump the version; published versions are immutable.`);
      console.log(`Already published identical ${pkg.name}@${pkg.version}`);
      continue;
    }
    run(['publish', pkg.tarball, '--access', 'public', '--provenance', '--tag', plan.npmTag, '--ignore-scripts']);
  }
}
module.exports = { publish };
if (require.main === module) {
  try { publish(JSON.parse(readFileSync(process.argv[2] || 'dist/publish-plan.json', 'utf8'))); }
  catch (error) { console.error(error.stderr?.toString() || error.message); process.exitCode = 1; }
}
