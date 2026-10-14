import test from 'node:test';
import assert from 'node:assert/strict';
import { injectCatalogMetadata } from '../metadata-build.mjs';
const moduleSource = 'const knownMetadata = {}; // BUILD_CATALOG_METADATA\nexport {knownMetadata};';
test('generated metadata keeps replacement tokens, quotes and Unicode as data', async () => {
  const source = ["$&","$$","$`","$'",'Kämpf "team"\\notes\n'].join('|');
  const headers = {['a'.repeat(64)]:{schema_version:1,season:20252026,season_type:'regular',source,observed_at:null,fetched_at:null}};
  const generated = injectCatalogMetadata(moduleSource,headers);
  const {knownMetadata} = await import('data:text/javascript;base64,'+Buffer.from(generated).toString('base64'));
  assert.deepEqual(knownMetadata,headers);
});
test('an absent or repeated build marker fails rather than shipping an empty or ambiguous index', () => {
  assert.throws(()=>injectCatalogMetadata('export {};',{}),/Missing or repeated/);
  assert.throws(()=>injectCatalogMetadata(moduleSource+'\n'+moduleSource,{}),/Missing or repeated/);
});
