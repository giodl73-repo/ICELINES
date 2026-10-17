import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const source=await readFile(process.env.ICELINES_DIGEST_WORKER_SOURCE??new URL('../dist/src/worker.js',import.meta.url),'utf8');
const fixtures=globalThis.__icelinesWorkerDigestFixtures??=new Map();let sequence=0;
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
async function harness(ready=Promise.resolve()){
  const sent=[];const calls=[];let constructions=0;
  const self={postMessage:message=>sent.push(message)};
  const bindings={default:()=>ready,package_digest:bytes=>{calls.push(bytes.length);return hash(bytes);},BrowserEngine:class{constructor(){constructions++;throw Error('Digest must not create an engine');}}};
  const key='digest-'+ ++sequence;fixtures.set(key,{self,bindings});
  const archive="import { importArchive } from './archive.js';";
  const native="await import(new URL('../pkg/icelines_wasm.js', import.meta.url).href)";
  assert.equal(source.split(archive).length,2,'Worker dependency fixture must replace exactly one archive import');
  assert.equal(source.split(native).length,2,'Worker dependency fixture must replace exactly one binding import');
  const prefix=`const self=globalThis.__icelinesWorkerDigestFixtures.get(${JSON.stringify(key)}).self;const fixtureBindings=globalThis.__icelinesWorkerDigestFixtures.get(${JSON.stringify(key)}).bindings;\n`;
  const text=prefix+source.replace(archive,"const importArchive=()=>{throw Error('Unexpected archive call');};").replace(native,'fixtureBindings');
  try{await import('data:text/javascript;base64,'+Buffer.from(text).toString('base64')+'#'+key);}finally{fixtures.delete(key);}
  return {calls,get constructions(){return constructions;},send:(id,generation,payload)=>self.onmessage({data:{schema_version:1,request_id:id,context_generation:generation,operation:'digest',payload}}),async responses(count){for(let i=0;i<100;i++){await new Promise(resolve=>setImmediate(resolve));if(sent.length===count)return sent;}throw Error('Worker response fixture did not settle');}};
}
test('stateless digest replies with exact checksum without creating an engine',async()=>{
  const h=await harness();const input=new TextEncoder().encode('Slafkovský');h.send(1,0,input.buffer);
  const [response]=await h.responses(1);assert.equal(response.value,hash(input));assert.equal(response.request_id,1);assert.equal(response.context_generation,0);assert.deepEqual(h.calls,[input.length]);assert.equal(h.constructions,0);
});
test('digest refuses oversized and malformed input before native copy or engine creation',async()=>{
  const h=await harness();h.send(1,0,new ArrayBuffer(100*1024*1024+1));h.send(2,0,new Uint8Array(3));
  const responses=await h.responses(2);for(const response of responses)assert.match(response.error.message,/exceeds 100 MiB or is not an ArrayBuffer/);
  assert.deepEqual(h.calls,[]);assert.equal(h.constructions,0);
});
test('new context cancels queued digest before initialization finishes',async()=>{
  let release;const gate=new Promise(resolve=>release=resolve);const h=await harness(gate);const input=new TextEncoder().encode('abc');
  h.send(1,1,input.buffer);h.send(2,2,input.buffer);release();
  const [old,current]=await h.responses(2);assert.equal(old.error.kind,'cancelled');assert.equal(current.value,hash(input));assert.deepEqual(h.calls,[3]);assert.equal(h.constructions,0);
});
