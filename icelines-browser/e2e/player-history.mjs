// Actual WASM history across already-loaded windows; localhost/public fixtures only.
import { chromium, expect } from '@playwright/test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdtemp, stat } from 'node:fs/promises';
import { resolve, dirname, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const root = resolve(repository, 'icelines-browser/dist');
const output = await mkdtemp(resolve(repository, 'target/history-browser-'));
const report = { verified: false, scope: 'Local owned Chromium: current/resident regular/playoff projections, missing records, goalie values and offline reads. Not complete-career, phone, memory or live-data acceptance.' };
const server = createServer(async (request, response) => {
  try {
    const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const file = resolve(root, '.' + (path === '/' ? '/index.html' : path));
    if (!file.startsWith(root + sep) || !(await stat(file)).isFile()) throw Error();
    response.writeHead(200, { 'Content-Type': ({ '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' })[extname(file)] || 'application/octet-stream' }).end(await readFile(file));
  } catch { response.writeHead(404).end(); }
});
await new Promise(accept => server.listen(0, '127.0.0.1', accept));
const browser = await chromium.launch({ channel: 'chromium', headless: true });
const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
const page = await context.newPage();
const payloadRequests = [];
page.on('request', request => { if (new URL(request.url()).pathname.includes('/data/')) payloadRequests.push(request.url()); });
const library = async () => { if (!await page.locator('#data-library').evaluate(element => element.open)) await page.locator('#data-library > summary').click(); };
const exportedRows = async () => {
  const [download] = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);
  return JSON.parse(await readFile(await download.path(), 'utf8')).rows;
};
const open = async name => {
  await page.locator('#rows td:nth-child(2) button').filter({ hasText: name }).first().click();
  await expect(page.locator('#player-dialog')).toBeVisible();
};
let failure;
try {
  report.build = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await expect(page.locator('#catalog option').first()).toBeAttached();
  const expected = [];
  for (const [id, title] of [['20242025-regular', '2024–25 · Regular season'], ['20242025-playoff', '2024–25 · Playoffs'], ['20252026-regular', '2025–26 · Regular season']]) {
    await library(); await page.locator('#catalog').selectOption(id); await page.locator('#load').click();
    await expect(page.locator('#context')).toHaveText(id.replace('-', ' · '));
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
    expected.push({ title, rows: await exportedRows() });
  }
  const player = expected[2].rows[0];
  const before = await page.locator('#context').textContent();
  await context.setOffline(true); const requests = payloadRequests.length;
  await open(player.name);
  const cards = page.locator('#player-history .history-window');
  await expect(cards).toHaveCount(3);
  await expect(cards.locator('h4')).toHaveText([expected[2].title, expected[0].title, expected[1].title]);
  for (let index = 0; index < 3; index++) {
    const entry = [expected[2], expected[0], expected[1]][index];
    const row = entry.rows.find(row => row.player_id === player.player_id);
    const values = await cards.nth(index).locator('dd').allTextContents();
    if (row) assert.deepEqual(values, [String(row.gp), String(row.goals), String(row.assists), String(row.points), row.pace_82 === null ? '—' : row.pace_82.toFixed(1)]);
    else await expect(cards.nth(index)).toContainText('No player record in this loaded package');
  }
  assert.equal(payloadRequests.length, requests, 'Opening history must not request additional packages');
  assert.equal(await page.locator('#context').textContent(), before, 'History must not select another engine window');
  await page.setViewportSize({ width: 390, height: 844 });
  for (const theme of ['night-game', 'fresh-ice', 'hockey-club']) {
    await page.locator('#close-player').click();
    await page.locator('#appearance').selectOption(theme); await open(player.name);
    await page.locator('#history-title').scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(output, `${theme}.png`) });
    assert.ok(await page.locator('#player-dialog').evaluate(element => element.scrollWidth <= element.clientWidth));
  }
  // Simulate a refresh completing while detail is open: old history must close.
  const catalog = JSON.parse(await readFile(resolve(root, 'catalog.json'), 'utf8'));
  const current = catalog.packages.find(entry => entry.id === '20252026-regular');
  const refreshed = JSON.parse(await readFile(resolve(root, current.url), 'utf8'));
  refreshed.source = 'Refreshed history fixture';
  await page.locator('#import').setInputFiles({ name: 'refreshed-history.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(refreshed)) });
  await expect(page.locator('#player-dialog')).not.toBeVisible();
  await expect(page.locator('#data-state')).toContainText('Refreshed history fixture');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  // A fixture with only a different player is missing evidence, not zero-valued stats.
  const older = catalog.packages.find(entry => entry.id === '20222023-regular');
  const missing = JSON.parse(await readFile(resolve(root, older.url), 'utf8'));
  const other = missing.bios.find(row => row.playerId !== player.player_id);
  missing.bios = [other]; missing.stats = missing.stats.filter(row => row.playerId === other.playerId);
  missing.goalies = []; missing.source = 'Missing history fixture';
  await page.locator('#import').setInputFiles({ name: 'missing-history.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(missing)) });
  await expect(page.locator('#context')).toContainText('20222023');
  await library();
  const sessions = page.locator('#resident-library');
  await sessions.locator('..').locator('summary').click();
  await sessions.locator('li').filter({ hasText: '20252026 · regular' }).getByRole('button').click();
  await expect(page.locator('#context')).toContainText('20252026');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  await open(player.name);
  await expect(page.locator('#player-history .history-window')).toHaveCount(4);
  await expect(page.locator('#player-history .history-window').last()).toContainText('does not mean zero games');
  await page.locator('#close-player').click();
  await page.locator('#kind').selectOption('goalies');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const goalies = await exportedRows(); await open(goalies[0].name);
  await expect(page.locator('#player-history .history-window').first().locator('dd')).toHaveText([
    String(goalies[0].gp), String(goalies[0].wins), goalies[0].save_pct === null ? '—' : goalies[0].save_pct.toFixed(3), goalies[0].gaa === null ? '—' : goalies[0].gaa.toFixed(2),
  ]);
  report.verified = true; report.evidence = { loaded_windows: 4, regular_playoff_separation: true, exact_query_values: true, no_package_requests: true, active_window_unchanged: true, stale_dialog_closed: true, missing_not_zero: true, goalie_values: true, narrow_width: 390 };
} catch (error) {
  failure = error; report.error = String(error); await page.screenshot({ path: resolve(output, 'failure.png') }).catch(() => {});
} finally {
  await browser.close(); await new Promise(accept => server.close(accept));
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(output, JSON.stringify(report));
}
if (failure) throw failure;
