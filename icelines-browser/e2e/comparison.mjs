// Local-only development smoke: actual Rust rows, two-player UI and offline operation.
import { chromium, expect } from '@playwright/test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdtemp, stat } from 'node:fs/promises';
import { resolve, dirname, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const root = resolve(repository, 'icelines-browser/dist');
const output = await mkdtemp(resolve(repository, 'target/comparison-browser-'));
const report = { verified: false, scope: 'Fresh isolated local Chromium; comparison of actual Rust query projections, offline interaction and context clearing. Not physical Android, memory or full native CompareView parity.' };
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
let failure;
try {
  report.build = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await expect(page.locator('#catalog option').first()).toBeAttached();
  await page.locator('#data-library > summary').click();
  await page.locator('#catalog').selectOption('20252026-regular');
  await page.locator('#load').click();
  await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '933');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const exported = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);
  const rows = JSON.parse(await readFile(await exported[0].path(), 'utf8')).rows;
  const pick = async name => {
    await page.locator('#rows button').filter({ hasText: name }).first().click();
    await expect(page.locator('#player-dialog')).toBeVisible();
    await page.locator('#compare-player').click();
    await expect(page.locator('#player-dialog')).not.toBeVisible();
  };
  await context.setOffline(true);
  await pick(rows[0].name); await pick(rows[1].name);
  await expect(page.locator('.comparison-table thead th')).toHaveCount(3);
  await expect(page.locator('#comparison-context')).toContainText('20252026 · regular · Skaters');
  const values = await page.locator('.comparison-table tbody tr').evaluateAll(rows => rows.map(row => Array.from(row.cells, cell => cell.textContent)));
  assert.deepEqual(values, [
    ['Games played', ...rows.slice(0, 2).map(row => String(row.gp))],
    ['Goals', ...rows.slice(0, 2).map(row => String(row.goals))],
    ['Assists', ...rows.slice(0, 2).map(row => String(row.assists))],
    ['Points', ...rows.slice(0, 2).map(row => String(row.points))],
    ['Pace per 82', ...rows.slice(0, 2).map(row => row.pace_82 === null ? '—' : row.pace_82.toFixed(1))],
  ]);
  await page.locator('#rows button').filter({ hasText: rows[2].name }).first().click();
  await expect(page.locator('#compare-player')).toBeDisabled();
  await page.locator('#close-player').click();
  await page.locator('#filter').fill('p>=1'); await page.locator('#query button[type=submit]').click();
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  await expect(page.locator('.comparison-table thead th')).toHaveCount(3);
  await page.setViewportSize({ width: 390, height: 844 });
  for (const theme of ['night-game', 'fresh-ice', 'hockey-club']) {
    await page.locator('#appearance').selectOption(theme);
    await page.locator('#comparison-title').focus();
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.ok(await page.locator('#player-comparison .table-scroll').evaluate(element => element.scrollWidth <= element.clientWidth), 'Both player columns fit the narrow comparison');
    await page.screenshot({ path: resolve(output, `${theme}.png`) });
  }
  await page.getByRole('button', { name: `Remove ${rows[0].name} from comparison`, exact: true }).click();
  await expect(page.locator('.comparison-table thead th')).toHaveCount(2);
  await page.getByRole('button', { name: 'Clear comparison', exact: true }).click();
  await expect(page.locator('#player-comparison')).toBeHidden();
  await pick(rows[0].name);
  await page.locator('#kind').selectOption('goalies');
  await expect(page.locator('#player-comparison')).toBeHidden();
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const goalieNames = await page.locator('#rows td:nth-child(2) button').allTextContents();
  await pick(goalieNames[0]); await pick(goalieNames[1]);
  await expect(page.locator('#comparison-context')).toContainText('Goalies');
  await expect(page.locator('.comparison-table tbody th')).toHaveText(['Games played', 'Wins', 'Save percentage', 'Goals against average']);
  const goalieDownload = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);
  const goalieRows = JSON.parse(await readFile(await goalieDownload[0].path(), 'utf8')).rows.slice(0, 2);
  const goalieValues = await page.locator('.comparison-table tbody tr').evaluateAll(rows => rows.map(row => Array.from(row.cells, cell => cell.textContent)));
  assert.deepEqual(goalieValues, [
    ['Games played', ...goalieRows.map(row => String(row.gp))],
    ['Wins', ...goalieRows.map(row => row.wins === null ? '—' : String(row.wins))],
    ['Save percentage', ...goalieRows.map(row => row.save_pct === null ? '—' : row.save_pct.toFixed(3))],
    ['Goals against average', ...goalieRows.map(row => row.gaa === null ? '—' : row.gaa.toFixed(2))],
  ]);
  const catalog = JSON.parse(await readFile(resolve(root, 'catalog.json'), 'utf8'));
  const entry = catalog.packages.find(entry => entry.id === '20252026-regular');
  const replacement = JSON.parse(await readFile(resolve(root, entry.url), 'utf8'));
  replacement.source = 'Comparison revision fixture';
  await page.locator('#import').setInputFiles({ name: 'comparison-revision.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(replacement)) });
  await expect(page.locator('#data-state')).toContainText('Comparison revision fixture');
  await expect(page.locator('#player-comparison')).toBeHidden();
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  await pick(goalieRows[0].name); await pick(goalieRows[1].name);
  await context.setOffline(false);
  await page.locator('#data-library > summary').click();
  await page.locator('#catalog').selectOption('20242025-regular'); await page.locator('#load').click();
  await expect(page.locator('#context')).toContainText('20242025');
  await expect(page.locator('#player-comparison')).toBeHidden();
  report.verified = true;
  report.evidence = { exact_engine_values: true, offline_selection: true, two_player_limit: true,
    filter_retains_picks: true, removal_and_clear: true, exact_goalie_values: true, revision_dataset_and_kind_clear: true,
    narrow_themes: ['night-game', 'fresh-ice', 'hockey-club'] };
} catch (error) {
  failure = error; report.error = String(error);
  await page.screenshot({ path: resolve(output, 'failure.png') }).catch(() => {});
} finally {
  await browser.close(); await new Promise(accept => server.close(accept));
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(output, JSON.stringify(report));
}
if (failure) throw failure;
