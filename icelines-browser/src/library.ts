import type { Dataset, PackageData, PackageMetadata, SavedDatasetEntry, ScheduleDataset, ScheduleMetadata, SavedScheduleEntry } from './types.js';
import { digest, MAX_BYTES } from './acquisition.js';

const packageKeys = ['schema_version', 'season', 'season_type', 'source', 'observed_at', 'fetched_at', 'bios', 'stats', 'goalies'];
const headerKeys = ['schema_version', 'season', 'season_type', 'source', 'observed_at', 'fetched_at'] as const;
const decoder = new TextDecoder('utf-8', { fatal: true });
function record(value: unknown): value is Record<string, unknown> { return !!value && typeof value === 'object' && !Array.isArray(value); }
// Validate the storage envelope and integrity here. Row/domain validation remains
// in the shared Rust engine, which must accept the bytes before activation.
export async function validateSavedDataset(value: unknown): Promise<Dataset> {
  const invalid = (): never => { throw new Error('Saved package is corrupt or unsupported. Its original record is preserved; reload a trusted package or use a newer application.'); };
  if (!record(value) || typeof value.id !== 'string' || !value.id.length || value.id.length > 1024
    || typeof value.revision !== 'string' || !/^[a-f0-9]{64}$/.test(value.revision)
    || typeof value.keepUpdated !== 'boolean' || !(value.bytes instanceof Uint8Array)
    || !value.bytes.length || value.bytes.length > MAX_BYTES || !record(value.metadata)) return invalid();
  const metadata = value.metadata;
  if (await digest(value.bytes) !== value.revision) return invalid();
  let parsed: unknown;
  try { parsed = JSON.parse(decoder.decode(value.bytes)); } catch { return invalid(); }
  if (!record(parsed) || Object.keys(parsed).some(key => !packageKeys.includes(key))
    || packageKeys.some(key => !(key in parsed)) || parsed.schema_version !== 1
    || !Number.isSafeInteger(parsed.season) || !['regular', 'playoff'].includes(String(parsed.season_type))
    || typeof parsed.source !== 'string' || !parsed.source.trim()
    || !Array.isArray(parsed.bios) || !Array.isArray(parsed.stats) || !Array.isArray(parsed.goalies)
    || headerKeys.some(key => metadata[key] !== parsed[key])
    || [parsed.observed_at, parsed.fetched_at].some(time => time !== null && typeof time !== 'string')) return invalid();
  // Reconstruct all display metadata from the digest-verified package bytes.
  // Stored duplicate arrays are never an independent source of player data.
  return { id: value.id, revision: value.revision, bytes: value.bytes, keepUpdated: value.keepUpdated, metadata: packageMetadata(parsed as unknown as PackageData) };
}
// Keep the raw validated bytes for backup; do not retain a second source-row tree.
export function packageMetadata(data: PackageData): PackageMetadata {
  return { schema_version: data.schema_version, season: data.season, season_type: data.season_type,
    source: data.source, observed_at: data.observed_at, fetched_at: data.fetched_at };
}
const namespace = 'icelines-v1:' + new URL('../', import.meta.url).pathname;
const channel = (() => {
  try { return typeof BroadcastChannel !== 'undefined' ? new BroadcastChannel(namespace) : null; }
  catch { return null; } // Denied messaging must not prevent memory-only use.
})();
export const onLibraryChange = (listener: () => void): void => { if (channel) channel.onmessage = listener; };
function notify(): void {
  // A notification failure cannot undo a committed transaction.
  try { channel?.postMessage('changed'); } catch { /* Other tabs can reload their library. */ }
}
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    let settled = false;
    const fail = (error: unknown): void => { settled = true; reject(error); };
    const request = indexedDB.open(namespace, 2);
    request.onupgradeneeded = event => {
      const db = request.result;
      if ((event as IDBVersionChangeEvent).oldVersion === 0) {
        db.createObjectStore('datasets', { keyPath: 'id' }); db.createObjectStore('settings');
      } else if (!db.objectStoreNames.contains('datasets') || !db.objectStoreNames.contains('settings')) {
        request.transaction!.abort(); return;
      }
      // Additive migration: original season records and pointers remain intact.
      db.createObjectStore('schedules', { keyPath: 'id' });
    };
    request.onerror = () => fail(request.error?.name === 'VersionError' ? new Error('Saved data was opened by a newer application. Reload the latest application; the saved data has been preserved.') : request.error ?? new Error('Local storage is unavailable'));
    request.onblocked = () => fail(new Error('Another tab must close before storage can update. Close it and retry.'));
    request.onsuccess = () => {
      const db = request.result;
      if (settled) { db.close(); return; }
      if (!db.objectStoreNames.contains('datasets') || !db.objectStoreNames.contains('settings') || !db.objectStoreNames.contains('schedules')) {
        db.close(); fail(new Error('Saved library schema is unsupported. Export in-memory data and reload the application; no saved records were deleted.')); return;
      }
      db.onversionchange = () => { db.close(); notify(); };
      settled = true; resolve(db);
    };
  });
}
function readRecord(db: IDBDatabase, id: IDBValidKey, store = 'datasets'): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(store, 'readonly');
    const request = transaction.objectStore(store).get(id);
    transaction.oncomplete = () => resolve(request.result);
    transaction.onabort = () => reject(transaction.error ?? new Error('Local library read was interrupted. Retry local storage.'));
  });
}
export async function loadSaved(id: string, expectedRevision: string): Promise<Dataset> {
  const db = await open();
  let value: unknown;
  try { value = await readRecord(db, id); } finally { db.close(); }
  if (!record(value) || value.revision !== expectedRevision) throw new Error('Saved data changed or was removed in another tab. Reload the library and choose the version to load.');
  const dataset = await validateSavedDataset(value);
  if (dataset.id !== id) throw new Error('Saved package identity does not match its library entry.');
  return dataset;
}
export async function listSaved(): Promise<SavedDatasetEntry[]> {
  const db = await open();
  try {
    const keys = await new Promise<IDBValidKey[]>((resolve, reject) => {
      const transaction = db.transaction('datasets', 'readonly');
      const request = transaction.objectStore('datasets').getAllKeys();
      transaction.oncomplete = () => resolve(request.result);
      transaction.onabort = () => reject(transaction.error ?? new Error('Local library read was interrupted. Retry local storage.'));
    });
    const entries: SavedDatasetEntry[] = [];
    // Read and verify one package at a time. Retain headers only, so opening the
    // library does not keep every season's bytes and decoded player arrays alive.
    // Concurrent deletion is harmless; selection checks the listed revision again.
    for (const key of keys) {
      const value = await readRecord(db, key);
      if (value === undefined) continue;
      const dataset = await validateSavedDataset(value);
      if (dataset.id !== key) throw new Error('Saved package identity does not match its library entry.');
      const metadata = Object.fromEntries(headerKeys.map(key => [key, dataset.metadata[key]])) as SavedDatasetEntry['metadata'];
      entries.push({ id: dataset.id, revision: dataset.revision, keepUpdated: dataset.keepUpdated, byteLength: dataset.bytes.byteLength, metadata });
    }
    return entries;
  } finally { db.close(); }
}
export interface SaveOptions { requireRefreshConsent?: boolean }
export async function saveDataset(dataset: Dataset, expectedRevision: string | null, options: SaveOptions = {}): Promise<void> {
  const snapshot = structuredClone(dataset);
  const validated = await validateSavedDataset(snapshot);
  await saveRecord(validated, expectedRevision, 'datasets', 'active', options);
}
async function saveRecord(snapshot: Dataset | ScheduleDataset, expectedRevision: string | null, storeName: string, pointer: string, options: SaveOptions): Promise<void> {
  const db = await open();
  try { await new Promise<void>((resolve, reject) => {
    const transaction = db.transaction([storeName, 'settings'], 'readwrite');
    const store = transaction.objectStore(storeName);
    let conflict = false;
    let writeError: unknown;
    const request = store.get(snapshot.id);
    request.onsuccess = () => {
      // Consent may change without changing the package digest. Check it in the
      // same serialized transaction as the write; messaging is only a UI hint.
      if (options.requireRefreshConsent && (!snapshot.keepUpdated || request.result?.keepUpdated !== true)) {
        writeError = new Error('Local refresh saving is disabled or the saved copy was removed. Save explicitly to enable it again.');
        transaction.abort(); return;
      }
      if ((request.result?.revision ?? null) !== expectedRevision) { conflict = true; transaction.abort(); return; }
      try { store.put(snapshot); transaction.objectStore('settings').put(snapshot.id, pointer); }
      catch (error) { writeError = error; transaction.abort(); }
    };
    transaction.oncomplete = () => { notify(); resolve(); };
    transaction.onabort = () => reject(conflict ? new Error('Saved data changed in another tab. Reload the library and choose the version to save.') : writeError ?? transaction.error ?? new Error('Save failed; the previous saved version is intact.'));
    transaction.onerror = () => { /* onabort reports rollback */ };
  }); } finally { db.close(); }
}
export async function removeSaved(id: string, expectedRevision: string): Promise<void> { await removeRecord(id, expectedRevision, 'datasets', 'active'); }
async function removeRecord(id: string, expectedRevision: string, storeName: string, pointer: string): Promise<void> {
  const db = await open();
  try { await new Promise<void>((resolve, reject) => {
    const transaction = db.transaction([storeName, 'settings'], 'readwrite');
    const store = transaction.objectStore(storeName);
    let conflict = false;
    const record = store.get(id);
    record.onsuccess = () => {
      if (!expectedRevision || record.result?.revision !== expectedRevision) { conflict = true; transaction.abort(); return; }
      store.delete(id);
      const active = transaction.objectStore('settings').get(pointer);
      active.onsuccess = () => { if (active.result === id) transaction.objectStore('settings').delete(pointer); };
    };
    transaction.oncomplete = () => { notify(); resolve(); };
    transaction.onabort = () => reject(conflict ? new Error('Saved data changed or was removed in another tab. Reload the library and choose the version to remove.') : transaction.error ?? new Error('Removal was interrupted; the saved copy is intact.'));
  }); } finally { db.close(); }
}
export async function savedActiveId(): Promise<string | undefined> { return readActiveId('active'); }
async function readActiveId(pointer: string): Promise<string | undefined> {
  const db = await open();
  try { return await new Promise((resolve, reject) => {
    const transaction = db.transaction('settings', 'readonly');
    const request = transaction.objectStore('settings').get(pointer);
    transaction.oncomplete = () => {
      const id: unknown = request.result;
      if (id !== undefined && (typeof id !== 'string' || !id.length || id.length > 1024)) {
        reject(new Error('Saved active pointer is corrupt. No saved records were deleted.')); return;
      }
      resolve(id as string | undefined);
    };
    transaction.onabort = () => reject(transaction.error ?? new Error('Local library read was interrupted. Retry local storage.'));
  }); } finally { db.close(); }
}

