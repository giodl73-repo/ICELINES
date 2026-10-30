import { readFile, readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';

export async function buildOfflineShell(root) {
  const files = ['index.html', 'style.css', 'catalog.json'];
  try { await readFile(new URL('dist/build-info.json', root)); files.push('build-info.json'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  for (const name of await readdir(new URL('src/', root))) if (name.endsWith('.ts')) files.push('src/' + name.replace(/\.ts$/, '.js'));
  for (const name of await readdir(new URL('public/pkg/', root))) if (/\.(js|wasm)$/.test(name)) files.push('pkg/' + name);
  if (!files.includes('pkg/icelines_wasm_bg.wasm') || !files.includes('pkg/icelines_wasm.js')) throw new Error('Generate matching WASM bindings before building');
  const assets = [];
  for (const path of files.sort()) {
    const bytes = await readFile(new URL('dist/' + path, root));
    assets.push({ path, sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length, gzip_bytes: gzipSync(bytes).length });
  }
  const template = await readFile(new URL('service-worker.template.js', root), 'utf8');
  const worker_sha256 = createHash('sha256').update(template).digest('hex');
  const manifest = { schema_version: 1, build: createHash('sha256').update(JSON.stringify({ assets, worker_sha256 })).digest('hex'), worker_sha256, assets };
  await writeFile(new URL('dist/sw.js', root), template.replace('__SHELL_MANIFEST__', JSON.stringify(manifest)));
  await writeFile(new URL('dist/shell-manifest.json', root), JSON.stringify(manifest, null, 2) + '\n');
  return manifest;
}
