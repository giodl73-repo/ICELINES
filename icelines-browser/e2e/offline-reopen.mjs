// Opt-in acceptance check: public static assets only, never live NHL adapters.
import { chromium, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const args = process.argv.slice(2);
const option = name => { const index = args.indexOf(name); return index < 0 ? undefined : args[index + 1]; };
const artifact = option('--artifact');
const remote = option('--url');
if (Boolean(artifact) === Boolean(remote)) throw new Error('Provide exactly one of --artifact <retained distribution> or --url <HTTPS workbench URL>.');
const output = await mkdtemp(resolve(repository, 'target/browser-acceptance-'));
const profile = await mkdtemp(resolve(tmpdir(), 'icelines-acceptance-'));
const report = { started_at: new Date().toISOString(), verified: false, scope: 'Isolated Chromium process close/reopen with offline network control; saved versus unsaved seasons, fresh query and player detail. Not physical-phone, total-memory or remote-rollback proof.', evidence: {}, profile_retained: profile };
let server;
let context;
let failure;
try {
  let url;
  if (artifact) {
    const root = resolve(repository, artifact);
    const info = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
    const shell = JSON.parse(await readFile(resolve(root, 'shell-manifest.json'), 'utf8'));
    const verification = spawnSync('python', [resolve(repository, 'scripts/verify-browser-artifact.py'), root, info.commit, shell.build], { encoding: 'utf8' });
    if (verification.status !== 0) throw new Error(`Retained artifact verification failed: ${verification.stderr || verification.error}`);
    report.retained_artifact = JSON.parse(verification.stdout);
    const mime = { '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' };
    server = createServer(async (request, response) => {
      try {
        const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
        const prefix = '/ICELINES/workbench/';
        if (!pathname.startsWith(prefix)) { response.writeHead(404).end(); return; }
        const file = resolve(root, pathname.slice(prefix.length) || 'index.html');
        if (!file.startsWith(root + sep) || !(await stat(file)).isFile()) { response.writeHead(404).end(); return; }
        response.writeHead(200, { 'Content-Type': mime[extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' });
        response.end(await readFile(file));
      } catch { response.writeHead(404).end(); }
    });
    await new Promise((accept, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', accept); });
    url = new URL(`http://127.0.0.1:${server.address().port}/ICELINES/workbench/`);
  } else {
    url = new URL(remote);
    if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash || !url.pathname.endsWith('/')) throw new Error('Remote URL must be HTTPS with a trailing slash and no credentials, query or fragment.');
  }
  report.url = url.href;
  context = await chromium.launchPersistentContext(profile, { channel: 'chromium', headless: true, serviceWorkers: 'allow' });
  report.browser = context.browser().version();
  const page = context.pages()[0] || await context.newPage();
  await page.goto(url.href);
  const identityResponse = await page.request.get(new URL('build-info.json', url).href);
  if (!identityResponse.ok()) throw new Error('Published build identity unavailable.');
  report.build = await identityResponse.json();
  await expect(page.locator('#catalog option').first()).toBeAttached({ timeout: 60000 });
  await page.locator('#data-library > summary').click();
  const packages = await page.locator('#catalog option').evaluateAll(options => options.map(option => ({ id: option.value, text: option.textContent })));
  const seasons = packages.filter(entry => entry.id.endsWith('-regular') && !entry.text.includes('· 0 skaters'));
  if (seasons.length < 2) throw new Error('Acceptance needs two nonempty regular-season packages.');
  report.saved_package = seasons[0].id;
  report.unsaved_package = seasons[1].id;
  await page.locator('#catalog').selectOption(seasons[0].id);
  await page.locator('#load').click();
  await expect(page.locator('#rows tr').first()).toBeVisible({ timeout: 60000 });
  const baseline = await page.locator('#rows tr').evaluateAll(rows => rows.map(row => ({ name: row.cells[1].textContent, games: Number(row.cells[3].textContent), points: Number(row.cells[6].textContent) })));
  report.evidence.saved_unfiltered_rows = baseline.length;
  // Loading returns focus to analysis and folds the data library.
  if (!(await page.locator('#save').isVisible())) await page.locator('#data-library > summary').click();
  await page.locator('#save').click();
  await expect(page.locator('#data-state')).toContainText('Saved locally');
  await expect(page.locator('#saved-count')).toHaveText('(1)');
  await expect(page.locator('#offline-state')).toContainText('Application shell available offline', { timeout: 60000 });
  await page.locator('#catalog').selectOption(seasons[1].id);
  await page.locator('#load').click();
  await expect(page.locator('#context')).toContainText(String(seasons[1].id.split('-')[0]));
  await expect(page.locator('#data-state')).toContainText('In memory');
  await expect(page.locator('#saved-count')).toHaveText('(1)');
  await context.close();
  context = undefined;
  if (server) await new Promise(accept => server.close(accept));
  report.evidence.browser_process_closed = true;
  report.evidence.local_server_stopped = Boolean(server);
  context = await chromium.launchPersistentContext(profile, { channel: 'chromium', headless: true, offline: true, serviceWorkers: 'allow' });
  const reopened = context.pages()[0] || await context.newPage();
  const failedRequests = [];
  reopened.on('requestfailed', request => failedRequests.push({ url: request.url(), error: request.failure()?.errorText }));
  await reopened.goto(url.href, { waitUntil: 'domcontentloaded' });
  await expect(reopened.locator('#data-state')).toContainText('Saved locally', { timeout: 60000 });
  await expect(reopened.locator('#context')).toContainText(String(seasons[0].id.split('-')[0]));
  await expect(reopened.locator('#saved-count')).toHaveText('(1)');
  await expect(reopened.locator('#rows tr')).toHaveCount(baseline.length);
  report.evidence.offline = await reopened.evaluate(async () => ({ online: navigator.onLine, probe_failed: await fetch(`./uncached-acceptance-probe-${Date.now()}`, { cache: 'no-store' }).then(() => false, () => true) }));
  expect(report.evidence.offline).toEqual({ online: false, probe_failed: true });
  await reopened.locator('#filter').fill('p>=1 AND gp>=1');
  await reopened.locator('#query button[type="submit"]').click();
  const expected = baseline.filter(row => row.points >= 1 && row.games >= 1).map(row => row.name);
  if (!expected.length || expected.length === baseline.length) throw new Error('Fixture must distinguish the fresh query from the restored initial view.');
  await expect(reopened.locator('#rows tr')).toHaveCount(expected.length);
  expect(await reopened.locator('#rows tr td:nth-child(2)').allTextContents()).toEqual(expected);
  report.evidence.fresh_filtered_rows = expected.length;
  await reopened.locator('#rows button').first().click();
  await expect(reopened.locator('#player-dialog')).toBeVisible();
  await expect(reopened.locator('#player-name')).toHaveText(expected[0]);
  report.evidence.player_detail = expected[0];
  report.evidence.failed_requests = failedRequests;
  await reopened.screenshot({ path: resolve(output, 'offline-player.png'), fullPage: true });
  report.verified = true;
} catch (error) {
  failure = error;
  report.error = String(error);
  const page = context?.pages()[0];
  if (page) await page.screenshot({ path: resolve(output, 'failure.png'), fullPage: true }).catch(() => {});
} finally {
  await context?.close();
  if (server?.listening) await new Promise(accept => server.close(accept));
  report.finished_at = new Date().toISOString();
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(`Acceptance report: ${resolve(output, 'report.json')}`);
  console.log(`Offline reopening verified: ${report.verified}`);
}
if (failure) throw failure;
