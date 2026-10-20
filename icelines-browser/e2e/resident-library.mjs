// Owned-browser selection/eviction/ownership probe, separate from OS measurement.
import { chromium, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdtemp, stat } from 'node:fs/promises';
import { resolve, dirname, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const index = process.argv.indexOf('--artifact');
if (index < 0 || !process.argv[index + 1]) throw new Error('Provide --artifact <retained distribution>.');
const root = resolve(repository, process.argv[index + 1]);
const info = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
const shell = JSON.parse(await readFile(resolve(root, 'shell-manifest.json'), 'utf8'));
const check = spawnSync('python', [resolve(repository, 'scripts/verify-browser-artifact.py'), root, info.commit, shell.build], { encoding: 'utf8' });
if (check.status !== 0) throw new Error(check.stderr || String(check.error));
const entries = JSON.parse(await readFile(resolve(root, 'catalog.json'), 'utf8')).packages.filter(entry => entry.season_type === 'regular' && entry.skaters > 0).slice(0, 9);
assert.equal(entries.length, 9);
const out = await mkdtemp(resolve(repository, 'target/resident-library-'));
const report = { verified: false, artifact: JSON.parse(check.stdout), scope: 'Owned desktop browser functional/DOM probe; held old buttons are an artificial observer, not a memory or leak measurement.' };
const server = createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const prefix = '/ICELINES/workbench/';
    if (!pathname.startsWith(prefix)) throw new Error('outside workbench');
    const file = resolve(root, pathname.slice(prefix.length) || 'index.html');
    if (!file.startsWith(root + sep) || !(await stat(file)).isFile()) throw new Error('invalid file');
    response.writeHead(200, { 'Content-Type': ({ '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' })[extname(file)] || 'application/octet-stream' }).end(await readFile(file));
  } catch { response.writeHead(404).end(); }
});
await new Promise(accept => server.listen(0, '127.0.0.1', accept));
const browser = await chromium.launch({ channel: 'chromium', headless: true });
let failure;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, serviceWorkers: 'block' });
  await page.goto(`http://127.0.0.1:${server.address().port}/ICELINES/workbench/`);
  await expect(page.locator('#catalog option').first()).toBeAttached();
  const openLibrary = async () => { if (!await page.locator('#load').isVisible()) await page.locator('#data-library > summary').click(); };
  const committed = async entry => {
    await expect(page.locator('#context')).toContainText(String(entry.season));
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count', String(entry.skaters));
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
  };
  const load = async entry => { await openLibrary(); await page.locator('#catalog').selectOption(entry.id); await page.locator('#load').click(); await committed(entry); await expect(page.locator('#load')).toBeEnabled(); };
  await load(entries[0]); await load(entries[1]);
  const firstButton = await page.locator('#resident-library li button').first().elementHandle();
  await page.evaluate(() => {
    const list = document.getElementById('resident-library');
    window.__residentRows = [...list.children];
    window.__residentMutations = { added: 0, removed: 0 };
    new MutationObserver(records => {
      for (const record of records) if (record.target === list) {
        window.__residentMutations.added += [...record.addedNodes].filter(node => node.nodeName === 'LI').length;
        window.__residentMutations.removed += [...record.removedNodes].filter(node => node.nodeName === 'LI').length;
      }
    }).observe(list, { childList: true });
  });
  for (let step = 0; step < 10; step++) { await page.locator('#query button[type="submit"]').click(); await committed(entries[1]); }
  const repeated = await page.evaluate(() => ({ ...window.__residentMutations, sameRows: window.__residentRows.every((row, i) => document.getElementById('resident-library').children[i] === row) }));
  await load(entries[1]);
  const refreshedSameRows = await page.evaluate(() => window.__residentRows.every((row, i) => document.getElementById('resident-library').children[i] === row));
  if (process.argv.includes('--require-reuse')) { assert.deepEqual(repeated, { added: 0, removed: 0, sameRows: true }); assert.equal(refreshedSameRows, true); }
  for (const entry of entries.slice(2)) await load(entry);
  await expect(page.locator('#resident-library > li')).toHaveCount(8);
  assert.deepEqual(await page.locator('#resident-library li span').allTextContents(), entries.slice(1).map((entry, i) => `${entry.season} · ${entry.season_type} · ${entry.source}${i === 7 ? ' · selected' : ''}`));
  const oldButton = await firstButton.evaluate(button => ({ connected: button.isConnected, callback: typeof button.onclick }));
  assert.equal(oldButton.connected, false);
  if (process.argv.includes('--require-reuse')) assert.equal(oldButton.callback, 'object'); // null, no per-dataset callback
  await openLibrary(); await page.locator('#resident-library li').first().locator('button').click(); await committed(entries[1]);
  await openLibrary();
  const [download] = await Promise.all([page.waitForEvent('download'), page.locator('#export-package').click()]);
  const exported = await readFile(await download.path());
  assert.equal(createHash('sha256').update(exported).digest('hex'), entries[1].sha256);
  let remaining = 8;
  while (remaining) {
    await openLibrary();
    if (await page.locator('#unload').isDisabled()) {
      const season = Number((await page.locator('#resident-library li span').first().textContent()).slice(0, 8));
      const entry = entries.find(item => item.season === season);
      await page.locator('#resident-library li').first().locator('button').click(); await committed(entry); await openLibrary();
    }
    await page.locator('#unload').click(); await expect(page.locator('#resident-library > li')).toHaveCount(--remaining);
  }
  await expect(page.locator('#resident-library')).toHaveText('No seasons in this tab yet.');
  await load(entries[0]);
  report.evidence = { repeated_queries: 10, repeated, refreshedSameRows, distinct_loads: 9, residents_after_eviction: 8, exact_order: true, oldButton, exact_selected_source_export: true, unload_to_empty: true, reload_after_empty: true };
  report.verified = true;
} catch (error) { failure = error; report.error = String(error); }
finally { await browser.close(); await new Promise(accept => server.close(accept)); await writeFile(resolve(out, 'report.json'), JSON.stringify(report, null, 2) + '\n'); }
console.log(`${out} ${JSON.stringify(report)}`);
if (failure) throw failure;
