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

test('trusted expansion backend receives authenticated wire bytes without browser gzip support',async()=>{
  const original=globalThis.DecompressionStream;let calls=0;
  try{
    globalThis.DecompressionStream=undefined;
    const expand=async(bytes,maximum)=>{calls++;assert.deepEqual(bytes,compressed);assert.equal(maximum,raw.length);return raw.slice();};
    assert.deepEqual(await downloadPackage(entry,base,options(),expand),raw);
    assert.equal(calls,1);
    await assert.rejects(downloadPackage({...entry,transport:{...entry.transport,sha256:'0'.repeat(64)}},base,options(),expand),/transport.*checksum/);
    assert.equal(calls,1,'Do not expand unauthenticated wire bytes');
    await assert.rejects(downloadPackage(entry,base,options(),async()=>raw.slice(1)),/Package size or checksum/);
    await assert.rejects(downloadPackage(entry,base,options(),async()=>{const wrong=raw.slice();wrong[0]^=1;return wrong;}),/Package size or checksum/);
  }finally{globalThis.DecompressionStream=original;}
});

test('abort and deadline during asynchronous expansion refuse the result without fallback',async()=>{
  for(const mode of ['abort','deadline']){
    const controller=new AbortController();let now=1;let requests=0;
    await assert.rejects(downloadPackage(entry,base,{...options(),signal:controller.signal,deadline:100,now:()=>now,fetch:async()=>{requests++;return new Response(compressed);}},async()=>{
      await Promise.resolve();if(mode==='abort')controller.abort(new Error('cancelled during expansion'));else now=100;return raw.slice();
    }),mode==='abort'?/cancelled during expansion/:/deadline/);
    assert.equal(requests,1,'Expansion failure must not fetch raw fallback');
  }
});

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

test('decoder input writes preserve offset views and remain bounded without copying',async()=>{
  const original=globalThis.DecompressionStream;const writes=[];
  const padded=Uint8Array.from({length:32768+22},(_,i)=>i%251);const input=padded.subarray(7,padded.length-8);
  const expected=input.slice();
  try{
    globalThis.DecompressionStream=class{constructor(){return new TransformStream({transform(chunk,controller){writes.push({length:chunk.length,buffer:chunk.buffer,offset:chunk.byteOffset});controller.enqueue(chunk);}});}};
    assert.deepEqual(await expandPackage(input,input.length,options()),expected);
    assert.ok(writes.length>1);assert.ok(writes.every(w=>w.length<=8192&&w.buffer===input.buffer));
    assert.equal(writes[0].offset,input.byteOffset);assert.equal(writes.reduce((n,w)=>n+w.length,0),input.length);
    assert.deepEqual(input,expected);
  }finally{globalThis.DecompressionStream=original;}
});

test('native gzip expansion preserves incompressible data across decoder input boundaries',async()=>{
  let state=0x12345678;
  const large=Uint8Array.from({length:256*1024+17},()=>{state^=state<<13;state^=state>>>17;state^=state<<5;return state&255;});
  const wire=new Uint8Array(gzipSync(large));assert.ok(wire.length>8192);
  const original=wire.slice();
  assert.deepEqual(await expandPackage(wire,large.length,options()),large);
  assert.deepEqual(wire,original);
  await assert.rejects(expandPackage(wire,large.length-1,options()),/Expanded package/);
});

test('cancellation stops decoder input before forwarding the whole payload',async()=>{
  const original=globalThis.DecompressionStream;const abort=new AbortController();let forwarded=0;
  const input=new Uint8Array(32769);
  try{
    globalThis.DecompressionStream=class{constructor(){return new TransformStream({transform(chunk,controller){forwarded+=chunk.length;controller.enqueue(chunk);abort.abort(new Error('cancelled during decoder write'));}});}};
    await assert.rejects(expandPackage(input,input.length,{...options(),signal:abort.signal}),/cancelled during decoder write/);
    assert.ok(forwarded>0&&forwarded<input.length);assert.equal(input.byteLength,32769);
  }finally{globalThis.DecompressionStream=original;}
});
