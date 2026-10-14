import test from 'node:test';
import assert from 'node:assert/strict';
import {gzipSync} from 'node:zlib';
import {createHash} from 'node:crypto';
import {downloadPackage,expandPackage} from '../dist/src/package-download.js';
const hash = b=>createHash('sha256').update(b).digest('hex');
const raw=new TextEncoder().encode('{"name":"Slafkovský","rows":[1,2,3]}');
const compressed=new Uint8Array(gzipSync(raw));
const entry={url:`data/${hash(raw)}.json`,sha256:hash(raw),bytes:raw.length,transport:{encoding:'gzip',url:`data/${hash(raw)}.json.gz`,sha256:hash(compressed),bytes:compressed.length}};
const options=(bytes=compressed)=>({signal:new AbortController().signal,deadline:Date.now()+10000,fetch:async()=>new Response(bytes)});
const base=new URL('https://example.test/ICELINES/workbench/');

test('compressed and legacy download preserve exact Unicode package bytes',async()=>{
  let fetched;
  assert.deepEqual(await downloadPackage(entry,base,{...options(),fetch:async url=>{fetched=url.href;return new Response(compressed);}}),raw);
  assert.equal(fetched,new URL(entry.transport.url,base).href);
  const {transport,...legacy}=entry;
  assert.deepEqual(await downloadPackage(legacy,base,options(raw)),raw);
});
test('wrong compressed digest, expanded digest, size and broken gzip are refused',async()=>{
  await assert.rejects(downloadPackage({...entry,transport:{...entry.transport,sha256:'0'.repeat(64)}},base,options()),/transport.*checksum/);
  await assert.rejects(downloadPackage({...entry,sha256:'0'.repeat(64)},base,options()),/Package size or checksum/);
  await assert.rejects(downloadPackage({...entry,bytes:raw.length-1},base,options()),/Expanded package/);
  await assert.rejects(downloadPackage({...entry,bytes:raw.length+1},base,options()),/Package size or checksum/);
  const broken=compressed.slice(0,compressed.length-5);
  await assert.rejects(downloadPackage({...entry,transport:{...entry.transport,bytes:broken.length,sha256:hash(broken)}},base,options(broken)));
});
test('browser without gzip support uses the authenticated raw package',async()=>{
  const decoder=globalThis.DecompressionStream;
  try {
    globalThis.DecompressionStream=undefined;
    let fetched;
    assert.deepEqual(await downloadPackage(entry,base,{...options(raw),fetch:async url=>{fetched=url.href;return new Response(raw);}}),raw);
    assert.equal(fetched,new URL(entry.url,base).href);
  } finally { globalThis.DecompressionStream=decoder; }
});
test('expansion bomb is bounded and cancellation/deadline remain errors',async()=>{
  const bomb=new Uint8Array(gzipSync(new Uint8Array(1024*1024)));
  await assert.rejects(expandPackage(bomb,128,options()),/Expanded package/);
  const controller=new AbortController();controller.abort(new Error('cancelled'));
  await assert.rejects(expandPackage(compressed,raw.length,{...options(),signal:controller.signal}),/cancelled/);
  await assert.rejects(expandPackage(compressed,raw.length,{...options(),deadline:0}),/deadline/);
});
