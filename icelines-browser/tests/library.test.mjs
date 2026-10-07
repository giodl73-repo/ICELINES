import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { indexedDB, IDBObjectStore, IDBFactory } from 'fake-indexeddb';

globalThis.indexedDB = indexedDB;
// Tests exercise transactions without creating a long-lived Node messaging handle.
globalThis.BroadcastChannel = undefined;
const { listSaved, loadSaved, saveDataset, removeSaved, savedActiveId, validateSavedDataset } = await import('../dist/src/library.js');
function dataset(id, version) {
  const metadata = { schema_version: 1, season: 20242025, season_type: 'regular', source: 'fixture:' + version, observed_at: null, fetched_at: null, bios: [], stats: [], goalies: [] };
  const bytes = new TextEncoder().encode(JSON.stringify(metadata));
  return { id, revision: createHash('sha256').update(bytes).digest('hex'), bytes, keepUpdated: false, metadata };
}
const revision = version => dataset('', version).revision;

test('stale removal preserves a replacement and its active pointer', async () => {
  globalThis.indexedDB = new IDBFactory();
  try {
    const original = dataset('remove-conflict', 'old');
    const replacement = dataset(original.id, 'new');
    await saveDataset(original, null); await saveDataset(replacement, original.revision);
    await assert.rejects(removeSaved(original.id, original.revision), /changed.*another tab/);
    assert.deepEqual((await loadSaved(replacement.id, replacement.revision)).bytes, replacement.bytes);
    assert.equal(await savedActiveId(), replacement.id);
    await removeSaved(replacement.id, replacement.revision);
    assert.equal((await listSaved()).length, 0);
    assert.equal(await savedActiveId(), undefined);
  } finally { globalThis.indexedDB = indexedDB; }
});

test('refresh saves check current consent inside the transaction even without notifications', async () => {
  globalThis.indexedDB = new IDBFactory();
  try {
    const original = { ...dataset('consent', 'old'), keepUpdated: true };
    await saveDataset(original, null);
    await saveDataset({ ...original, keepUpdated: false }, original.revision);
    const fresh = { ...dataset('consent', 'fresh'), keepUpdated: true };
    await assert.rejects(saveDataset(fresh, original.revision, { requireRefreshConsent: true }), /refresh saving.*disabled/i);
    assert.equal((await loadSaved(original.id, original.revision)).keepUpdated, false);
    await removeSaved(original.id, original.revision);
    await assert.rejects(saveDataset(fresh, null, { requireRefreshConsent: true }), /refresh saving.*disabled/i);
    assert.equal((await listSaved()).length, 0);
    assert.equal(await savedActiveId(), undefined);
    // An explicit user save can still create a new record.
    await saveDataset({ ...fresh, keepUpdated: false }, null);
    assert.equal((await loadSaved(fresh.id, fresh.revision)).keepUpdated, false);
  } finally { globalThis.indexedDB = indexedDB; }
});

test('library entries retain verified headers without package bytes or player arrays', async () => {
  const factory = new IDBFactory();
  globalThis.indexedDB = factory;
  try {
    for (let index = 0; index < 24; index++) {
      const row = dataset('historical-' + index, String(index));
      row.metadata.stats = Array.from({ length: 100 }, (_, player) => ({ player, evidence: 'x'.repeat(100) }));
      row.bytes = new TextEncoder().encode(JSON.stringify(row.metadata));
      row.revision = createHash('sha256').update(row.bytes).digest('hex');
      await saveDataset(row, null);
    }
    const getAll = IDBObjectStore.prototype.getAll;
    IDBObjectStore.prototype.getAll = function () { throw new Error('Bulk package reads exceed the library memory contract'); };
    let entries;
    try { entries = await listSaved(); }
    finally { IDBObjectStore.prototype.getAll = getAll; }
    assert.equal(entries.length, 24);
    for (const entry of entries) {
      assert.equal('bytes' in entry, false);
      assert.equal('stats' in entry.metadata, false);
      assert.equal('bios' in entry.metadata, false);
      assert.equal('goalies' in entry.metadata, false);
      assert.ok(entry.byteLength > 10000);
    }
    const entry = entries[0];
    const selected = await loadSaved(entry.id, entry.revision);
    assert.equal('stats' in selected.metadata, false);
    assert.equal(JSON.parse(new TextDecoder().decode(selected.bytes)).stats.length, 100);
    assert.equal(selected.bytes.byteLength, entry.byteLength);
  } finally { globalThis.indexedDB = indexedDB; }
});

