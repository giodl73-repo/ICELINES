// Explicit live acceptance probe, separate from deterministic CI tests.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { refreshStats, refreshSchedule } from '../icelines-browser/dist/src/acquisition.js';
import { initSync, BrowserEngine } from '../icelines-browser/dist/pkg/icelines_wasm.js';
initSync({ module: await readFile(new URL('../icelines-browser/dist/pkg/icelines_wasm_bg.wasm', import.meta.url)) });
let requests = 0;
const relayFetch = async (url, options) => {
  assert.equal(url.origin, 'https://icelines-relay.giodl73.workers.dev');
  requests++;
  const response = await fetch(url, { ...options, headers: { Origin: 'https://giodl73-repo.github.io' } });
  assert.equal(response.headers.get('Access-Control-Allow-Origin'), 'https://giodl73-repo.github.io');
  return response;
};
const engine = new BrowserEngine();
try {
  const bytes = await refreshStats(20252026, 'regular', new AbortController().signal, { fetch: relayFetch });
  const revision = engine.load_package(bytes);
  const leaders = JSON.parse(engine.query(JSON.stringify({ filter: 'p >= 100', sort: 'points', goalies: false, minimum_games: 1, today: '2026-10-04' })));
  assert.ok(leaders.rows.length > 0);
  const schedule = JSON.parse(engine.schedule_week(await refreshSchedule('2026-10-04', new AbortController().signal, { fetch: relayFetch })));
  assert.ok(Array.isArray(schedule));
  console.log(JSON.stringify({ revision, bytes: bytes.length, requests, leaders: leaders.rows.map(row => ({ name: row.name, points: row.points })), schedule_games: schedule.length }, null, 2));
} finally { engine.free(); }
