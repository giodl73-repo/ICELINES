import type { PackageData } from './types.js';
import { readPublicJSON, type ReadOptions } from './network.js';
import { liveReads } from './live-coordination.js';
export const MAX_BYTES = 100 * 1024 * 1024;
export async function digest(bytes: Uint8Array): Promise<string> {
  const hash = await crypto.subtle.digest('SHA-256', new Uint8Array(bytes).buffer);
  return Array.from(new Uint8Array(hash), value => value.toString(16).padStart(2, '0')).join('');
}
export async function boundedBytes(response: Response): Promise<Uint8Array> {
  if (!response.ok) throw new Error(`Download failed (${response.status})`);
  if (Number(response.headers.get('content-length')) > MAX_BYTES) throw new Error('Package exceeds 100 MiB');
  const reader = response.body?.getReader();
  if (!reader) throw new Error('Download has no body');
  let length = 0; const parts: Uint8Array[] = [];
  try { for (;;) {
    const { value, done } = await reader.read(); if (done) break;
    length += value.length;
    if (length > MAX_BYTES) throw new Error('Package exceeds 100 MiB');
    parts.push(value);
  } } catch (error) { await reader.cancel(); throw error; }
  const bytes = new Uint8Array(length); let offset = 0;
  for (const part of parts) { bytes.set(part, offset); offset += part.length; }
  return bytes;
}
async function report(kind: string, name: string, season: number, type: string, options: ReadOptions): Promise<unknown[]> {
  const rows: unknown[] = [];
  let expectedTotal: number | undefined;
  let previousPlayerId = 0;
  for (let start = 0; start < 10000; start += 100) {
    const url = new URL(`https://icelines-relay.giodl73.workers.dev/stats/rest/en/${kind}/${name}`);
    url.search = new URLSearchParams({ isAggregate: 'false', isGame: 'false', start: String(start), limit: '100', sort: JSON.stringify([{ property: 'playerId', direction: 'ASC' }]), cayenneExp: `seasonId=${season} and gameTypeId=${type === 'playoff' ? 3 : 2}` }).toString();
    const data = await readPublicJSON(url, options) as { data?: unknown[]; total?: number } | null;
    if (!data || !Array.isArray(data.data) || !Number.isInteger(data.total) || data.total! < 0 || data.total! > 10000 || data.data.length > 100) throw new Error('Source schema changed');
    if (expectedTotal !== undefined && expectedTotal !== data.total) throw new Error('Source changed during pagination; refresh again');
    expectedTotal = data.total;
    for (const row of data.data) {
      const playerId = (row as { playerId?: unknown } | null)?.playerId;
      if (typeof playerId !== 'number' || !Number.isSafeInteger(playerId) || playerId <= previousPlayerId) throw new Error('Source returned duplicate or unordered pagination; refresh again');
      previousPlayerId = playerId;
    }
    rows.push(...data.data);
    if (rows.length > data.total!) throw new Error('Source returned inconsistent pagination');
    if (rows.length === data.total) return rows;
    if (!data.data.length) throw new Error('Source returned incomplete pagination');
  }
  throw new Error('Source exceeds bounded pagination');
}
export async function refreshStats(season: number, seasonType: 'regular' | 'playoff', signal: AbortSignal, dependencies: Partial<Omit<ReadOptions, 'signal' | 'deadline'>> = {}): Promise<Uint8Array> {
  return liveReads.run(signal, signal => acquireStats(season, seasonType, signal, dependencies));
}
async function acquireStats(season: number, seasonType: 'regular' | 'playoff', signal: AbortSignal, dependencies: Partial<Omit<ReadOptions, 'signal' | 'deadline'>>): Promise<Uint8Array> {
  const year = Math.floor(season / 10000);
  if (!Number.isInteger(season) || year < 1917 || year > 9998 || season % 10000 !== year + 1 || !['regular', 'playoff'].includes(seasonType)) throw new Error('Invalid season context');
  const controller = new AbortController();
  const stop = (): void => controller.abort(signal.reason);
  signal.throwIfAborted(); signal.addEventListener('abort', stop, { once: true });
  const options: ReadOptions = { ...dependencies, signal: controller.signal, deadline: (dependencies.now ?? Date.now)() + 120000 };
  try {
  // Two requests at most; goalie fetch follows skater acquisition.
  const [bios, stats] = await Promise.all([report('skater', 'bios', season, seasonType, options), report('skater', 'summary', season, seasonType, options)]);
  const biosIds = bios.map(row => (row as { playerId: number }).playerId);
  if (biosIds.length !== stats.length || stats.some((row, index) => (row as { playerId: number }).playerId !== biosIds[index])) throw new Error('Source reports disagree on player coverage; refresh again');
  const goalies = await report('goalie', 'summary', season, seasonType, options);
  const packageData: PackageData = { schema_version: 1, season, season_type: seasonType,
    source: 'NHL live API', observed_at: null, fetched_at: new Date().toISOString(), bios, stats, goalies };
  return new TextEncoder().encode(JSON.stringify(packageData));
  } finally { controller.abort(); signal.removeEventListener('abort', stop); }
}

export async function refreshSchedule(date: string, signal: AbortSignal, dependencies: Partial<Omit<ReadOptions, 'signal' | 'deadline'>> = {}): Promise<Uint8Array> {
  return liveReads.run(signal, signal => acquireSchedule(date, signal, dependencies));
}
async function acquireSchedule(date: string, signal: AbortSignal, dependencies: Partial<Omit<ReadOptions, 'signal' | 'deadline'>>): Promise<Uint8Array> {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || !Number.isFinite(Date.parse(date + 'T00:00:00Z'))
    || new Date(date + 'T00:00:00Z').toISOString().slice(0, 10) !== date) throw new Error('Choose a valid schedule date');
  const raw = await readPublicJSON(new URL('https://icelines-relay.giodl73.workers.dev/v1/schedule/' + date),
    { ...dependencies, signal, deadline: (dependencies.now ?? Date.now)() + 120000 });
  // All hockey/date/score projection and payload validation belong to Rust.
  return new TextEncoder().encode(JSON.stringify(raw));
}
