// Local-only team navigation acceptance against actual Rust queries and raw package stints.
import { chromium, expect } from '@playwright/test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdtemp, stat } from 'node:fs/promises';
import { resolve, dirname, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const root = resolve(repository, 'icelines-browser/dist');
const output = await mkdtemp(resolve(repository, 'target/team-focus-browser-'));
const report = { verified: false, scope: 'Local Chromium team navigation using actual Rust season-stint filters. Not current rosters, team-only totals, physical Android or memory acceptance.' };
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
  const catalog = JSON.parse(await readFile(resolve(root, 'catalog.json'), 'utf8'));
  const entry = catalog.packages.find(value => value.id === '20252026-regular');
  const raw = JSON.parse(await readFile(resolve(root, entry.url), 'utf8'));
  const traded = raw.stats.find(row => row.teamAbbrevs?.includes(','));
  assert.ok(traded, 'Fixture contains a traded player');
  const team = traded.teamAbbrevs.split(',')[0].trim();
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await expect(page.locator('#catalog option').first()).toBeAttached();
  await expect(page.locator('#team-code')).toBeDisabled();
  await page.locator('#data-library > summary').click();
  await page.locator('#catalog').selectOption(entry.id); await page.locator('#load').click();
  await expect(page.locator('#rows')).toHaveAttribute('data-result-count', String(entry.skaters));
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const exported = async () => {
    const [file] = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);
    return JSON.parse(await readFile(await file.path(), 'utf8'));
  };
  const baseline = await exported();
  await context.setOffline(true);
  await page.locator('#filter').fill('p>=1000'); await page.locator('#minimum-games').fill('1000');
  await page.locator('#team-focus > summary').click(); await page.locator('#team-code').fill(team.toLowerCase());
  await page.locator('#team-query button').click();
  await expect(page.locator('#filter')).toHaveValue(`team.any=${team}`);
  await expect(page.locator('#minimum-games')).toHaveValue('0');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const focused = await exported();
  const ids = new Set(raw.stats.filter(row => row.teamAbbrevs.split(',').map(value => value.trim()).includes(team)).map(row => row.playerId));
  assert.deepEqual(focused.rows, baseline.rows.filter(row => ids.has(row.player_id)), 'Exact ordered full-season values, selected independently from raw package team stints');
  assert.ok(focused.rows.some(row => row.player_id === traded.playerId));
  assert.equal(focused.query.filter, `team.any=${team}`); assert.equal(focused.query.requested_minimum_games, 0);
  assert.equal(new URL(page.url()).hash.includes('team.any'), false, 'Bookmarks omit filter text by default');
  await page.getByText('Share this view', { exact: true }).click();
  await page.locator('#share-filter').check(); await page.locator('#create-link').click();
  const share = new URL(await page.locator('#shared-link').inputValue());
  assert.equal(new URLSearchParams(share.hash.split('?')[1]).get('filter'), `team.any=${team}`);
  await page.locator('#team-code').fill('MTL/SEA');
  await page.locator('#team-query button').click();
  assert.equal(await page.locator('#team-code').evaluate(input => input.validity.patternMismatch), true);
  await expect(page.locator('#filter')).toHaveValue(`team.any=${team}`);
  await page.locator('#rows button').filter({ hasText: focused.rows.find(row => row.player_id === traded.playerId).name }).first().click();
  const otherTeam = traded.teamAbbrevs.split(',')[1].trim();
  await expect(page.locator('#player-dialog')).toBeVisible();
  await page.getByRole('button', { name: `Explore ${otherTeam} season players`, exact: true }).click();
  await expect(page.locator('#player-dialog')).not.toBeVisible();
  await expect(page.locator('#filter')).toHaveValue(`team.any=${otherTeam}`);
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const other = await exported();
  const otherIds = new Set(raw.stats.filter(row => row.teamAbbrevs.split(',').map(value => value.trim()).includes(otherTeam)).map(row => row.playerId));
  assert.deepEqual(other.rows, baseline.rows.filter(row => otherIds.has(row.player_id)));
  await page.locator('#kind').selectOption('goalies');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  const goalie = await exported();
  const goalieIds = raw.goalies.filter(row => row.teamAbbrevs.split(',').map(value => value.trim()).includes(otherTeam)).map(row => row.playerId).sort((a,b) => a-b);
  assert.deepEqual(goalie.rows.map(row => row.player_id).sort((a,b) => a-b), goalieIds, 'Goalies include sub-default GP via explicit zero floor');
  for (const row of goalie.rows) {
    const source = raw.goalies.find(value => value.playerId === row.player_id);
    assert.equal(row.gp, source.gamesPlayed); assert.equal(row.wins, source.wins);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  for (const theme of ['night-game', 'fresh-ice', 'hockey-club']) {
    await page.locator('#appearance').selectOption(theme); await page.locator('#team-code').focus();
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.screenshot({ path: resolve(output, `${theme}.png`) });
  }
  await page.locator('#team-code').fill('ZZZZZZZZ'); await page.locator('#team-query button').click();
  await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '0');
  await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  assert.equal((await exported()).rows.length, 0, 'Unknown valid codes produce an empty result');
  report.verified = true;
  report.evidence = { offline: true, traded_player_membership: true, full_season_values: true, goalie_membership_and_floor: true,
    filter_replacement: true, exports: true, sharing_opt_in: true, player_team_shortcuts: true, unknown_team_empty: true, narrow_width: 390 };
} catch (error) {
  failure = error; report.error = String(error);
  await page.screenshot({ path: resolve(output, 'failure.png') }).catch(() => {});
} finally {
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  await browser.close(); await new Promise(accept => server.close(accept));
}
console.log(JSON.stringify({ output, ...report }, null, 2));
if (failure) throw failure;
