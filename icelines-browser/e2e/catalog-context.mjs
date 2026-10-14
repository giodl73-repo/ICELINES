// Opt-in real-browser regression for public catalog context validation; no live NHL reads.
import { chromium, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdtemp, stat } from 'node:fs/promises';
import { resolve, dirname, extname, sep } from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const index = process.argv.indexOf('--artifact');
if (index < 0 || !process.argv[index + 1])
    throw new Error('Provide --artifact <retained distribution>. This test requires the first two catalog packages to be nonempty regular seasons.');
const root = resolve(repository, process.argv[index + 1]);
const info = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
const shell = JSON.parse(await readFile(resolve(root, 'shell-manifest.json'), 'utf8'));
const verification = spawnSync('python', [resolve(repository, 'scripts/verify-browser-artifact.py'), root, info.commit, shell.build], { encoding: 'utf8' });
if (verification.status !== 0)
    throw new Error(verification.stderr || String(verification.error));
const out = await mkdtemp(resolve(repository, 'target/catalog-context-'));
const report = { verified: false, artifact: JSON.parse(verification.stdout), scope: 'Owned browser catalog-context fixture; unsigned two-entry catalog, handler-driven reload, fresh/cached rejection and previous-good export. Not memory/offline/mobile acceptance.' };
const published=JSON.parse(await readFile(resolve(root,'catalog.json'),'utf8'));
let currentCatalog={...published,packages:published.packages.slice(0,2)};
const server = createServer(async (req, res) => { try {
    const p = decodeURIComponent(new URL(req.url, 'http://localhost').pathname).replace('/ICELINES/workbench/', '');
    if(p==='catalog.json'){res.writeHead(200,{'Content-Type':'application/json','Cache-Control':'no-store'}).end(JSON.stringify(currentCatalog));return;}
    const f = resolve(root, p || 'index.html');
    if (!f.startsWith(root + sep) || !(await stat(f)).isFile())
        throw Error();
    res.writeHead(200, { 'Content-Type': ({ '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' })[extname(f)] || 'application/octet-stream' }).end(await readFile(f));
}
catch {
    res.writeHead(404).end();
} });
await new Promise(r => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({ channel: 'chromium', headless: true });
let failure;
let currentPage;
try {
  const page=currentPage=await browser.newPage({viewport:{width:1280,height:800}});
  await page.goto(`http://127.0.0.1:${server.address().port}/ICELINES/workbench/`);
  await expect(page.locator('#catalog option')).toHaveCount(2);
  const [first,second]=published.packages;
  assert.ok(first?.skaters>0 && second?.skaters>0 && first.season_type==='regular' && second.season_type==='regular', 'Requires two nonempty regular-season fixtures');
  const open=async()=>{if(!await page.locator('#load').isVisible())await page.locator('#data-library > summary').click();};
  const load=async entry=>{await open();await page.locator('#catalog').selectOption(entry.id);await page.locator('#load').click();};
  await load(first);await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(first.skaters));await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
  const baseline=await page.locator('#context').textContent();const outcomes=[];
  const reload=async()=>{await open();await page.evaluate(()=>document.getElementById('retry-catalog').click());await expect(page.locator('#catalog-status')).toContainText('2 season packages');await expect(page.locator('#retry-catalog')).toBeEnabled();};
  for(const cached of [false,true]) {
    if(cached){currentCatalog={...published,packages:[first,second]};await reload();await load(second);await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(second.skaters));await expect(page.locator('#rows')).toHaveJSProperty('inert',false);await load(first);await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(first.skaters));await expect(page.locator('#rows')).toHaveJSProperty('inert',false);}
    for(const change of [{season:20262027},{season_type:'playoff'},{source:'wrong catalog source'}]) {
      currentCatalog={...published,packages:[first,{...second,...change}]};await reload();await load(second);
      await expect(page.locator('#status')).toContainText(/catalog context/i);
      assert.equal(await page.locator('#context').textContent(),baseline);
      await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(first.skaters));
      const [download]=await Promise.all([page.waitForEvent('download'),page.locator('#export-package').click()]);
      assert.deepEqual(await readFile(await download.path()),await readFile(resolve(root,first.url)));
      await page.locator('#query button[type="submit"]').click();await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
      outcomes.push({cached,change,previous_rows:first.skaters,exact_previous_export:true,requery:true});
    }
  }
  currentCatalog={...published,packages:[first,second]};await reload();await load(second);await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(second.skaters));await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
  report.verified=true;report.scope='Owned browser with intentionally unsigned two-entry catalog and unavailable offline-shell cache; fixture invokes hidden reload handler after swapping catalog; catalog mismatch fresh/cached rejection, previous-good full result/exact package export/requery and retry. Not offline, memory, production or phone proof.';report.evidence={outcomes,retry_rows:second.skaters};
}catch(error){failure=error;report.error=String(error);}
finally{await browser.close();await new Promise(r=>server.close(r));await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');console.log(out,JSON.stringify(report));}
if(failure)throw failure;
