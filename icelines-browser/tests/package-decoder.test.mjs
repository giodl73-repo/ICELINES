import test from 'node:test';
import assert from 'node:assert/strict';
import {gzipSync} from 'node:zlib';
import {expandInWorker} from '../dist/src/package-decoder.js';

class FakeWorker {
  static latest;
  constructor() { FakeWorker.latest=this; this.terminated=0; }
  postMessage(payload,transfer) { this.request=structuredClone(payload,{transfer}); }
  terminate() { this.terminated++; }
}
function setup(t) {
  const previous=globalThis.Worker; globalThis.Worker=FakeWorker;
  t.after(()=>{if(previous===undefined)delete globalThis.Worker;else globalThis.Worker=previous;});
  const controller=new AbortController();
  return {controller,options:{signal:controller.signal,deadline:Date.now()+5000}};
}
test('decoder transfers only the compressed view and preserves the returned length',async t=>{
  const {options}=setup(t);
  // Sentinels outside the view must never become gzip input. The fake response
  // uses spare capacity so the consumer must respect the explicit output length.
  const backing=new Uint8Array([99,1,2,88]);
  const promise=expandInWorker(backing.subarray(1,3),4,options);const worker=FakeWorker.latest;
  assert.deepEqual([...new Uint8Array(worker.request.bytes)],[1,2]);assert.equal(backing.byteLength,4);
  worker.onmessage({data:{schema_version:1,bytes:new Uint8Array([10,20,0,0]).buffer,length:2}});
  assert.deepEqual([...await promise],[10,20]);assert.equal(worker.terminated,1);assert.equal(worker.onmessage,null);
});
test('decoder abort terminates the worker and rejects acquisition',async t=>{
  const {controller,options}=setup(t);const promise=expandInWorker(new Uint8Array([1]),4,options);const worker=FakeWorker.latest;
  controller.abort(new Error('cancelled by user'));
  await assert.rejects(promise,/cancelled by user/);assert.equal(worker.terminated,1);
});
test('decoder deadline terminates an unresponsive worker',async t=>{
  const {options}=setup(t);options.deadline=Date.now()+25;
  const promise=expandInWorker(new Uint8Array([1]),4,options);const worker=FakeWorker.latest;
  await assert.rejects(promise,/deadline exceeded/);assert.equal(worker.terminated,1);
});
test('decoder refuses malformed and oversized responses and always terminates',async t=>{
  const {options}=setup(t);
  for(const data of [{schema_version:2},{schema_version:1,bytes:new ArrayBuffer(5),length:1},{schema_version:1,bytes:new ArrayBuffer(4),length:5}]){
    const promise=expandInWorker(new Uint8Array([1]),4,options);const worker=FakeWorker.latest;
    worker.onmessage({data});await assert.rejects(promise,/Invalid package decoder response/);assert.equal(worker.terminated,1);
  }
});
test('decoder reports crashes, decoding failures and post failures without retrying',async t=>{
  const {options}=setup(t);
  for(const event of ['onerror','onmessageerror']){
    const promise=expandInWorker(new Uint8Array([1]),4,options);const worker=FakeWorker.latest;
    worker[event]({});await assert.rejects(promise,/decoder/);assert.equal(worker.terminated,1);
  }
  const promise=expandInWorker(new Uint8Array([1]),4,options);const worker=FakeWorker.latest;
  worker.onmessage({data:{schema_version:1,error:'invalid gzip'}});await assert.rejects(promise,/invalid gzip/);assert.equal(worker.terminated,1);
  const post=FakeWorker.prototype.postMessage;
  FakeWorker.prototype.postMessage=function(){throw Error('post failed');};
  try {await assert.rejects(expandInWorker(new Uint8Array([1]),4,options),/post failed/);assert.equal(FakeWorker.latest.terminated,1);}
  finally {FakeWorker.prototype.postMessage=post;}
});
test('decoder retains bounded gzip behavior where Worker is unavailable',async t=>{
  const previous=globalThis.Worker;delete globalThis.Worker;
  t.after(()=>{if(previous!==undefined)globalThis.Worker=previous;});
  const expected=new TextEncoder().encode('canonical package bytes');
  const decoded=await expandInWorker(new Uint8Array(gzipSync(expected)),expected.length,{signal:new AbortController().signal,deadline:Date.now()+5000});
  assert.deepEqual(decoded,expected);
});
