import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {digest} from '../dist/src/acquisition.js';
const expected = bytes => createHash('sha256').update(bytes).digest('hex');

test('digest preserves exact offset, empty and Unicode bytes without consuming input',async()=>{
  for (const bytes of [new Uint8Array(),new TextEncoder().encode('Slafkovský'),new Uint8Array([99,1,2,3,88]).subarray(1,4)]) {
    const before=bytes.slice(); assert.equal(await digest(bytes),expected(before)); assert.deepEqual(bytes,before);
  }
});
test('ordinary inputs reach WebCrypto as an exact view of the same buffer',async()=>{
  const original=crypto.subtle.digest; const bytes=new Uint8Array([9,1,2,3,8]).subarray(1,4); let observed;
  crypto.subtle.digest=function(algorithm,input){observed=input;return original.call(this,algorithm,input);};
  try { assert.equal(await digest(bytes),expected(bytes)); assert.equal(observed.buffer,bytes.buffer); assert.equal(observed.byteOffset,1); assert.equal(observed.byteLength,3); }
  finally { crypto.subtle.digest=original; }
});
test('mutation after invocation does not change the digest snapshot',async()=>{
  const bytes=new Uint8Array([1,2,3]); const hash=expected(bytes); const pending=digest(bytes); bytes.fill(0); assert.equal(await pending,hash);
});
test('transfer after invocation preserves the pending hash snapshot',async()=>{
  const bytes=new Uint8Array([7,1,2,3,9]).subarray(1,4);const hash=expected(bytes);const pending=digest(bytes);
  structuredClone(bytes.buffer,{transfer:[bytes.buffer]});assert.equal(bytes.byteLength,0);assert.equal(await pending,hash);
});
test('shared-buffer views use a bounded ordinary copy and preserve offsets',async()=>{
  const original=crypto.subtle.digest;const shared=new Uint8Array(new SharedArrayBuffer(5));shared.set([9,1,2,3,8]);const bytes=shared.subarray(1,4);const hash=expected(bytes);let observed;
  crypto.subtle.digest=function(algorithm,input){observed=input;return original.call(this,algorithm,input);};
  try { const pending=digest(bytes);shared.fill(0);assert.equal(await pending,hash);assert.ok(observed.buffer instanceof ArrayBuffer);assert.equal(observed.byteLength,3);assert.notEqual(observed.buffer,bytes.buffer); }
  finally { crypto.subtle.digest=original; }
});
test('native synchronous and asynchronous digest errors remain rejections',async()=>{
  const original=crypto.subtle.digest;const error=new Error('digest refused');
  try { crypto.subtle.digest=()=>{throw error;};await assert.rejects(digest(new Uint8Array([1])),e=>e===error);
    crypto.subtle.digest=()=>Promise.reject(error);await assert.rejects(digest(new Uint8Array([1])),e=>e===error); }
  finally { crypto.subtle.digest=original; }
});
test('already detached input remains an error',async()=>{
  const bytes=new Uint8Array([1]);structuredClone(bytes.buffer,{transfer:[bytes.buffer]});await assert.rejects(digest(bytes),TypeError);
});
