import test from 'node:test';
import assert from 'node:assert/strict';
import { refreshSchedule } from '../dist/src/acquisition.js';
globalThis.BroadcastChannel = undefined;
const { ScheduleController, scheduleStorage } = await import('../dist/src/schedule.js');
const { IDBFactory } = await import('fake-indexeddb');
const bytes = value => new TextEncoder().encode(JSON.stringify(value));
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; };

test('schedule acquisition uses a fixed date endpoint and bounded uncached safe reads', async () => {
  let seen;
  const raw = { gameWeek: [{ date: '2026-10-03', games: [] }] };
  const result = await refreshSchedule('2026-10-03', new AbortController().signal, { fetch: async (url, options) => {
    seen = { url: String(url), options }; return new Response(JSON.stringify(raw));
  } });
  assert.deepEqual(JSON.parse(new TextDecoder().decode(result)), raw);
  assert.equal(seen.url, 'https://icelines-relay.giodl73.workers.dev/v1/schedule/2026-10-03');
  assert.equal(seen.options.cache, 'no-store'); assert.equal(seen.options.credentials, 'omit'); assert.equal(seen.options.redirect, 'error');
});

test('invalid schedule dates are refused before any request', async () => {
  for (const date of ['now', '2026-02-30', '2026-13-01', '2026-1-03', 'https://example.com']) {
    await assert.rejects(refreshSchedule(date, new AbortController().signal, { fetch: () => { throw new Error('unexpected network'); } }), /valid schedule date/);
  }
});

test('changing dates without a loaded snapshot does not claim retained data', () => {
  const controller = new ScheduleController(async () => [], () => {});
  controller.cancel(); assert.match(controller.state.message, /Date changed.*No schedule has been loaded/);
  assert.equal(controller.state.snapshot, undefined);
});

test('failed acquisition or shared projection keeps the previous complete schedule', async () => {
  let mode = 'valid';
  const games = [{ game_id: 1, away_score: null, home_score: 0 }];
  const controller = new ScheduleController(async () => { if (mode === 'schema') throw new Error('invalid games'); return games; }, () => {},
    async () => { if (mode === 'network') throw new Error('Source unavailable'); return bytes({ mode }); }, () => '2026-10-03T12:00:00Z');
  await controller.refresh('2026-10-03'); const good = controller.state.snapshot;
  assert.equal(good.games[0].away_score, null); assert.equal(good.games[0].home_score, 0);
  for (mode of ['network', 'schema']) {
    await controller.refresh('2026-10-04'); assert.equal(controller.state.snapshot, good);
    assert.match(controller.state.message, /previous schedule.*stale/); assert.equal(controller.state.loading, false);
  }
});

test('late acquisition after switching dates cannot project or replace the current schedule', async () => {
  const old = deferred(); let oldSignal; const projected = [];
  const controller = new ScheduleController(async value => { projected.push(JSON.parse(new TextDecoder().decode(value)).date); return []; }, () => {},
    async (date, signal) => { if (date === '2026-10-03') { oldSignal = signal; return old.promise; } return bytes({date}); });
  const pending = controller.refresh('2026-10-03'); await controller.refresh('2026-10-04');
  old.resolve(bytes({date:'obsolete'})); await pending;
  assert.equal(oldSignal.aborted, true); assert.deepEqual(projected, ['2026-10-04']);
  assert.equal(controller.state.snapshot.requestedDate, '2026-10-04');
});

test('cancelling an in-flight projection preserves prior data and ignores its late result', async () => {
  let mode = 'ready'; const pending = deferred(), entered = deferred();
  const controller = new ScheduleController(async () => {
    if (mode === 'ready') return [];
    entered.resolve(); return pending.promise;
  }, () => {}, async () => bytes({mode}));
  await controller.refresh('2026-10-03'); const good = controller.state.snapshot;
  mode = 'delayed'; const refreshing = controller.refresh('2026-10-04');
  await entered.promise; controller.cancel();
  pending.resolve([{ game_id: 2 }]); await refreshing;
  assert.equal(controller.state.snapshot, good); assert.equal(controller.state.loading, false);
  assert.match(controller.state.message, /cancelled/);
});

test('saved policy refreshes complete replacements, restores without network, and separates unload/remove', async () => {
  globalThis.indexedDB = new IDBFactory(); let version = 0, projections = 0;
  const project = async () => { projections++; return [{ game_id: 1 }]; };
  const controller = new ScheduleController(project, () => {}, async date => bytes({ gameWeek: [{ date, games: [] }], version: version++ }), () => '2026-04-29T12:00:00Z');
  await controller.refresh('2026-04-29'); assert.equal(controller.state.entries.length, 0);
  await controller.save(); await controller.save(true); const original = controller.state.snapshot.revision;
  await controller.refresh('2026-04-29');
  assert.notEqual(controller.state.snapshot.revision, original);
  assert.equal(controller.state.entries[0].revision, controller.state.snapshot.revision);
  assert.match(controller.state.message, /refreshed and saved/);
  const restored = new ScheduleController(project, () => {}, async () => { throw new Error('No network on recovery'); });
  const before = projections; await restored.restoreActive(); assert.equal(projections, before + 1);
  assert.equal(restored.state.snapshot.revision, controller.state.snapshot.revision);
  assert.equal(restored.state.snapshot.dataset.keepUpdated, true);
  restored.unload(); assert.equal(restored.state.snapshot, undefined); assert.equal(restored.state.entries.length, 1);
  await restored.load(restored.state.entries[0]); const memory = restored.state.snapshot;
  await restored.remove(); assert.equal(restored.state.snapshot, memory); assert.equal(restored.state.entries.length, 0);
  assert.equal(memory.dataset.keepUpdated, false);
});

