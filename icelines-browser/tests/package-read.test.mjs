import test from 'node:test';
import assert from 'node:assert/strict';
import {readPublicBytes} from '../dist/src/network.js';
const url = new URL('https://example.test/ICELINES/data/package.json');
const options = fetch => ({signal:new AbortController().signal,deadline:Date.now()+120000,fetch,wait:async () => {}});

test('public package reads retry safe failures and preserve exact bytes with private caching disabled', async () => {
  let calls = 0; const bytes = new Uint8Array([0,255,7,9]);
  const result = await readPublicBytes(url,4,options(async (_url,request) => {
    assert.equal(request.credentials,'omit'); assert.equal(request.cache,'no-store'); assert.equal(request.redirect,'error');
    calls++; return calls < 3 ? new Response('',{status:503}) : new Response(bytes);
  }));
  assert.deepEqual(result,bytes); assert.equal(calls,3);
});

test('package header and streamed overflows are terminal and cancel the response body', async () => {
  for (const header of [true,false]) {
    let calls = 0; let cancelled = 0;
    await assert.rejects(readPublicBytes(url,4,options(async () => {
      calls++;
      return new Response(new ReadableStream({start(controller){controller.enqueue(new Uint8Array(5));},cancel(){cancelled++;}}),{headers:header ? {'content-length':'5'} : {}});
    })),/advertised byte limit/);
    assert.equal(calls,1); assert.equal(cancelled,1);
  }
});

test('package body stalls share the 20-second request mechanism and four-attempt ceiling', async () => {
  let cancelled = 0; let calls = 0;
  await assert.rejects(readPublicBytes(url,4,{...options(async () => {
    calls++; return new Response(new ReadableStream({cancel(){cancelled++;}}));
  }),timeoutMs:5}),/timed out/);
  assert.equal(calls,4); assert.equal(cancelled,4);
});

test('cancelling a streamed season download releases its reader and never retries', async () => {
  const controller = new AbortController(); let calls = 0; let cancelled = 0;
  await assert.rejects(readPublicBytes(url,4,{...options(async () => {
    calls++; return new Response(new ReadableStream({start(){queueMicrotask(() => controller.abort());},cancel(){cancelled++;}}));
  }),signal:controller.signal}));
  assert.equal(calls,1); assert.equal(cancelled,1);
});

test('expired download deadlines and invalid byte budgets prevent transport', async () => {
  const fetch = async () => {assert.fail('Transport must not start');};
  for (const max of [0,-1,NaN,1.5,100*1024*1024+1]) assert.throws(() => readPublicBytes(url,max,options(fetch)),/byte limit/);
  await assert.rejects(readPublicBytes(url,4,{...options(fetch),deadline:Date.now()-1}),/deadline/);
});
