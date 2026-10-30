import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {validateCatalog,readCatalog} from '../dist/src/catalog.js';

const published = JSON.parse(await readFile(new URL('../dist/catalog.json',import.meta.url),'utf8'));
const first = published.packages[0];
const catalog = entry => ({schema_version:1,packages:[entry]});

test('published historical catalog validates all entries and projects only runtime fields', () => {
  const entries = validateCatalog(published);
  assert.equal(entries.length,75);
  assert.equal(entries[0].url,`data/${entries[0].sha256}.json`);
  assert.deepEqual(Object.keys(entries[0]).sort(),['bytes','goalies','id','season','season_type','sha256','skaters','source','url'].sort());
  assert.deepEqual(validateCatalog({schema_version:1,packages:[]}),[]);
});

test('catalog cannot redirect package reads outside the exact content-addressed deployment path', () => {
  for (const url of ['https://other.test/data.json','//other.test/data.json','../data.json','data/../../data.json',first.url+'?token=x',first.url+'#x','data/%2e%2e/file.json','data/'+'0'.repeat(64)+'.json']) {
    assert.throws(() => validateCatalog(catalog({...first,url})),/Invalid/,url);
  }
});

test('catalog rejects schema drift, oversized lists, ambiguous IDs and duplicate season contexts', () => {
  for (const value of [null,[],{}, {schema_version:2,packages:[]},{schema_version:1,packages:Array(2049).fill(first)}]) assert.throws(() => validateCatalog(value),/catalog/);
  assert.throws(() => validateCatalog({schema_version:1,packages:[first,{...first,id:'another'}]}),/Ambiguous/);
  assert.throws(() => validateCatalog({schema_version:1,packages:[first,{...first,season:20242025}]}),/Ambiguous/);
});

test('catalog refuses invalid seasons, counts, sizes, digests and display identities', () => {
  for (const change of [{season:20242026},{season:20042005},{season:'20242025'},{season_type:'current'},
    {skaters:-1},{goalies:NaN},{skaters:10001},{bytes:0},{bytes:100*1024*1024+1},{bytes:1.5},
    {sha256:'invalid'},{id:'https://example.test/'},{source:' '},{source:'x'.repeat(1025)}]) assert.throws(() => validateCatalog(catalog({...first,...change})),/Invalid/);
});

test('catalog acquisition uses bounded safe public reads and validates before returning entries', async () => {
  let calls = 0;
  const entries = await readCatalog(new URL('https://example.test/ICELINES/catalog.json'),async (url,options) => {
    calls++; assert.equal(url.href,'https://example.test/ICELINES/catalog.json');
    assert.equal(options.cache,'no-store'); assert.equal(options.credentials,'omit'); assert.equal(options.redirect,'error'); assert.ok(options.signal);
    return new Response(JSON.stringify(published));
  });
  assert.equal(entries.length,75); assert.equal(calls,1);
  await assert.rejects(readCatalog(new URL('https://example.test/ICELINES/catalog.json'),async () => new Response('{}')),/catalog/);
  await assert.rejects(readCatalog(new URL('https://example.test/ICELINES/catalog.json'),async () => new Response('missing',{status:404})),/404/);
});
