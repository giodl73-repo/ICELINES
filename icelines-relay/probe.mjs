import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
const base = 'https://icelines-relay.giodl73.workers.dev';
const origin = 'https://giodl73-repo.github.io';
const params = new URLSearchParams({ isAggregate: 'false', isGame: 'false', start: '0', limit: '100', cayenneExp: 'seasonId=20252026 and gameTypeId=2' });
const paths = ['skater/bios', 'skater/summary', 'goalie/summary'].map(path => '/stats/rest/en/' + path + '?' + params);
paths.push('/v1/schedule/2026-10-04');
const results = [];
for (const path of paths) {
  const response = await fetch(base + path, { headers: { Origin: origin, Accept: 'application/json' }, redirect: 'error', credentials: 'omit', signal: AbortSignal.timeout(20000) });
  const bytes = new Uint8Array(await response.arrayBuffer());
  const payload = JSON.parse(new TextDecoder().decode(bytes));
  const result = { path, status: response.status, bytes: bytes.byteLength, cors_origin: response.headers.get('Access-Control-Allow-Origin'), sha256: createHash('sha256').update(bytes).digest('hex'), total: payload.total ?? null, error: payload.error ?? null };
  results.push(result); console.log(JSON.stringify(result));
  assert.equal(response.status, 200); assert.equal(result.cors_origin, origin);
  if (path.startsWith('/stats/')) { assert.ok(Array.isArray(payload.data)); assert.ok(Number.isInteger(payload.total)); assert.ok(payload.data.length <= 100); }
  else assert.ok(Array.isArray(payload.gameWeek));
}
console.log(`Validated ${results.length} live relay routes; browser-origin acceptance remains separate.`);
