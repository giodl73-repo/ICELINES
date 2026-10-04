import test from 'node:test';
import assert from 'node:assert/strict';
import { LiveReadCoordinator } from '../dist/src/live-coordination.js';
const deferred = () => { let resolve; const promise = new Promise(a => { resolve = a; }); return { promise, resolve }; };
class Locks {
  queue = []; active = false; names = [];
  request(name, options, callback) {
    this.names.push(name);
    return new Promise((resolve, reject) => {
      const entry = { callback, resolve, reject, stop: () => { this.queue = this.queue.filter(value => value !== entry); reject(options.signal.reason); }, signal: options.signal };
      options.signal.addEventListener('abort', entry.stop, { once: true }); this.queue.push(entry); this.pump();
    });
  }
  pump() {
    if (this.active || !this.queue.length) return;
    const entry = this.queue.shift(); entry.signal.removeEventListener('abort', entry.stop);
    if (entry.signal.aborted) { entry.reject(entry.signal.reason); this.pump(); return; }
    this.active = true;
    Promise.resolve().then(entry.callback).then(entry.resolve, entry.reject).finally(() => { this.active = false; this.pump(); });
  }
}

test('two tab coordinators serialize every live acquisition under the same deployment lock', async () => {
  const locks = new Locks(); const first = new LiveReadCoordinator(locks), second = new LiveReadCoordinator(locks);
  let inFlight = 0, peak = 0; const held = deferred();
  const one = first.run(new AbortController().signal, async () => { peak = Math.max(peak, ++inFlight); await held.promise; inFlight--; return 'first'; });
  const two = second.run(new AbortController().signal, async () => { peak = Math.max(peak, ++inFlight); inFlight--; return 'second'; });
  await new Promise(resolve => setImmediate(resolve)); assert.equal(peak, 1); assert.equal(locks.queue.length, 1);
  held.resolve(); assert.deepEqual(await Promise.all([one, two]), ['first', 'second']);
  assert.equal(peak, 1); assert.equal(new Set(locks.names).size, 1); assert.match(locks.names[0], /^icelines-live:/);
});

test('cancellation while queued never starts acquisition and a later waiter still runs', async () => {
  const locks = new Locks(), coordinator = new LiveReadCoordinator(locks); const held = deferred();
  const one = coordinator.run(new AbortController().signal, () => held.promise);
  const abort = new AbortController(); let reads = 0;
  const two = coordinator.run(abort.signal, async () => { reads++; });
  const rejected = assert.rejects(two, /cancelled/); abort.abort(new Error('cancelled')); await rejected;
  const three = coordinator.run(new AbortController().signal, async () => { reads++; });
  held.resolve(); await one; await three; assert.equal(reads, 1);
});

test('the shared foreground deadline includes waiting for another tab', async () => {
  const locks = new Locks(); const held = deferred();
  const one = new LiveReadCoordinator(locks).run(new AbortController().signal, () => held.promise);
  const queued = new LiveReadCoordinator(locks, 10); let reads = 0;
  await assert.rejects(queued.run(new AbortController().signal, async () => { reads++; }), /deadline exceeded/);
  held.resolve(); await one; assert.equal(reads, 0);
});

test('a deadline releases the lock even if an adapter stops responding', async () => {
  const locks = new Locks(); const held = deferred();
  await assert.rejects(new LiveReadCoordinator(locks, 10).run(new AbortController().signal, () => held.promise), /deadline exceeded/);
  const next = await new LiveReadCoordinator(locks).run(new AbortController().signal, async () => 'recovered');
  assert.equal(next, 'recovered'); held.resolve();
});

test('manual fallback serializes locally and queued cancellation cannot bypass the current reader', async () => {
  const coordinator = new LiveReadCoordinator(undefined), held = deferred();
  assert.equal(coordinator.crossTab, false); const order = [];
  const one = coordinator.run(new AbortController().signal, async () => { order.push('one'); await held.promise; });
  const abort = new AbortController(); const two = coordinator.run(abort.signal, async () => { order.push('cancelled'); });
  const rejected = assert.rejects(two, /cancelled/); abort.abort(new Error('cancelled')); await rejected;
  const three = coordinator.run(new AbortController().signal, async () => { order.push('three'); });
  await new Promise(resolve => setImmediate(resolve)); assert.deepEqual(order, ['one']);
  held.resolve(); await one; await three; assert.deepEqual(order, ['one', 'three']);
});
