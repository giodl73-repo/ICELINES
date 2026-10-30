import test from 'node:test';
import assert from 'node:assert/strict';
import { PollingController, STATS_POLL_MS, SCHEDULE_POLL_MS } from '../dist/src/polling.js';
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; };
function setup(run = async () => {}) {
  const timers = new Map(), states = []; let id = 0;
  const environment = { visible: true, online: true, coordinated: true, busy: false };
  const clock = { set(callback, delay) { const handle = ++id; timers.set(handle, { callback, delay }); return handle; }, clear(handle) { timers.delete(handle); }, random: () => 0.5 };
  const controller = new PollingController(SCHEDULE_POLL_MS, run, Object.fromEntries(Object.keys(environment).map(key => [key, () => environment[key]])), state => states.push(state), clock);
  const tick = async () => {
    const [handle, timer] = [...timers.entries()][0]; timers.delete(handle); timer.callback();
    await new Promise(resolve => setImmediate(resolve));
  };
  return { controller, timers, states, environment, tick };
}

test('polling is explicit and tab-only, has positive jitter, and resets on context change', async () => {
  let calls = 0; const value = setup(async () => { calls++; });
  value.controller.setContext('2026-04-29'); assert.equal(value.timers.size, 0);
  value.controller.setEnabled(true); assert.equal(value.controller.enabled, true);
  assert.equal([...value.timers.values()][0].delay, 63000);
  await value.tick(); assert.equal(calls, 1); assert.equal(value.timers.size, 1);
  value.controller.setContext('2026-04-30'); assert.equal(value.controller.enabled, false); assert.equal(value.timers.size, 0);
  assert.equal(STATS_POLL_MS, 86400000); assert.equal(SCHEDULE_POLL_MS, 60000);
});

test('hidden/offline checks pause queued ticks and abort active polling without catch-up bursts', async () => {
  const held = deferred(); let signal;
  const value = setup(async next => { signal = next; return held.promise; });
  value.controller.setContext('date'); value.controller.setEnabled(true); await value.tick();
  assert.equal(value.timers.size, 0);
  value.environment.visible = false; value.controller.environmentChanged(); assert.equal(signal.aborted, true);
  held.resolve(); await new Promise(resolve => setImmediate(resolve)); assert.equal(value.timers.size, 0);
  value.environment.visible = true; value.environment.online = false; value.controller.environmentChanged(); assert.equal(value.timers.size, 0);
  value.environment.online = true; value.controller.environmentChanged();
  assert.equal([...value.timers.values()][0].delay, 63000);
});

test('manual busy periods and slow responses never overlap polling', async () => {
  const held = deferred(); let calls = 0;
  const value = setup(async () => { calls++; return held.promise; });
  value.controller.setContext('date'); value.controller.setEnabled(true);
  value.environment.busy = true; await value.tick(); assert.equal(calls, 0);
  value.environment.busy = false; await value.tick(); assert.equal(calls, 1); assert.equal(value.timers.size, 0);
  value.controller.environmentChanged(); assert.equal(value.timers.size, 0);
  held.resolve(); await new Promise(resolve => setImmediate(resolve)); assert.equal(value.timers.size, 1);
});

test('failed reads wait a full interval and stopping cancels outstanding work', async () => {
  const value = setup(async () => { throw new Error('Source unavailable'); });
  value.controller.setContext('date'); value.controller.setEnabled(true); await value.tick();
  assert.equal([...value.timers.values()][0].delay, 63000); assert.match(value.states.at(-1).message, /failed/);
  value.controller.setEnabled(false); assert.equal(value.timers.size, 0);
  const held = deferred(); let signal; const active = setup(async next => { signal = next; return held.promise; });
  active.controller.setContext('date'); active.controller.setEnabled(true); await active.tick();
  active.controller.setContext(undefined); assert.equal(signal.aborted, true);
  held.resolve(); await new Promise(resolve => setImmediate(resolve)); assert.equal(active.timers.size, 0);
});

test('missing context or cross-tab coordination never enables automatic reads', () => {
  const value = setup(); value.controller.setEnabled(true); assert.equal(value.controller.enabled, false);
  value.controller.setContext('date'); value.environment.coordinated = false;
  value.controller.setEnabled(true); assert.equal(value.controller.enabled, false); assert.equal(value.timers.size, 0);
  assert.match(value.states.at(-1).message, /unavailable.*manual/);
});
