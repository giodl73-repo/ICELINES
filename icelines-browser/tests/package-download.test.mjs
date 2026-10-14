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
test('multi-chunk expansion preserves an offset input view and the exact output limit',async()=>{
  const large = Uint8Array.from({length:256*1024+17},(_,i)=>i%251);
  const wire = new Uint8Array(gzipSync(large));
  const padded = new Uint8Array(wire.length+13); padded.fill(0xff); padded.set(wire,7);
  const view = padded.subarray(7,7+wire.length);
  assert.deepEqual(await expandPackage(view,large.length,options()),large);
  assert.deepEqual(await expandPackage(view,large.length+9,options()),large);
  await assert.rejects(expandPackage(view,large.length-1,options()),/Expanded package/);
  assert.deepEqual(view,wire);
});

test('successful decoding disposes temporary buffers without detaching canonical output or input',async()=>{
  const original=ArrayBuffer.prototype.transfer; const disposed=[];
  try {
    ArrayBuffer.prototype.transfer=function(length){disposed.push({buffer:this,length:this.byteLength});return original.call(this,length);};
    const decoded=await downloadPackage(entry,base,options());
    assert.deepEqual(decoded,raw);assert.equal(decoded.buffer.byteLength,raw.length);
    assert.ok(disposed.some(item=>item.length===compressed.length));
    assert.ok(disposed.length>=2);assert.ok(disposed.every(item=>item.buffer.byteLength===0));
    assert.deepEqual(compressed,new Uint8Array(gzipSync(raw)));
  }finally{ArrayBuffer.prototype.transfer=original;}
});

test('decoder waits for EOF before disposing shared queued output views',async()=>{
  const originalDecoder=globalThis.DecompressionStream;
  const originalTransfer=ArrayBuffer.prototype.transfer;
  const shared=new Uint8Array([1,2,3,4,5,6]);let flushComplete=false;const disposalAfterFlush=[];
  try {
    globalThis.DecompressionStream=class {
      constructor(){return new TransformStream({transform(_,controller){controller.enqueue(shared.subarray(0,3));controller.enqueue(shared.subarray(3));},flush(){flushComplete=true;}});}
    };
    ArrayBuffer.prototype.transfer=function(length){disposalAfterFlush.push(flushComplete);return originalTransfer.call(this,length);};
    const input=new Uint8Array([7,8]);
    const decoded=await expandPackage(input,6,options());
    assert.deepEqual(decoded,new Uint8Array([1,2,3,4,5,6]));
    assert.equal(shared.buffer.byteLength,0);assert.deepEqual(input,new Uint8Array([7,8]));
    assert.deepEqual(disposalAfterFlush,[true]);
  }finally{globalThis.DecompressionStream=originalDecoder;ArrayBuffer.prototype.transfer=originalTransfer;}
});

test('missing or throwing optional disposal preserves exact acquisition and corrupt-data refusal',async()=>{
  const original=ArrayBuffer.prototype.transfer;
  try {
    for(const replacement of [undefined,function(){throw new Error('disposal unavailable');}]){
      ArrayBuffer.prototype.transfer=replacement;
      assert.deepEqual(await downloadPackage(entry,base,options()),raw);
      await assert.rejects(downloadPackage({...entry,sha256:'0'.repeat(64)},base,options()),/Package size or checksum/);
    }
  }finally{ArrayBuffer.prototype.transfer=original;}
});