const scheduleHeaderKeys = ['schema_version', 'kind', 'requested_date', 'source', 'observed_at', 'fetched_at'] as const;
export const MAX_SCHEDULE_BYTES = 3 * 1024 * 1024;
function validDate(date: unknown): date is string {
  return typeof date === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(date)
    && Number.isFinite(Date.parse(date + 'T00:00:00Z')) && new Date(date + 'T00:00:00Z').toISOString().slice(0, 10) === date;
}
export async function makeScheduleDataset(date: string, payload: Uint8Array, fetchedAt: string): Promise<ScheduleDataset> {
  if (!payload.length || payload.length > 2 * 1024 * 1024) throw new Error('Schedule response exceeds 2 MiB');
  const metadata: ScheduleMetadata = { schema_version: 1, kind: 'nhl-game-week', requested_date: date,
    source: 'NHL schedule', observed_at: null, fetched_at: fetchedAt };
  const bytes = new TextEncoder().encode(JSON.stringify({ ...metadata, payload: JSON.parse(decoder.decode(payload)) }));
  return validateSavedSchedule({ id: 'nhl-game-week:' + date, revision: await digest(bytes), bytes, metadata, keepUpdated: false });
}
export async function importScheduleDataset(bytes: Uint8Array): Promise<ScheduleDataset> {
  if (!bytes.length || bytes.length > MAX_SCHEDULE_BYTES) throw new Error('Schedule package exceeds 3 MiB');
  const metadata: unknown = JSON.parse(decoder.decode(bytes));
  if (!record(metadata)) throw new Error('Invalid schedule package');
  return validateSavedSchedule({ id: 'nhl-game-week:' + metadata.requested_date, revision: await digest(bytes), bytes, metadata, keepUpdated: false });
}
export async function validateSavedSchedule(value: unknown): Promise<ScheduleDataset> {
  const invalid = (): never => { throw new Error('Saved schedule is corrupt or unsupported. Its original record is preserved.'); };
  if (!record(value) || typeof value.id !== 'string' || typeof value.revision !== 'string' || !/^[a-f0-9]{64}$/.test(value.revision)
    || !(value.bytes instanceof Uint8Array) || !value.bytes.length || value.bytes.length > MAX_SCHEDULE_BYTES
    || typeof value.keepUpdated !== 'boolean' || !record(value.metadata) || await digest(value.bytes) !== value.revision) return invalid();
  let parsed: unknown;
  try { parsed = JSON.parse(decoder.decode(value.bytes)); } catch { return invalid(); }
  if (!record(parsed) || Object.keys(parsed).some(key => ![...scheduleHeaderKeys, 'payload'].includes(key))
    || scheduleHeaderKeys.some(key => !(key in parsed) || parsed[key] !== (value.metadata as Record<string, unknown>)[key])
    || parsed.schema_version !== 1 || parsed.kind !== 'nhl-game-week' || parsed.source !== 'NHL schedule'
    || parsed.observed_at !== null || !validDate(parsed.requested_date) || value.id !== 'nhl-game-week:' + parsed.requested_date
    || typeof parsed.fetched_at !== 'string' || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(parsed.fetched_at)
    || !validDate(parsed.fetched_at.slice(0, 10))
    || !Number.isFinite(Date.parse(parsed.fetched_at)) || !record(parsed.payload)) return invalid();
  const metadata = Object.fromEntries(scheduleHeaderKeys.map(key => [key, parsed[key]])) as unknown as ScheduleMetadata;
  return { id: value.id, revision: value.revision, bytes: value.bytes, keepUpdated: value.keepUpdated, metadata };
}
// Caller must project these bytes through shared Rust before activation.
export function schedulePayload(dataset: ScheduleDataset): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(JSON.parse(decoder.decode(dataset.bytes)).payload));
}
export async function listSavedSchedules(): Promise<SavedScheduleEntry[]> {
  const db = await open();
  try {
    const keys = await new Promise<IDBValidKey[]>((resolve, reject) => {
      const transaction = db.transaction('schedules'); const request = transaction.objectStore('schedules').getAllKeys();
      transaction.oncomplete = () => resolve(request.result);
      transaction.onabort = () => reject(transaction.error ?? new Error('Schedule library read interrupted'));
    });
    const entries: SavedScheduleEntry[] = [];
    for (const key of keys) {
      const value = await readRecord(db, key, 'schedules'); if (value === undefined) continue;
      const dataset = await validateSavedSchedule(value);
      if (dataset.id !== key) throw new Error('Saved schedule identity mismatch');
      entries.push({ id: dataset.id, revision: dataset.revision, keepUpdated: dataset.keepUpdated, metadata: dataset.metadata, byteLength: dataset.bytes.length });
    }
    return entries;
  } finally { db.close(); }
}
export async function loadSavedSchedule(entry: SavedScheduleEntry): Promise<ScheduleDataset> {
  const db = await open(); let value: unknown;
  try { value = await readRecord(db, entry.id, 'schedules'); } finally { db.close(); }
  if (!record(value) || value.revision !== entry.revision) throw new Error('Saved schedule changed or was removed in another tab. Reload the schedule library.');
  const dataset = await validateSavedSchedule(value);
  if (dataset.id !== entry.id) throw new Error('Saved schedule identity mismatch');
  return dataset;
}
export async function saveSchedule(dataset: ScheduleDataset, expectedRevision: string | null, options: SaveOptions = {}): Promise<void> {
  const snapshot = structuredClone(dataset);
  await saveRecord(await validateSavedSchedule(snapshot), expectedRevision, 'schedules', 'activeSchedule', options);
}
export const removeSavedSchedule = (id: string, expectedRevision: string): Promise<void> => removeRecord(id, expectedRevision, 'schedules', 'activeSchedule');
export const savedActiveScheduleId = (): Promise<string | undefined> => readActiveId('activeSchedule');
