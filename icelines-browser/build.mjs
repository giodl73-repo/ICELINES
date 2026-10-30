import { cp, mkdir, readFile, writeFile, realpath, lstat, rm } from 'node:fs/promises';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { buildOfflineShell } from './offline-build.mjs';
const root = fileURLToPath(new URL('.', import.meta.url));
const expectedDist = resolve(await realpath(root), 'dist');
try {
  const stat = await lstat(expectedDist);
  if (stat.isSymbolicLink() || await realpath(expectedDist) !== expectedDist) throw new Error('Refusing to clean a redirected dist directory');
  await rm(expectedDist, { recursive: true });
} catch (error) { if (error.code !== 'ENOENT') throw error; }
const result = spawnSync(process.execPath, ['node_modules/typescript/bin/tsc'], { cwd: root, stdio: 'inherit' });
if (result.status !== 0) process.exit(result.status ?? 1);
await mkdir(new URL('./dist/', import.meta.url), { recursive: true });
for (const name of ['index.html', 'style.css']) await cp(new URL(name, import.meta.url), new URL('dist/' + name, import.meta.url));
await cp(new URL('public/pkg/', import.meta.url), new URL('dist/pkg/', import.meta.url), { recursive: true });
const catalog = JSON.parse(await readFile(new URL('public/catalog.json', import.meta.url), 'utf8'));
await cp(new URL('public/catalog.json', import.meta.url), new URL('dist/catalog.json', import.meta.url));
await mkdir(new URL('dist/data/', import.meta.url), { recursive: true });
for (const entry of catalog.packages) {
  if (!/^data\/[a-f0-9]{64}\.json$/.test(entry.url)) throw new Error('Unsafe publication package path');
  await cp(new URL('public/' + entry.url, import.meta.url), new URL('dist/' + entry.url, import.meta.url));
}
try { await cp(new URL('public/build-info.json', import.meta.url), new URL('dist/build-info.json', import.meta.url)); }
catch (error) { if (error.code !== 'ENOENT') throw error; }
await writeFile(new URL('dist/.nojekyll', import.meta.url), '');
const manifest = JSON.parse(await readFile(new URL('package.json', import.meta.url), 'utf8'));
const shell = await buildOfflineShell(new URL('.', import.meta.url));
console.log(`IceLines browser ${manifest.version} built in dist/; offline shell ${shell.build.slice(0, 12)}, ${shell.assets.reduce((sum, asset) => sum + asset.gzip_bytes, 0)} bytes gzip (summed assets)`);
