// Manual real-release verification; CI fixtures never download live upstreams.
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { basename } from 'node:path';
import { createHash } from 'node:crypto';
import { importArchive } from '../icelines-browser/dist/src/archive.js';
import init, { BrowserEngine } from '../icelines-browser/public/pkg/icelines_wasm.js';

const [path, expectedHash] = process.argv.slice(2);
if (!path || !/^[a-f0-9]{64}$/.test(expectedHash ?? '')) throw new Error('Usage: node scripts/verify-browser-release.mjs <archive> <published-sha256>');
const archive = await readFile(path);
const digest = createHash('sha256').update(archive).digest('hex');
if (digest !== expectedHash) throw new Error('Published archive digest mismatch');
await init({ module_or_path: await readFile(new URL('../icelines-browser/public/pkg/icelines_wasm_bg.wasm', import.meta.url)) });
const output = new URL('../target/browser-release-import/', import.meta.url);
await mkdir(output, { recursive: true });
const results = [];
for (const seasonType of ['regular', 'playoff']) {
  const bytes = await importArchive(archive, basename(path), seasonType);
  const data = JSON.parse(new TextDecoder().decode(bytes));
  const engine = new BrowserEngine();
  try {
    const revision = engine.load_package(bytes);
    const skaters = JSON.parse(engine.query(JSON.stringify({ filter: '', sort: 'points', goalies: false, minimum_games: 0, today: '2025-07-01' })));
    const goalies = JSON.parse(engine.query(JSON.stringify({ filter: '', sort: 'wins', goalies: true, minimum_games: 0, today: '2025-07-01' })));
    await writeFile(new URL(`${data.season}-${seasonType}.json`, output), bytes);
    results.push({ season: data.season, season_type: seasonType, revision, skaters: skaters.rows.length, goalies: goalies.rows.length,
      leaders: skaters.rows.slice(0, 3).map(row => ({ player_id: row.player_id, name: row.name, points: row.points })) });
  } finally { engine.free(); }
}
const evidence = { archive: basename(path), archive_sha256: digest, archive_bytes: archive.length, results };
await writeFile(new URL('verification.json', output), JSON.stringify(evidence, null, 2) + '\n');
console.log(JSON.stringify(evidence, null, 2));