test('selection refuses replaced or removed revisions and returns independently owned bytes', async () => {
  const factory = new IDBFactory();
  globalThis.indexedDB = factory;
  try {
    await saveDataset(dataset('selection', 'old'), null);
    const [entry] = await listSaved();
    await saveDataset(dataset('selection', 'new'), entry.revision);
    await assert.rejects(loadSaved(entry.id, entry.revision), /changed or was removed/);
    const first = await loadSaved(entry.id, revision('new')); first.bytes[0] = 99;
    const second = await loadSaved(entry.id, revision('new'));
    assert.equal(second.bytes[0], dataset('selection', 'new').bytes[0]);
    await removeSaved(entry.id, revision('new'));
    await assert.rejects(loadSaved(entry.id, revision('new')), /changed or was removed/);
  } finally { globalThis.indexedDB = indexedDB; }
});
test('library commits bytes, policy, and active pointer together; stored bytes are owned', async () => {
  const value = dataset('first', 'r1'); await saveDataset(value, null);
  value.bytes[0] = 99;
  const saved = await loadSaved('first', revision('r1'));
  assert.deepEqual(saved.bytes, dataset('first', 'r1').bytes);
  assert.equal(saved.keepUpdated, false); assert.equal(await savedActiveId(), 'first');
});
test('revision conflict rolls back both the package and active pointer', async () => {
  await saveDataset(dataset('second', 'r1'), null);
  await assert.rejects(saveDataset(dataset('first', 'r2'), 'stale'), /another tab/);
  assert.equal((await listSaved()).find(row => row.id === 'first').revision, revision('r1'));
  assert.equal(await savedActiveId(), 'second');
});
test('concurrent writers cannot silently overwrite the same saved revision', async () => {
  const outcomes = await Promise.allSettled([saveDataset(dataset('first', 'writer-a'), revision('r1')), saveDataset(dataset('first', 'writer-b'), revision('r1'))]);
  assert.equal(outcomes.filter(row => row.status === 'fulfilled').length, 1);
  assert.equal(outcomes.filter(row => row.status === 'rejected').length, 1);
  const winner = (await listSaved()).find(row => row.id === 'first');
  assert.ok([revision('writer-a'), revision('writer-b')].includes(winner.revision));
});
test('removing an inactive saved copy preserves the active pointer and memory object', async () => {
  const memory = dataset('third', 'r1'); await saveDataset(memory, null);
  await removeSaved('second', revision('r1'));
  assert.equal(await savedActiveId(), 'third');
  assert.deepEqual(memory.bytes, dataset('third', 'r1').bytes);
});
test('removing the active saved copy clears its pointer without deleting other datasets', async () => {
  await removeSaved('third', revision('r1'));
  assert.equal(await savedActiveId(), undefined);
  assert.equal((await listSaved()).length, 1);
});

test('pointer write failure rolls back a staged replacement and preserves memory', async () => {
  await saveDataset(dataset('rollback', 'old'), null);
  const memory = dataset('rollback', 'new');
  const put = IDBObjectStore.prototype.put;
  IDBObjectStore.prototype.put = function (...args) {
    if (this.name === 'settings') throw new DOMException('Storage quota exceeded', 'QuotaExceededError');
    return put.apply(this, args);
  };
  try { await assert.rejects(saveDataset(memory, revision('old')), { name: 'QuotaExceededError' }); }
  finally { IDBObjectStore.prototype.put = put; }
  assert.equal((await listSaved()).find(row => row.id === 'rollback').revision, revision('old'));
  assert.equal(await savedActiveId(), 'rollback');
  assert.equal(memory.revision, revision('new'));
  assert.deepEqual(memory.bytes, dataset('rollback', 'new').bytes);
});

