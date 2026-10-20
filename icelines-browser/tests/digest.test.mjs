import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {digest} from '../dist/src/acquisition.js';
const expected = bytes => createHash('sha256').update(bytes).digest('hex');
const bufferOf = input => input instanceof ArrayBuffer ? input : input.buffer;

test('digest preserves exact offset, empty and Unicode bytes without consuming input',async()=>{
  for (const bytes of [new Uint8Array(),new TextEncoder().encode('Slafkovský'),new Uint8Array([99,1,2,3,88]).subarray(1,4)]) {
    const before=bytes.slice(); assert.equal(await digest(bytes),expected(before)); assert.deepEqual(bytes,before);
  }
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
  try { const pending=digest(bytes);shared.fill(0);assert.equal(await pending,hash);assert.ok(bufferOf(observed) instanceof ArrayBuffer);assert.equal(observed.byteLength,3);assert.notEqual(bufferOf(observed),bytes.buffer); }
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

test('subclass accessors and iterators do not replace the stored bytes',async()=>{
  class SharedView extends Uint8Array { *[Symbol.iterator]() { yield 9; } }
  class ShadowView extends Uint8Array { get buffer() { return new Uint8Array([9,9,9]).buffer; } get byteOffset() { return 2; } get byteLength() { return 0; } }
  for(const bytes of [new SharedView(new SharedArrayBuffer(3)),new ShadowView(3)]) {
    bytes.set([1,2,3]);assert.equal(await digest(bytes),expected(new Uint8Array([1,2,3])));
  }
});
test('resizable out-of-bounds views stay rejected and valid empty views hash empty',async()=>{
  const buffer=new ArrayBuffer(4,{maxByteLength:8});const view=new Uint8Array(buffer,2,2);buffer.resize(1);
  await assert.rejects(digest(view),TypeError);assert.equal(await digest(new Uint8Array(buffer,0,0)),expected(new Uint8Array()));
});

test('resizable input uses a fixed copy and later resizing preserves its hash',async()=>{
 const native=crypto.subtle.digest;let observed;const buffer=new ArrayBuffer(5,{maxByteLength:8});const bytes=new Uint8Array(buffer,1,3);bytes.set([1,2,3]);
 crypto.subtle.digest=function(a,d){observed=d;return native.call(this,a,d);};
 try{const pending=digest(bytes);buffer.resize(0);assert.equal(await pending,expected(new Uint8Array([1,2,3])));assert.notEqual(bufferOf(observed),buffer);assert.equal(bufferOf(observed).resizable,false);}
 finally{crypto.subtle.digest=native;}
});
