import assert from 'node:assert/strict';
import { readFile, readdir, lstat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';
import init, { BrowserEngine } from '../icelines-browser/dist/pkg/icelines_wasm.js';
const dist = new URL('../icelines-browser/dist/', import.meta.url);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const shell = JSON.parse(await readFile(new URL('shell-manifest.json', dist), 'utf8'));
const catalog = JSON.parse(await readFile(new URL('catalog.json', dist), 'utf8'));
assert.equal(shell.schema_version, 1); assert.equal(catalog.schema_version, 1);
const allowed = new Set(['.nojekyll', 'sw.js', 'shell-manifest.json', 'pkg/icelines_wasm.d.ts', 'pkg/icelines_wasm_bg.wasm.d.ts']);
for (const asset of shell.assets) {
  assert.match(asset.path, /^(?:index\.html|style\.css|catalog\.json|build-info\.json|src\/[a-z-]+\.js|pkg\/icelines_wasm(?:_bg)?\.(?:js|wasm))$/);
  const bytes = await readFile(new URL(asset.path, dist));
  assert.equal(hash(bytes), asset.sha256, 'Shell asset digest mismatch');
  assert.equal(bytes.length, asset.bytes, 'Shell byte measurement mismatch');
  assert.equal(gzipSync(bytes).length, asset.gzip_bytes, 'Shell gzip measurement mismatch');
  allowed.add(asset.path);
}
const template = await readFile(new URL('../icelines-browser/service-worker.template.js', import.meta.url), 'utf8');
assert.equal(shell.worker_sha256, hash(template));
assert.equal(shell.build, hash(JSON.stringify({ assets: shell.assets, worker_sha256: shell.worker_sha256 })));
assert.equal(await readFile(new URL('sw.js', dist), 'utf8'), template.replace('__SHELL_MANIFEST__', JSON.stringify(shell)));
const wasm = await readFile(new URL('pkg/icelines_wasm_bg.wasm', dist));
assert.ok(WebAssembly.validate(wasm));
const info = JSON.parse(await readFile(new URL('build-info.json', dist), 'utf8'));
assert.equal(info.wasm_sha256, hash(wasm));
await init({ module_or_path: wasm });
// Hand-audited tied players: names and secondary points deliberately disagree
// with the canonical ID tie. Exercise the distributed WASM, not just Rust tests.
const seed = JSON.parse(await readFile(new URL(catalog.packages[0].url, dist), 'utf8'));
const cases = [
  { id: 2, name: 'Alpha', gp: 82, goals: 20, assists: 30 },
  { id: 1, name: 'Zed', gp: 82, goals: 20, assists: 30 },
  { id: 3, name: 'Beta', gp: 80, goals: 20, assists: 50 },
];
const fixture = { ...seed, source: 'count-order-fixture', goalies: [],
  bios: cases.map(row => ({ ...seed.bios[0], playerId: row.id, skaterFullName: row.name,
    gamesPlayed: row.gp, goals: row.goals, assists: row.assists, points: row.goals + row.assists })),
  stats: cases.map(row => ({ ...seed.stats[0], playerId: row.id, gamesPlayed: row.gp,
    goals: row.goals, assists: row.assists, points: row.goals + row.assists })),
};
const fixtureEngine = new BrowserEngine();
try {
  const weekBytes = await readFile(new URL('../icelines-sources/tests/fixtures/browser-schedule-week.json', import.meta.url));
  const games = JSON.parse(fixtureEngine.schedule_week(weekBytes));
  assert.deepEqual(games.map(game => game.game_id), [2025030104, 2025030125]);
  assert.equal(games[0].home_score, 0); assert.equal(games[1].away_score, null);
  assert.equal(games[0].series_game, 'Game 4'); assert.equal(games[0].away_wins, 3);
  assert.throws(() => fixtureEngine.schedule_week(new TextEncoder().encode('{"gameWeek":[]}')));
  fixtureEngine.load_package(new TextEncoder().encode(JSON.stringify(fixture)));
  fixtureEngine.schedule_week(weekBytes);
  assert.throws(() => fixtureEngine.schedule_week(new Uint8Array(2 * 1024 * 1024 + 1)));
  for (const [sort, expected] of [['points', [3, 1, 2]], ['goals', [1, 2, 3]], ['assists', [3, 1, 2]], ['gp', [1, 2, 3]]]) {
    const result = JSON.parse(fixtureEngine.query(JSON.stringify({ filter: '', sort, goalies: false, minimum_games: 0, today: '2026-10-03' })));
    assert.deepEqual(result.rows.map(row => row.player_id), expected, `Distributed WASM count ordering: ${sort}`);
  }
} finally { fixtureEngine.free(); }
const identities = new Set();
const runtimeGzipBytes = shell.assets.reduce((sum, asset) => sum + asset.gzip_bytes, 0)
  + gzipSync(await readFile(new URL('sw.js', dist))).length
  + gzipSync(await readFile(new URL('shell-manifest.json', dist))).length;
let largestStarterGzipBytes = runtimeGzipBytes;
for (const entry of catalog.packages) {
  assert.match(entry.url, /^data\/[a-f0-9]{64}\.json$/);
  assert.equal(entry.url, `data/${entry.sha256}.json`);
  assert.ok(!identities.has(entry.id), 'Duplicate catalog identity'); identities.add(entry.id);
  const bytes = await readFile(new URL(entry.url, dist));
  assert.equal(bytes.length, entry.bytes); assert.equal(hash(bytes), entry.sha256);
  largestStarterGzipBytes = Math.max(largestStarterGzipBytes, runtimeGzipBytes + gzipSync(bytes).length);
  const engine = new BrowserEngine();
  try {
    assert.equal(engine.load_package(bytes), entry.sha256);
    const result = JSON.parse(engine.query(JSON.stringify({ filter: '', sort: 'points', goalies: false, minimum_games: 0, today: '2025-07-01' })));
    assert.equal(result.season, entry.season); assert.equal(result.season_type, entry.season_type);
    assert.ok(result.rows.length > 0 && result.rows.length <= entry.skaters);
  } finally { engine.free(); }
  allowed.add(entry.url);
}
async function inventory(directory, prefix = '') {
  for (const name of await readdir(directory)) {
    const path = prefix + name; const url = new URL(name, directory); const stat = await lstat(url);
    assert.ok(!stat.isSymbolicLink(), 'Publication cannot contain links');
    if (stat.isDirectory()) await inventory(new URL(name + '/', directory), path + '/');
    else assert.ok(allowed.has(path), `Unexpected publication file: ${path}`);
  }
}
await inventory(dist);
assert.ok(largestStarterGzipBytes <= 10 * 1024 * 1024, 'Application plus any single catalog package exceeds the provisional 10 MiB gzip budget');
assert.ok(catalog.packages.length > 0);
assert.ok(catalog.packages.length >= 9, 'Residency acceptance requires at least nine distinct windows');
const residentEngine = new BrowserEngine();
try {
  const windows = catalog.packages.slice(0,9);
  for (const entry of windows) residentEngine.load_package(await readFile(new URL(entry.url,dist)));
  const revisions = JSON.parse(residentEngine.resident_revisions());
  assert.equal(revisions.length,8);
  assert.ok(!revisions.includes(windows[0].sha256), 'Oldest of nine windows must be evicted');
  assert.throws(() => residentEngine.select_resident(windows[0].sha256));
  residentEngine.select_resident(windows[1].sha256);
  const restored = JSON.parse(residentEngine.query(JSON.stringify({filter:'',sort:'points',goalies:false,minimum_games:0,today:'2026-10-04'})));
  assert.equal(restored.revision,windows[1].sha256);
  assert.equal(restored.season,windows[1].season);
  assert.equal(restored.season_type,windows[1].season_type);
  residentEngine.unload_active();
  assert.equal(JSON.parse(residentEngine.resident_revisions()).length,7);
  assert.throws(() => residentEngine.select_resident(windows[1].sha256));
  residentEngine.select_resident(windows[8].sha256);
} finally { residentEngine.free(); }
assert.equal((await readFile(new URL('.nojekyll', dist))).length, 0);
console.log(`Verified ${shell.assets.length} shell assets, four hand-audited count sorts, eight-window residency and ${catalog.packages.length} public packages with actual WASM loading and queries. Largest app plus single-package gzip sum: ${largestStarterGzipBytes} bytes (10 MiB budget).`);