test('save snapshots bytes and policy before asynchronous storage access', async () => {
  const memory = dataset('snapshot', 'initial');
  const saving = saveDataset(memory, null);
  memory.bytes[0] = 99; memory.keepUpdated = true; memory.revision = 'mutated';
  await saving;
  const saved = await loadSaved('snapshot', revision('initial'));
  assert.equal(saved.revision, revision('initial')); assert.equal(saved.keepUpdated, false);
  assert.equal(saved.bytes[0], dataset('snapshot', 'initial').bytes[0]);
});

test('denied access rejects promptly and saved records remain after access is restored', async () => {
  globalThis.indexedDB = { open() { throw new DOMException('Access denied', 'SecurityError'); } };
  try {
    await assert.rejects(listSaved(), { name: 'SecurityError' });
    await assert.rejects(saveDataset(dataset('denied', 'new'), null), { name: 'SecurityError' });
  } finally { globalThis.indexedDB = indexedDB; }
  assert.equal((await listSaved()).some(row => row.id === 'denied'), false);
  assert.equal((await listSaved()).find(row => row.id === 'rollback').revision, revision('old'));
});

test('notification failure does not misreport a committed save', async () => {
  globalThis.BroadcastChannel = class { postMessage() { throw new Error('Channel closed'); } };
  const isolated = await import('../dist/src/library.js?notification-failure');
  globalThis.BroadcastChannel = undefined;
  await isolated.saveDataset(dataset('notification', 'committed'), null);
  assert.equal((await listSaved()).find(row => row.id === 'notification').revision, revision('committed'));
  assert.equal(await savedActiveId(), 'notification');
});

test('denied BroadcastChannel construction does not prevent local saves', async () => {
  globalThis.BroadcastChannel = class { constructor() { throw new DOMException('Messaging denied', 'SecurityError'); } };
  let isolated;
  try { isolated = await import('../dist/src/library.js?denied-messaging'); }
  finally { globalThis.BroadcastChannel = undefined; }
  await isolated.saveDataset(dataset('messaging-denied', 'r1'), null);
  assert.equal((await listSaved()).find(row => row.id === 'messaging-denied').revision, revision('r1'));
});

test('future database versions are refused without deleting original records', async () => {
  const factory = new IDBFactory();
  const namespace = 'icelines-v1:' + new URL('../', new URL('../dist/src/library.js', import.meta.url)).pathname;
  await new Promise((resolve, reject) => {
    const request = factory.open(namespace, 3);
    request.onupgradeneeded = () => { request.result.createObjectStore('original').put('preserved', 'record'); };
    request.onsuccess = () => { request.result.close(); resolve(); };
    request.onerror = () => reject(request.error);
  });
  globalThis.indexedDB = factory;
  try { await assert.rejects(listSaved(), /newer application/); }
  finally { globalThis.indexedDB = indexedDB; }
  const original = await new Promise((resolve, reject) => {
    const request = factory.open(namespace, 3);
    request.onsuccess = () => {
      const db = request.result; const read = db.transaction('original').objectStore('original').get('record');
      read.onsuccess = () => { db.close(); resolve(read.result); };
    };
    request.onerror = () => reject(request.error);
  });
  assert.equal(original, 'preserved');
});

test('stored byte corruption or revision tampering is refused', async () => {
  const changed = dataset('corrupt', 'r1');
  changed.bytes[changed.bytes.length - 2] = 32;
  await assert.rejects(validateSavedDataset(changed), /corrupt or unsupported/);
  const wrongDigest = dataset('digest', 'r1'); wrongDigest.revision = '0'.repeat(64);
  await assert.rejects(validateSavedDataset(wrongDigest), /corrupt or unsupported/);
});

test('saved metadata cannot override package context or retain independent report arrays', async () => {
  const wrongContext = dataset('context', 'r1'); wrongContext.metadata.season_type = 'playoff';
  await assert.rejects(validateSavedDataset(wrongContext), /corrupt or unsupported/);
  const duplicate = dataset('arrays', 'r1'); duplicate.metadata.stats = [{ injected: 'not in original bytes' }];
  const restored = await validateSavedDataset(duplicate);
  assert.equal('stats' in restored.metadata, false);
  assert.deepEqual(JSON.parse(new TextDecoder().decode(restored.bytes)).stats, []);
});

