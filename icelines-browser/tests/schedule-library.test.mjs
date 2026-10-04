import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { IDBFactory, IDBObjectStore, IDBDatabase } from 'fake-indexeddb';
globalThis.BroadcastChannel = undefined;
const library = await import('../dist/src/library.js');
const namespace = 'icelines-v1:' + new URL('../', new URL('../dist/src/library.js', import.meta.url)).pathname;
const bytes = value => new TextEncoder().encode(JSON.stringify(value));
const hash = value => createHash('sha256').update(value).digest('hex');
const season = () => {
  const metadata = { schema_version: 1, season: 20242025, season_type: 'regular', source: 'fixture', observed_at: null, fetched_at: null, bios: [], stats: [], goalies: [] };
  const data = bytes(metadata); return { id: 'season', revision: hash(data), bytes: data, metadata, keepUpdated: false };
};
const schedule = (date = '2026-04-29', time = '2026-04-29T12:00:00Z') => library.makeScheduleDataset(date, bytes({ gameWeek: [{ date, games: [] }] }), time);
async function open(factory, version, upgrade) {
  return new Promise((resolve, reject) => {
    const request = factory.open(namespace, version);
    request.onupgradeneeded = () => upgrade?.(request.result);
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
}
async function transaction(db, names, action) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(names, 'readwrite'); action(tx);
    tx.oncomplete = resolve; tx.onabort = () => reject(tx.error);
  });
}

test('version-one migration preserves season bytes and pointer; older apps refuse upgraded storage', async () => {
  const factory = new IDBFactory(); globalThis.indexedDB = factory;
  const original = season();
  const old = await open(factory, 1, db => {
    db.createObjectStore('datasets', { keyPath: 'id' }).put(original);
    db.createObjectStore('settings').put(original.id, 'active');
  }); old.close();
  assert.equal((await library.listSaved()).length, 1);
  assert.deepEqual((await library.loadSaved(original.id, original.revision)).bytes, original.bytes);
  assert.equal(await library.savedActiveId(), original.id);
  assert.deepEqual(await library.listSavedSchedules(), []);
  await assert.rejects(open(factory, 1), { name: 'VersionError' });
});

test('interrupted additive migration rolls back schema and preserves original records for retry', async () => {
  const factory = new IDBFactory(); globalThis.indexedDB = factory;
  const original = season();
  const old = await open(factory, 1, db => {
    db.createObjectStore('datasets', { keyPath: 'id' }).put(original);
    db.createObjectStore('settings').put(original.id, 'active');
  }); old.close();
  const create = IDBDatabase.prototype.createObjectStore;
  IDBDatabase.prototype.createObjectStore = function (...args) {
    const store = create.apply(this, args); if (args[0] === 'schedules') store.transaction.abort(); return store;
  };
  try { await assert.rejects(library.listSavedSchedules(), { name: 'AbortError' }); }
  finally { IDBDatabase.prototype.createObjectStore = create; }
  const preserved = await open(factory, 1);
  assert.equal(preserved.objectStoreNames.contains('schedules'), false); preserved.close();
  assert.equal(await library.savedActiveId(), original.id);
  assert.deepEqual((await library.loadSaved(original.id, original.revision)).bytes, original.bytes);
});

test('schedule saves, policy, reload and removal are independent from season saves', async () => {
  globalThis.indexedDB = new IDBFactory();
  const original = season(); await library.saveDataset(original, null);
  const first = await schedule(); first.keepUpdated = true; await library.saveSchedule(first, null);
  const [entry] = await library.listSavedSchedules();
  assert.equal('bytes' in entry, false); assert.equal('payload' in entry.metadata, false);
  assert.equal(entry.keepUpdated, true);
  assert.deepEqual((await library.loadSavedSchedule(entry)).bytes, first.bytes);
  assert.equal(await library.savedActiveId(), original.id);
  assert.equal(await library.savedActiveScheduleId(), first.id);
  const next = await schedule('2026-04-30'); await library.saveSchedule(next, null);
  await library.removeSavedSchedule(first.id, first.revision);
  assert.equal(await library.savedActiveScheduleId(), next.id);
  await library.removeSavedSchedule(next.id, next.revision);
  assert.equal(await library.savedActiveScheduleId(), undefined);
  assert.equal(await library.savedActiveId(), original.id);
  assert.deepEqual((await library.loadSaved(original.id, original.revision)).bytes, original.bytes);
});