test('another tab revoking the policy prevents an in-flight refresh from saving again', async () => {
  globalThis.indexedDB = new IDBFactory();
  const pending = deferred(); let held = false;
  const controller = new ScheduleController(async () => [], () => {}, async date => held ? pending.promise : bytes({ gameWeek: [{ date, games: [] }] }), () => '2026-04-29T12:00:00Z');
  await controller.refresh('2026-04-29'); await controller.save(); await controller.save(true);
  const saved = await scheduleStorage.load(controller.state.entries[0]);
  held = true; const refreshing = controller.refresh('2026-04-29');
  await scheduleStorage.save({ ...saved, keepUpdated: false }, saved.revision);
  await controller.reloadLibrary();
  pending.resolve(bytes({ gameWeek: [{ date: '2026-04-29', games: [] }], changed: true }));
  await refreshing;
  assert.equal(controller.state.snapshot.dataset.keepUpdated, false);
  const retained = await scheduleStorage.load(controller.state.entries[0]);
  assert.equal(retained.revision, saved.revision);
  assert.equal(retained.keepUpdated, false);
});

test('refresh cannot overwrite revoked schedule consent when the notification is missed', async () => {
  globalThis.indexedDB = new IDBFactory();
  const pending = deferred(); let held = false;
  const controller = new ScheduleController(async () => [], () => {}, async date => held ? pending.promise : bytes({ gameWeek: [{ date, games: [] }] }), () => '2026-04-29T12:00:00Z');
  await controller.refresh('2026-04-29'); await controller.save(); await controller.save(true);
  const saved = await scheduleStorage.load(controller.state.entries[0]);
  held = true; const refreshing = controller.refresh('2026-04-29');
  await scheduleStorage.save({ ...saved, keepUpdated: false }, saved.revision);
  // Deliberately skip reloadLibrary: transaction consent cannot rely on messaging.
  pending.resolve(bytes({ gameWeek: [{ date: '2026-04-29', games: [] }], changed: true }));
  await refreshing;
  assert.notEqual(controller.state.snapshot.revision, saved.revision);
  const retained = await scheduleStorage.load({ ...controller.state.entries[0], revision: saved.revision });
  assert.equal(retained.keepUpdated, false);
  assert.equal(controller.state.snapshot.dataset.keepUpdated, false);
});

test('failed saved refresh keeps fresh memory and the previous saved revision; new dates stay session-only', async () => {
  globalThis.indexedDB = new IDBFactory(); let fail = false, version = 0;
  const storage = { ...scheduleStorage, save: async (...args) => { if (fail) throw new Error('Quota'); return scheduleStorage.save(...args); } };
  const controller = new ScheduleController(async () => [], () => {}, async date => bytes({ gameWeek: [{ date, games: [] }], version: version++ }), () => '2026-04-29T12:00:00Z', storage);
  await controller.refresh('2026-04-29'); await controller.save(); await controller.save(true);
  const original = controller.state.snapshot.revision; fail = true;
  await controller.refresh('2026-04-29');
  assert.notEqual(controller.state.snapshot.revision, original);
  assert.equal(controller.state.entries[0].revision, original); assert.match(controller.state.message, /Fresh schedule.*saving failed/);
  await controller.refresh('2026-04-30'); assert.equal(controller.state.snapshot.dataset.keepUpdated, false);
  assert.equal(controller.state.entries.length, 1); assert.equal(controller.state.entries[0].metadata.requested_date, '2026-04-29');
});

test('storage denial leaves memory queryable and delayed saved loads cannot replace a new context', async () => {
  globalThis.indexedDB = new IDBFactory();
  const held = deferred(); let hold = false;
  const storage = { ...scheduleStorage, load: entry => hold ? held.promise : scheduleStorage.load(entry) };
  const controller = new ScheduleController(async () => [], () => {}, async date => bytes({ gameWeek: [{ date, games: [] }] }), () => '2026-04-29T12:00:00Z', storage);
  await controller.refresh('2026-04-29'); await controller.save(); const entry = controller.state.entries[0];
  const saved = await scheduleStorage.load(entry); hold = true; const loading = controller.load(entry);
  await controller.refresh('2026-04-30'); const current = controller.state.snapshot;
  held.resolve(saved); await loading; assert.equal(controller.state.snapshot, current);
  const factory = globalThis.indexedDB;
  globalThis.indexedDB = { open() { throw new DOMException('Denied', 'SecurityError'); } };
  try {
    await controller.reloadLibrary(); assert.equal(controller.state.snapshot, current);
    assert.equal(controller.state.storageAvailable, false); assert.match(controller.state.storageMessage, /unavailable/);
  } finally { globalThis.indexedDB = factory; }
  await controller.reloadLibrary(); assert.equal(controller.state.storageAvailable, true);
});

test('portable schedule import runs the shared projection and never enables persistence policy', async () => {
  globalThis.indexedDB = new IDBFactory();
  const controller = new ScheduleController(async () => [], () => {}, async date => bytes({ gameWeek: [{ date, games: [] }] }), () => '2026-04-29T12:00:00Z');
  await controller.refresh('2026-04-29'); await controller.save(); await controller.save(true);
  const exported = controller.state.snapshot.dataset.bytes;
  let projections = 0;
  const imported = new ScheduleController(async () => { projections++; return []; }, () => {});
  await imported.import(exported); assert.equal(projections, 1);
  assert.equal(imported.state.snapshot.dataset.keepUpdated, false); assert.equal(imported.state.entries.length, 0);
  assert.equal(imported.state.snapshot.revision, controller.state.snapshot.revision);
});