test('future package schemas, unknown fields, invalid UTF-8 and malformed policies are refused', async () => {
  for (const mutate of [
    row => { row.metadata.schema_version = 2; },
    row => { row.metadata.future = true; },
    row => { row.metadata.goalies = null; },
  ]) {
    const row = dataset('future', 'r1'); mutate(row);
    row.bytes = new TextEncoder().encode(JSON.stringify(row.metadata));
    row.revision = createHash('sha256').update(row.bytes).digest('hex');
    await assert.rejects(validateSavedDataset(row), /corrupt or unsupported/);
  }
  const invalidUTF8 = dataset('utf8', 'r1'); invalidUTF8.bytes = new Uint8Array([255]);
  invalidUTF8.revision = createHash('sha256').update(invalidUTF8.bytes).digest('hex');
  await assert.rejects(validateSavedDataset(invalidUTF8), /corrupt or unsupported/);
  const invalidPolicy = dataset('policy', 'r1'); invalidPolicy.keepUpdated = 'true';
  await assert.rejects(validateSavedDataset(invalidPolicy), /corrupt or unsupported/);
});

test('invalid replacement cannot change the previous saved package or active pointer', async () => {
  await saveDataset(dataset('verified', 'old'), null);
  const corrupt = dataset('verified', 'new'); corrupt.revision = '0'.repeat(64);
  await assert.rejects(saveDataset(corrupt, revision('old')), /corrupt or unsupported/);
  assert.equal((await listSaved()).find(row => row.id === 'verified').revision, revision('old'));
  assert.equal(await savedActiveId(), 'verified');
});

test('invalid active pointer is refused without modifying the settings record', async () => {
  const namespace = 'icelines-v1:' + new URL('../', new URL('../dist/src/library.js', import.meta.url)).pathname;
  const db = await new Promise((resolve, reject) => {
    const request = indexedDB.open(namespace, 2);
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
  const write = value => new Promise((resolve, reject) => {
    const transaction = db.transaction('settings', 'readwrite');
    transaction.objectStore('settings').put(value, 'active');
    transaction.oncomplete = resolve; transaction.onabort = () => reject(transaction.error);
  });
  try {
    await write(42);
    await assert.rejects(savedActiveId(), /pointer is corrupt/);
    const retained = await new Promise((resolve, reject) => {
      const request = db.transaction('settings').objectStore('settings').get('active');
      request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
    });
    assert.equal(retained, 42);
    await write('verified');
  } finally { db.close(); }
});

test('corrupt database records are refused but remain recoverable without automatic deletion', async () => {
  const factory = new IDBFactory();
  const namespace = 'icelines-v1:' + new URL('../', new URL('../dist/src/library.js', import.meta.url)).pathname;
  const corrupt = dataset('persisted-corruption', 'original'); corrupt.revision = '0'.repeat(64);
  await new Promise((resolve, reject) => {
    const request = factory.open(namespace, 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore('datasets', { keyPath: 'id' }).put(corrupt);
      request.result.createObjectStore('settings').put(corrupt.id, 'active');
    };
    request.onsuccess = () => { request.result.close(); resolve(); };
    request.onerror = () => reject(request.error);
  });
  globalThis.indexedDB = factory;
  try {
    await assert.rejects(listSaved(), /corrupt or unsupported/);
    await assert.rejects(loadSaved(corrupt.id, corrupt.revision), /corrupt or unsupported/);
    assert.equal(await savedActiveId(), corrupt.id);
    const retained = await new Promise((resolve, reject) => {
      const request = factory.open(namespace, 2);
      request.onsuccess = () => {
        const db = request.result; const read = db.transaction('datasets').objectStore('datasets').get(corrupt.id);
        read.onsuccess = () => { db.close(); resolve(read.result); };
        read.onerror = () => reject(read.error);
      };
      request.onerror = () => reject(request.error);
    });
    assert.deepEqual(retained.bytes, corrupt.bytes);
    assert.equal(retained.revision, corrupt.revision);
  } finally { globalThis.indexedDB = indexedDB; }
});