test('stale schedule removal preserves the replacement and active pointer', async () => {
  globalThis.indexedDB = new IDBFactory();
  const original = await schedule(); await library.saveSchedule(original, null);
  const replacement = await schedule('2026-04-29', '2026-04-29T13:00:00Z');
  await library.saveSchedule(replacement, original.revision);
  await assert.rejects(library.removeSavedSchedule(original.id, original.revision), /changed.*another tab/);
  assert.deepEqual((await library.loadSavedSchedule(replacement)).bytes, replacement.bytes);
  assert.equal(await library.savedActiveScheduleId(), replacement.id);
  await library.removeSavedSchedule(replacement.id, replacement.revision);
  assert.equal(await library.savedActiveScheduleId(), undefined);
});

test('schedule revision conflicts and quota failures preserve package and active pointers', async () => {
  globalThis.indexedDB = new IDBFactory();
  const original = await schedule(); await library.saveSchedule(original, null);
  const changed = await schedule('2026-04-29', '2026-04-29T13:00:00Z');
  await assert.rejects(library.saveSchedule(changed, 'stale'), /another tab/);
  const put = IDBObjectStore.prototype.put;
  IDBObjectStore.prototype.put = function (...args) {
    if (this.name === 'settings' && args[1] === 'activeSchedule') throw new DOMException('Quota', 'QuotaExceededError');
    return put.apply(this, args);
  };
  try { await assert.rejects(library.saveSchedule(changed, original.revision), { name: 'QuotaExceededError' }); }
  finally { IDBObjectStore.prototype.put = put; }
  const [entry] = await library.listSavedSchedules(); assert.equal(entry.revision, original.revision);
  assert.equal(await library.savedActiveScheduleId(), original.id);
  const one = library.saveSchedule(changed, original.revision);
  const two = library.saveSchedule(await schedule('2026-04-29', '2026-04-29T14:00:00Z'), original.revision);
  const outcomes = await Promise.allSettled([one, two]);
  assert.equal(outcomes.filter(value => value.status === 'fulfilled').length, 1);
  await assert.rejects(library.loadSavedSchedule(entry), /changed or was removed/);
});

test('schedule packages bind context to verified bytes, refuse future schemas and export round-trip', async () => {
  const original = await schedule();
  const imported = await library.importScheduleDataset(original.bytes);
  assert.equal(imported.revision, original.revision); assert.equal(imported.keepUpdated, false);
  assert.deepEqual(JSON.parse(new TextDecoder().decode(library.schedulePayload(imported))).gameWeek[0].games, []);
  const wrong = structuredClone(original); wrong.metadata.requested_date = '2026-04-30';
  await assert.rejects(library.validateSavedSchedule(wrong), /corrupt or unsupported/);
  for (const change of [row => { row.schema_version = 2; }, row => { row.extra = true; }, row => { row.fetched_at = '2026-02-30T12:00:00Z'; }, row => { row.requested_date = '2026-02-30'; }]) {
    const value = JSON.parse(new TextDecoder().decode(original.bytes)); change(value);
    await assert.rejects(library.importScheduleDataset(bytes(value)), /corrupt or unsupported/);
  }
  const corrupt = structuredClone(original); corrupt.bytes[0] = 99;
  await assert.rejects(library.validateSavedSchedule(corrupt), /corrupt or unsupported/);
  await assert.rejects(library.importScheduleDataset(new Uint8Array(library.MAX_SCHEDULE_BYTES + 1)), /exceeds 3 MiB/);
});

test('corrupt schedules are preserved and do not disable a valid season library', async () => {
  const factory = new IDBFactory(); globalThis.indexedDB = factory;
  const original = season(); await library.saveDataset(original, null);
  const corrupt = await schedule(); corrupt.revision = '0'.repeat(64);
  const db = await open(factory, 2);
  await transaction(db, 'schedules', tx => tx.objectStore('schedules').put(corrupt)); db.close();
  await assert.rejects(library.listSavedSchedules(), /corrupt or unsupported/);
  await assert.rejects(library.loadSavedSchedule({ ...corrupt, byteLength: corrupt.bytes.length }), /corrupt or unsupported/);
  assert.equal((await library.listSaved()).length, 1);
  const retainedDB = await open(factory, 2);
  const retained = await new Promise(resolve => { const request = retainedDB.transaction('schedules').objectStore('schedules').get(corrupt.id); request.onsuccess = () => resolve(request.result); }); retainedDB.close();
  assert.deepEqual(retained.bytes, corrupt.bytes);
});
