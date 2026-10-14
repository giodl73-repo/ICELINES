// Windows-only, opt-in OS process measurement of a retained production artifact.
import { chromium, expect } from '../icelines-browser/node_modules/@playwright/test/index.mjs';
import { createServer } from 'node:http';
import { mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { cpus, totalmem, release } from 'node:os';
import { basename, dirname, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createInterface } from 'node:readline';
import { diagnoseMemory } from '../icelines-browser/e2e/memory-diagnostics.mjs';
import { normalWorkloadPhases } from '../icelines-browser/e2e/memory-scope.mjs';

if (process.platform !== 'win32') throw new Error('This sampler uses Windows process counters; other platforms need their own OS sampler.');
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const option = name => { const index = args.indexOf(name); return index < 0 ? undefined : args[index + 1]; };
const diagnostics = false;
if (!option('--artifact') || !option('--archive')) throw new Error('Provide --artifact <retained distribution> and --archive <public .tar.gz release>. Paths are relative to the repository root.');
const root = resolve(repository, option('--artifact'));
const archive = resolve(repository, option('--archive'));
const output = await mkdtemp(resolve(repository, 'target/dom-counter-diagnostic-'));
const stopFile = resolve(output, 'stop-sampling');
const samples = [];
let phase = 'blank_browser';
let server, browserServer, browser, sampler;
let samplerError = '';
let samplerExit;
let samplerFinished;
let failure;
const report = {
  started_at: new Date().toISOString(), completed: false,
  host: { platform: process.platform, arch: process.arch, os_release: release(), cpu: cpus()[0]?.model, logical_cpus: cpus().length, ram_bytes: totalmem() },
  scope: 'Single isolated headless Chromium on this Windows workstation. Includes owned browser/renderer/GPU/utility processes; excludes Node runner, local HTTP server and sampler. Working-set sums may count shared pages more than once. Private-byte sums measure private allocations, not just resident RAM. Sample maxima may miss transient spikes; lifetime working-set peaks are per-process, not concurrent group peaks. No physical-phone or remote-rollback claim.',
  sampling: { requested_interval_ms: 100, tree_discovery_interval_ms: 500 }, packages: [], imports: [], warm_query_ms: [],
};
const pause = ms => new Promise(accept => setTimeout(accept, ms));
try {
  const info = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
  const shell = JSON.parse(await readFile(resolve(root, 'shell-manifest.json'), 'utf8'));
  const verification = spawnSync('python', [resolve(repository, 'scripts/verify-browser-artifact.py'), root, info.commit, shell.build], { encoding: 'utf8' });
  if (verification.status !== 0) throw new Error(`Artifact verification failed: ${verification.stderr || verification.error}`);
  report.artifact = JSON.parse(verification.stdout);
  const archiveBytes = await readFile(archive);
  const archiveSeason = basename(archive).match(/^data-(\d{8})\.tar\.gz$/)?.[1];
  if (!archiveSeason) throw new Error('Use an official archive filename: data-<eight-digit season>.tar.gz.');
  report.archive = { sha256: createHash('sha256').update(archiveBytes).digest('hex'), bytes: archiveBytes.length };
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
  const url = `http://127.0.0.1:${server.address().port}/ICELINES/workbench/`;
  browserServer = await chromium.launchServer({ channel: 'chromium', headless: true });
  browser = await chromium.connect(browserServer.wsEndpoint());
  const rootProcessId = browserServer.process().pid;
  report.browser = browser.version();
  sampler = spawn('powershell.exe', ['-NoProfile', '-File', resolve(repository, 'icelines-browser/e2e/sample-browser-memory.ps1'), '-RootProcessId', String(rootProcessId), '-StopFile', stopFile], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  samplerFinished = new Promise(accept => {
    sampler.on('error', error => { samplerError += String(error); });
    sampler.on('close', code => { samplerExit = code; accept(); });
  });
  sampler.stderr.on('data', data => { samplerError += data; });
  createInterface({ input: sampler.stdout }).on('line', line => {
    try { samples.push({ phase, ...JSON.parse(line) }); } catch (error) { samplerError += `Invalid sampler output: ${error}`; }
  });
  const context = await browser.newContext({ serviceWorkers: 'allow', viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();
  let viewportRenderer = false;
  const queryCommitted = async expectedRows => {
    if (viewportRenderer) {
      // Validate the complete result, not just the currently mounted slice.
      await expect(page.locator('#rows')).toHaveAttribute('data-result-count', String(expectedRows), { timeout: 60000 });
      await expect(page.locator('#query-summary')).toContainText(`${expectedRows} skaters`);
      // 800 CSS pixels / 72-pixel rows, rounded up, plus 16 overscan rows per side.
      expect(await page.locator('#rows tr[data-result-index]').count()).toBeLessThanOrEqual(44);
    } else await expect(page.locator('#rows tr')).toHaveCount(expectedRows, { timeout: 60000 });
    // Attached rows retain their count while a request is pending. The form
    // stays enabled too, so neither condition alone proves query completion.
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false, { timeout: 60000 });
  };
  report.query_completion = 'Complete result count (and query summary for viewport rendering), then pending tbody.inert cleared before starting the next query.';
  await expect.poll(() => samples.length, { timeout: 15000 }).toBeGreaterThan(3);
  report.dom_checkpoints=[];
  const captureDOM=async label=>{
    const session=await context.newCDPSession(page);
    try{
      const counters=await session.send('Memory.getDOMCounters');
      const live=await page.evaluate(()=>({elements:document.getElementsByTagName('*').length,mounted_players:document.querySelectorAll('#rows tr[data-result-index]').length,result_count:document.querySelector('#rows')?.dataset.resultCount??null,library_items:document.querySelectorAll('#resident-library > li').length}));
      report.dom_checkpoints.push({label,at:new Date().toISOString(),counters,live,latest_os_sample:samples.at(-1)});
      console.log('DOM checkpoint '+label+': '+JSON.stringify({counters,live}));
    }finally{await session.detach();}
  };
  phase = 'startup';
  await page.goto(url);
  await expect(page.locator('#catalog option').first()).toBeAttached({ timeout: 60000 });
  viewportRenderer = await page.locator('#table-view').count() > 0;
  report.renderer = viewportRenderer ? 'Viewport rows; complete engine result retained, at most 44 mounted players at 1280x800.' : 'Full table rows.';
  await expect.poll(() => samples.filter(sample => sample.phase === 'startup').length, { timeout: 15000 }).toBeGreaterThanOrEqual(2);
  await expect(page.locator('#offline-state')).toContainText('Application shell available offline',{timeout:60000});
  await captureDOM('ready_before_engine');
  const catalog = JSON.parse(await readFile(resolve(root, 'catalog.json'), 'utf8'));
  for (const [index, entry] of catalog.packages.entries()) {
    if (samplerExit !== undefined) throw new Error(`Sampler stopped unexpectedly: ${samplerError}`);
    phase = `package_${entry.id}`;
    if (!(await page.locator('#load').isVisible())) await page.locator('#data-library > summary').click();
    await page.locator('#catalog').selectOption(entry.id);
    const before = performance.now();
    await page.locator('#load').click();
    await expect(page.locator('#context')).toContainText(String(entry.season));
    await expect(page.locator('#context')).toContainText(entry.season_type);
    await expect(page.locator('#query-summary')).toContainText('skaters', { timeout: 60000 });
    await queryCommitted(entry.skaters);
    await expect(page.locator('#load')).toBeEnabled();
    const residents = await page.locator('#resident-library > li').count();
    expect(residents).toBeLessThanOrEqual(8);
    report.packages.push({ id: entry.id, bytes: entry.bytes, load_and_render_ms: performance.now() - before, rows: entry.skaters, resident_windows: residents });
    console.log(`Loaded ${index + 1}/${catalog.packages.length}: ${entry.id}, residents ${residents}`);
    await expect.poll(() => samples.filter(sample => sample.phase === phase).length, { timeout: 15000 }).toBeGreaterThanOrEqual(2);
    if([0,7,9,10,25,74].includes(index))await captureDOM('loaded_'+(index+1));
  }
  for (const seasonType of ['regular', 'playoff']) {
    phase = `archive_${seasonType}`;
    if (!(await page.locator('#import').isVisible())) await page.locator('#data-library > summary').click();
    await page.locator('#import-type').selectOption(seasonType);
    const before = performance.now();
    await page.locator('#import').setInputFiles(archive);
    await expect(page.locator('#context')).toContainText(archiveSeason, { timeout: 60000 });
    await expect(page.locator('#context')).toContainText(seasonType, { timeout: 60000 });
    const expectedPackage = catalog.packages.find(entry => String(entry.season) === archiveSeason && entry.season_type === seasonType);
    if (!expectedPackage) throw new Error('Archive context must exist in the verified artifact catalog.');
    await queryCommitted(expectedPackage.skaters);
    await expect(page.locator('#status')).toContainText('Imported into memory', { timeout: 60000 });
    await expect(page.locator('#rows tr').first()).toBeAttached();
    report.imports.push({ season_type: seasonType, import_and_render_ms: performance.now() - before, rows: viewportRenderer ? Number(await page.locator('#rows').getAttribute('data-result-count')) : await page.locator('#rows tr').count() });
    await pause(1000);
  }
  phase = 'warm_query';
  await page.locator('#filter').fill('p>=1 AND gp>=1');
  const expectedRows = viewportRenderer
    // The same fixture expectation as the full-table cell check, independent of
    // the mounted viewport. This does not truncate or alter the engine query.
    ? JSON.parse(await readFile(resolve(root, catalog.packages.find(entry => String(entry.season) === archiveSeason && entry.season_type === 'playoff').url), 'utf8')).stats.filter(row => row.gamesPlayed >= 1 && row.points >= 1).length
    : await page.locator('#rows tr').evaluateAll(rows => rows.filter(row => Number(row.cells[3].textContent) >= 1 && Number(row.cells[6].textContent) >= 1).length);
  if (!expectedRows) throw new Error('Imported playoff fixture needs nonempty warm-query results.');
  for (let index = 0; index < 20; index++) {
    const before = performance.now();
    await page.locator('#query button[type="submit"]').click();
    await queryCommitted(expectedRows);
    await expect(page.locator('#query button[type="submit"]')).toBeEnabled();
    report.warm_query_ms.push(performance.now() - before);
  }
  phase = 'resident_settled';
  await pause(1500);
  if (diagnostics) report.diagnostics = await diagnoseMemory(browser, context, page, new URL('src/worker.js', url).href, name => { phase = name; }, () => pause(1500));
  await captureDOM('after_twenty_queries');
  phase = 'engine_restarted';
  if (!(await page.locator('#restart-engine').isVisible())) await page.locator('#data-library > summary').click();
  await page.locator('#restart-engine').click();
  await expect(page.locator('#status')).toContainText('Session-only data needs reloading', { timeout: 60000 });
  await expect(page.locator('#rows tr')).toHaveCount(0);
  await pause(2000);
  await captureDOM('after_restart');
  await writeFile(stopFile, 'stop');
  await samplerFinished;
  if (samplerExit !== 0 || samplerError) throw new Error(`Memory sampler failed: ${samplerError}`);
  const phases = [...new Set(samples.map(sample => sample.phase))];
  report.memory = phases.map(name => {
    const group = samples.filter(sample => sample.phase === name);
    const sum = (sample, metric) => sample.processes.reduce((value, process) => value + process[metric], 0);
    return { phase: name, samples: group.length, min_processes: Math.min(...group.map(sample => sample.processes.length)), max_processes: Math.max(...group.map(sample => sample.processes.length)), minimum_private_bytes: Math.min(...group.map(sample => sum(sample, 'private_bytes'))), last_private_bytes: sum(group.at(-1), 'private_bytes'), peak_private_bytes: Math.max(...group.map(sample => sum(sample, 'private_bytes'))), peak_working_set_bytes: Math.max(...group.map(sample => sum(sample, 'working_set_bytes'))), peak_sum_lifetime_working_set_bytes: Math.max(...group.map(sample => sum(sample, 'lifetime_peak_working_set_bytes'))) };
  });
  report.missing_sampled_phases = ['blank_browser', 'startup', ...catalog.packages.map(entry => `package_${entry.id}`), 'archive_regular', 'archive_playoff', 'warm_query', 'resident_settled', 'engine_restarted'].filter(name => !phases.includes(name));
  if (report.missing_sampled_phases.length) throw new Error(`Missing measured phases: ${report.missing_sampled_phases.join(', ')}`);
  const intervals = samples.slice(1).map((sample, index) => sample.elapsed_ms - samples[index].elapsed_ms);
  report.sampling.actual_median_interval_ms = intervals.toSorted((a, b) => a - b)[Math.floor(intervals.length / 2)];
  report.sampling.actual_max_interval_ms = Math.max(...intervals);
  const normalPhases = normalWorkloadPhases(report.memory);
  if (!normalPhases.length) throw new Error('No normal workload phases measured.');
  report.target_scope = diagnostics ? 'Pre-intervention samples only; post-GC engine restart is diagnostic, not a normal-workload control.' : 'All normal workload phases including engine restart.';
  report.sampled_peak_private_bytes = Math.max(...normalPhases.map(phase => phase.peak_private_bytes));
  report.sampled_peak_working_set_bytes = Math.max(...normalPhases.map(phase => phase.peak_working_set_bytes));
  const baseline = report.memory.find(phase => phase.phase === 'blank_browser');
  report.sampled_private_bytes_over_blank_baseline = report.sampled_peak_private_bytes - baseline.peak_private_bytes;
  report.provisional_250_mib_target = {
    sampled_private_bytes_below_target: report.sampled_peak_private_bytes <= 250 * 1024 * 1024,
    sampled_sum_working_set_below_target: report.sampled_peak_working_set_bytes <= 250 * 1024 * 1024,
    note: 'Observed process-counter comparison only; not an exact concurrent physical-memory peak or a mobile claim. A failed target requires investigation or an evidence-backed budget revision.',
  };
  report.provisional_250_mib_target={sampled_private_bytes_below_target:null,sampled_sum_working_set_below_target:null,note:'Additional DOM-counter CDP and live-element observations affect timing; diagnostic only, never normal acceptance.'};
  report.target_scope='Full75/eight/two/20/restart with original sampler/flags/channel and no forced GC. Nine additional DOM-counter observations, not a normal benchmark. DOM counters may include collected-later objects and other contexts; live element count covers current document only. No heap snapshot or tracing.';
  if(report.dom_checkpoints.length!==9)throw Error('Incomplete DOM counter observations');
  report.completed = true;
} catch (error) { failure = error; report.error = String(error); }
finally {
  if (sampler && samplerExit === undefined) { await writeFile(stopFile, 'stop'); await samplerFinished; }
  await browser?.close();
  await browserServer?.close();
  if (server?.listening) await new Promise(accept => server.close(accept));
  report.finished_at = new Date().toISOString();
  await writeFile(resolve(output, 'samples.json'), JSON.stringify(samples) + '\n');
  await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(`Memory report: ${resolve(output, 'report.json')}`);
  console.log(`Completed: ${report.completed}; sampled private bytes: ${report.sampled_peak_private_bytes}; working set: ${report.sampled_peak_working_set_bytes}`);
}
if (failure) throw failure;
