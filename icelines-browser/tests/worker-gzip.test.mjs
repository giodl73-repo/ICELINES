import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {gzipSync,gunzipSync} from 'node:zlib';
const source=await readFile(new URL('../dist/src/worker.js',import.meta.url),'utf8');
const fixtures=globalThis.__icelinesWorkerGzipFixtures??=new Map();let sequence=0;
async function harness(ready=Promise.resolve()){
  const sent=[];const calls=[];let constructions=0;
  const self={postMessage:(message,options)=>sent.push({message,options})};
  const bindings={default:()=>ready,expand_catalog_gzip:(bytes,maximum)=>{calls.push({bytes:bytes.length,maximum});const result=new Uint8Array(gunzipSync(bytes));assert.ok(result.length<=maximum);return result;},BrowserEngine:class{constructor(){constructions++;throw Error('Expansion must not create an engine');}}};
  const key='gzip-'+ ++sequence;fixtures.set(key,{self,bindings});
  const archive="import { importArchive } from './archive.js';";
  const native="await import(new URL('../pkg/icelines_wasm.js', import.meta.url).href)";
  assert.equal(source.split(archive).length,2);assert.equal(source.split(native).length,2);
  const prefix=`const self=globalThis.__icelinesWorkerGzipFixtures.get(${JSON.stringify(key)}).self;const fixtureBindings=globalThis.__icelinesWorkerGzipFixtures.get(${JSON.stringify(key)}).bindings;\n`;
  const text=prefix+source.replace(archive,"const importArchive=()=>{throw Error('Unexpected archive call');};").replace(native,'fixtureBindings');
  try{await import('data:text/javascript;base64,'+Buffer.from(text).toString('base64')+'#'+key);}finally{fixtures.delete(key);}
  return {calls,get constructions(){return constructions;},send:(id,generation,payload)=>self.onmessage({data:{schema_version:1,request_id:id,context_generation:generation,operation:'expandGzip',payload}}),async responses(count){for(let i=0;i<100;i++){await new Promise(resolve=>setImmediate(resolve));if(sent.length===count)return sent;}throw Error('Worker fixture did not settle');}};
}
test('stateless expansion transfers exact canonical bytes without creating an engine',async()=>{
  const h=await harness();const raw=new TextEncoder().encode('Slafkovský');const wire=new Uint8Array(gzipSync(raw));
  h.send(1,0,{bytes:wire.buffer,maximum:raw.length});
  const [{message,options}]=await h.responses(1);assert.deepEqual(new Uint8Array(message.value),raw);assert.equal(message.request_id,1);assert.deepEqual(options.transfer,[message.value]);assert.equal(h.constructions,0);assert.equal(h.calls.length,1);
});
test('malformed and oversized requests are refused before native copy or engine creation',async()=>{
  const h=await harness();const bad=[null,{}, {bytes:new Uint8Array(3),maximum:1}, {bytes:new ArrayBuffer(100*1024*1024+1),maximum:1}, ...[0,-1,1.5,100*1024*1024+1].map(maximum=>({bytes:new ArrayBuffer(1),maximum}))];
  bad.forEach((payload,index)=>h.send(index+1,0,payload));
  for(const {message} of await h.responses(bad.length))assert.match(message.error.message,/Invalid gzip byte limit or input/);
  assert.deepEqual(h.calls,[]);assert.equal(h.constructions,0);
});
test('new context cancels queued expansion before native initialization completes',async()=>{
  let release;const gate=new Promise(resolve=>release=resolve);const h=await harness(gate);const wire=new Uint8Array(gzipSync('abc'));
  const payload={bytes:wire.buffer,maximum:3};h.send(1,1,payload);h.send(2,2,payload);release();
  const [old,current]=await h.responses(2);assert.equal(old.message.error.kind,'cancelled');assert.deepEqual(new Uint8Array(current.message.value),new TextEncoder().encode('abc'));assert.equal(h.calls.length,1);assert.equal(h.constructions,0);
});
