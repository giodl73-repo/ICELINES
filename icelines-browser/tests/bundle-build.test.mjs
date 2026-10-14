import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { bundleFrontend } from '../bundle-build.mjs';
import { injectCatalogMetadata } from '../metadata-build.mjs';

async function fixture() {
  const root = pathToFileURL((await mkdtemp(join(tmpdir(), 'icelines-bundle-'))) + '/');
  await mkdir(new URL('src/', root));
  await writeFile(new URL('package.json', root), '{"type":"module"}');
  await writeFile(new URL('src/catalog-metadata.js', root), 'const knownMetadata = {}; // BUILD_CATALOG_METADATA\nexport {knownMetadata};');
  await writeFile(new URL('src/main.js', root), "import {knownMetadata} from './catalog-metadata.js'; export {knownMetadata}; export const workerUrl=new URL('./worker.js',import.meta.url).href;");
  return root;
}
test('bundling refuses uninjected metadata before changing the entry point', async () => {
  const root = await fixture();
  const before = await readFile(new URL('src/main.js', root));
  await assert.rejects(bundleFrontend(root), /after authenticated catalog metadata injection/);
  assert.deepEqual(await readFile(new URL('src/main.js', root)), before);
});
test('bundle preserves Unicode metadata and worker URL relative to the entry point', async () => {
  const root = await fixture();
  const headers = { ['a'.repeat(64)]: { schema_version: 1, season: 20252026, season_type: 'regular', source: 'Kämpf "$&"\\notes\n', fetched_at: null, observed_at: null } };
  const metadata = new URL('src/catalog-metadata.js', root);
  await writeFile(metadata, injectCatalogMetadata(await readFile(metadata, 'utf8'), headers));
  await bundleFrontend(root);
  await writeFile(metadata, 'throw Error("Separate metadata must not execute");');
  const result = await import(new URL('src/main.js', root));
  assert.deepEqual(result.knownMetadata, headers);
  assert.equal(result.workerUrl, new URL('src/worker.js', root).href);
});
