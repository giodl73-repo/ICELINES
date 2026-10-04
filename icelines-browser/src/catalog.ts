import type { CatalogEntry } from './types.js';
import { MAX_BYTES } from './acquisition.js';
import { readPublicJSON } from './network.js';

function object(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}
export function validateCatalog(value: unknown): CatalogEntry[] {
  if (!object(value) || value.schema_version !== 1 || !Array.isArray(value.packages) || value.packages.length > 2048) throw new Error('Unsupported or oversized catalog');
  const ids = new Set<string>(); const contexts = new Set<string>();
  return value.packages.map(entry => {
    if (!object(entry)) throw new Error('Invalid catalog entry');
    const {id,season,season_type,source,url,sha256,bytes,skaters,goalies} = entry;
    const year = typeof season === 'number' ? Math.floor(season / 10000) : 0;
    if (typeof id !== 'string' || !/^[A-Za-z0-9_.:-]{1,128}$/.test(id)
      || !Number.isSafeInteger(season) || year < 1900 || year > 2999 || season !== year * 10000 + year + 1 || season === 20042005
      || (season_type !== 'regular' && season_type !== 'playoff')
      || typeof source !== 'string' || !source.trim() || source.length > 1024
      || typeof sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(sha256)
      || url !== `data/${sha256}.json`
      || typeof bytes !== 'number' || !Number.isSafeInteger(bytes) || bytes <= 0 || bytes > MAX_BYTES
      || [skaters,goalies].some(count => typeof count !== 'number' || !Number.isSafeInteger(count) || count < 0 || count > 10000)) throw new Error('Invalid catalog identity, context, package path or size');
    const context = `${season}:${season_type}`;
    if (ids.has(id) || contexts.has(context)) throw new Error('Ambiguous catalog identity or season context');
    ids.add(id); contexts.add(context);
    return {id,season:season as number,season_type,source,url:url as string,sha256,bytes,skaters:skaters as number,goalies:goalies as number};
  });
}

export async function readCatalog(url: URL, request: typeof fetch = fetch): Promise<CatalogEntry[]> {
  const controller = new AbortController();
  // The shared reader bounds body size, request time, retry count and redirects.
  return validateCatalog(await readPublicJSON(url,{signal:controller.signal,deadline:Date.now() + 20000,fetch:request}));
}
