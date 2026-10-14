import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
// Match library tests: metadata projection needs no live Node messaging handle.
globalThis.BroadcastChannel = undefined;
const { metadataForVerifiedCatalogBytes } = await import('../dist/src/catalog-metadata.js');

const encode = value => new TextEncoder().encode(JSON.stringify(value));
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const project = ({schema_version,season,season_type,source,observed_at,fetched_at}) => ({schema_version,season,season_type,source,observed_at,fetched_at});

test('every generated catalog header matches the complete independent JSON parse', async () => {
  const catalog = JSON.parse(await readFile(new URL('../dist/catalog.json',import.meta.url),'utf8'));
  for (const entry of catalog.packages) {
    const bytes = new Uint8Array(await readFile(new URL('../dist/'+entry.url,import.meta.url)));
    assert.equal(digest(bytes),entry.sha256);
    const expected = project(JSON.parse(new TextDecoder().decode(bytes)));
    const actual = metadataForVerifiedCatalogBytes(bytes,entry.sha256);
    assert.deepEqual(actual,expected);
    actual.source = 'caller mutation';
    assert.deepEqual(metadataForVerifiedCatalogBytes(bytes,entry.sha256),expected);
  }
});
test('unknown digests retain full JSON parsing, reordered Unicode headers and duplicate-key behavior', () => {
  const data = {bios:[],source:'Kämpf \\"bios\\": [',fetched_at:null,season_type:'regular',season:20252026,observed_at:null,schema_version:1};
  const bytes = encode(data);
  assert.deepEqual(metadataForVerifiedCatalogBytes(bytes,digest(bytes)),project(data));
  const duplicate = new TextEncoder().encode('{"source":"first","bios":[],"source":"last","season":20252026}');
  assert.equal(metadataForVerifiedCatalogBytes(duplicate,digest(duplicate)).source,'last');
});
test('unknown malformed bodies and UTF-8 remain refused before activation', () => {
  const broken = new TextEncoder().encode('{"schema_version":1,"season":20252026,"bios":['+' '.repeat(9000)+'!]}');
  assert.throws(()=>metadataForVerifiedCatalogBytes(broken,digest(broken)),SyntaxError);
  const badUtf8 = new Uint8Array([123,34,97,34,58,34,255,34,125]);
  assert.throws(()=>metadataForVerifiedCatalogBytes(badUtf8,digest(badUtf8)));
});
