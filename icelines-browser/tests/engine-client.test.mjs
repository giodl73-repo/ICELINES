import test from 'node:test';
import assert from 'node:assert/strict';
import { EngineClient, EngineStoppedError } from '../dist/src/engine-client.js';
function setup(timeout = 120000) {
  const workers = [], failures = [];
  const client = new EngineClient(message => failures.push(message), () => {
    const worker = { messages: [], terminated: false, onmessage: null, onerror: null, onmessageerror: null,
      postMessage(value, transfer = []) { this.messages.push(structuredClone(value, { transfer })); }, terminate() { this.terminated = true; } };
    workers.push(worker); return worker;
  }, timeout);
  return { client, workers, failures };
}
function reply(worker, value, error, overrides = {}) {
  const request = worker.messages.at(-1);
  worker.onmessage({ data: { schema_version: 1, request_id: request.request_id,
    context_generation: request.context_generation, ...(error ? { error } : { value }), ...overrides } });
}
test('matching owned response resolves and ordinary query errors leave engine usable', async () => {
  const { client, workers, failures } = setup();
  const first = client.request('currentSeason', null); reply(workers[0], 20252026); assert.equal(await first, 20252026);
  const second = client.request('query', {}); const rejected = assert.rejects(second, /missing data/);
  reply(workers[0], undefined, {kind:'missing_data', message:'missing data: load a package'}); await rejected;
  assert.equal(client.available, true); assert.deepEqual(failures, []);
});
test('context change rejects obsolete requests immediately and ignores their late responses', async () => {
  const { client, workers } = setup(); const old = client.request('query', {});
  const cancelled = assert.rejects(old, /cancelled/); const oldMessage = workers[0].messages.at(-1);
  client.changeContext(); await cancelled;
  const current = client.request('currentSeason', null);
  workers[0].onmessage({data: {...oldMessage, value:'stale'}});
  reply(workers[0], 20252026); assert.equal(await current, 20252026);
});
test('crash rejects every pending request once and retired workers cannot satisfy new requests', async () => {
  const { client, workers, failures } = setup();
  const pending = [client.request('load', new ArrayBuffer(0)), client.request('query', {})];
  const checks = pending.map(p => assert.rejects(p, EngineStoppedError));
  const retired = workers[0]; retired.onerror(); retired.onmessageerror(); await Promise.all(checks);
  assert.equal(failures.length, 1); assert.equal(retired.terminated, true); assert.equal(client.available, false);
  client.restart(); const current = client.request('currentSeason', null);
  reply(retired, 'obsolete'); reply(workers[1], 20252026); assert.equal(await current, 20252026);
});
test('unavailable engine rejects promptly instead of leaving requests pending', async () => {
  const { client, workers } = setup(); workers[0].onerror();
  await assert.rejects(client.request('query', {}), EngineStoppedError);
});
test('manual restart cancels pending work and creates a distinct worker', async () => {
  const { client, workers, failures } = setup(); const pending = client.request('detail', 1);
  const cancelled = assert.rejects(pending, /cancelled/); client.restart(); await cancelled;
  assert.equal(workers.length, 2); assert.equal(workers[0].terminated, true); assert.deepEqual(failures, []);
});
test('initialization failure and WASM trap responses require worker recovery', async () => {
  const { client, workers, failures } = setup(); const pending = client.request('load', null);
  const rejected = assert.rejects(pending, EngineStoppedError);
  reply(workers[0], undefined, {kind:'engine_failure', message:'WASM initialization failed'}); await rejected;
  assert.equal(client.available, false); assert.equal(failures.length, 1);
});
test('malformed response envelopes and wrong context fail closed', async () => {
  for (const overrides of [{schema_version:2}, {request_id:'1'}, {context_generation:999}, {error:{kind:'bad'}}, {value:1,error:{kind:'bad',message:'bad'}}]) {
    const { client, workers, failures } = setup(); const pending = client.request('query', {});
    const rejected = assert.rejects(pending, EngineStoppedError); reply(workers[0], 1, undefined, overrides); await rejected;
    assert.equal(failures.length, 1); assert.equal(client.available, false);
  }
});
test('postMessage serialization/send failure settles the request and reports one failure', async () => {
  const { client, workers, failures } = setup(); workers[0].postMessage = () => { throw new Error('DataCloneError'); };
  await assert.rejects(client.request('query', {}), EngineStoppedError); assert.equal(failures.length, 1);
});
test('constructor failure is reported asynchronously and can be retried explicitly', async () => {
  const failures = []; const client = new EngineClient(message => failures.push(message), () => { throw new Error('CSP'); });
  await Promise.resolve(); assert.equal(failures.length, 1); assert.equal(client.available, false);
  await assert.rejects(client.request('currentSeason', null), EngineStoppedError);
  client.restart(); await Promise.resolve(); assert.equal(failures.length, 2);
});
test('unresponsive worker is terminated by bounded watchdog', async () => {
  const { client, workers, failures } = setup(10);
  await assert.rejects(client.request('query', {}), /did not respond/);
  assert.equal(workers[0].terminated, true); assert.equal(failures.length, 1);
});

test('explicit staging transfer detaches only the owned copy and keeps backup bytes intact', async () => {
  const {client, workers} = setup();
  const backup = new Uint8Array([1, 2, 3]); const staging = backup.slice().buffer;
  const pending = client.request('load', staging, [staging]);
  assert.equal(staging.byteLength, 0);
  assert.deepEqual([...backup], [1, 2, 3]);
  assert.deepEqual([...new Uint8Array(workers[0].messages.at(-1).payload)], [1, 2, 3]);
  reply(workers[0], {revision:'accepted'});
  assert.equal((await pending).revision, 'accepted');
  assert.deepEqual([...backup], [1, 2, 3]);
});

test('borrowed input stays attached unless the caller explicitly transfers ownership', async () => {
  const {client, workers} = setup(); const bytes = new Uint8Array([4, 5, 6]);
  const pending = client.request('load', bytes.buffer);
  assert.equal(bytes.byteLength, 3);
  assert.notEqual(workers[0].messages.at(-1).payload, bytes.buffer);
  reply(workers[0], 'accepted'); assert.equal(await pending, 'accepted');
  assert.deepEqual([...bytes], [4, 5, 6]);
});