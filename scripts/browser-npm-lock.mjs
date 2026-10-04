// Re-pin downloaded known tarballs without changing dependency versions.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const url = new URL('../icelines-browser/package-lock.json', import.meta.url);
const lock = JSON.parse(await readFile(url, 'utf8'));
for (const name of ['fake-indexeddb', 'typescript']) {
  const entry = lock.packages['node_modules/' + name];
  const bytes = await readFile(new URL(`../target/browser-npm/${name}-${entry.version}.tgz`, import.meta.url));
  const [algorithm, expected] = entry.integrity.split('-');
  if (createHash(algorithm).update(bytes).digest('base64') !== expected) throw new Error('Existing package integrity mismatch');
  entry.integrity = 'sha512-' + createHash('sha512').update(bytes).digest('base64');
  entry.resolved = `https://registry.npmjs.org/${name}/-/${name}-${entry.version}.tgz`;
}
await writeFile(url, JSON.stringify(lock, null, 2) + '\n');
