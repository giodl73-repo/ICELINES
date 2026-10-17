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
const out = await mkdtemp(resolve(repository, 'target/persisted-catalog-context-'));
const report = { verified: false, artifact: JSON.parse(verification.stdout), scope: 'Owned persisted-context fixture with unsigned two-entry catalog and blocked service workers. Not memory/offline/production/mobile acceptance.' };
const published=JSON.parse(await readFile(resolve(root,'catalog.json'),'utf8'));
let denySecond=false;let deniedPayloads=0;
let currentCatalog={...published,packages:published.packages.slice(0,2)};
const server = createServer(async (req, res) => { try {
    const p = decodeURIComponent(new URL(req.url, 'http://localhost').pathname).replace('/ICELINES/workbench/', '');
    if(denySecond && [published.packages[1].url,published.packages[1].transport?.url].includes(p)){deniedPayloads++;res.writeHead(503).end('Persisted fixture forbids network payload');return;}
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

const profile=await mkdtemp(resolve(repository,'target/persisted-context-profile-'));
let context;let failure;
const url=`http://127.0.0.1:${server.address().port}/ICELINES/workbench/`;
const [first,second]=published.packages;
try {
  assert.ok(first.skaters>0 && second.skaters>0 && first.season_type==='regular' && second.season_type==='regular');
  const launch=async()=>{
    context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,serviceWorkers:'block',viewport:{width:1280,height:800}});
    await context.addInitScript(()=>{
      globalThis.__datasetReads=[];
      const original=IDBObjectStore.prototype.get;
      IDBObjectStore.prototype.get=function(key){if(this.name==='datasets')globalThis.__datasetReads.push(String(key));return original.call(this,key);};
    });
    const page=context.pages()[0]??await context.newPage();
    // Explicit public first-season route avoids startup restoring the saved
    // second season into resident memory before the rejection test.
    await page.goto(url+`#/leaders?season=${first.season}&type=regular&kind=skaters&sort=points&data=public`);
    await expect(page.locator('#catalog option')).toHaveCount(2);
    await expect(page.locator('#saved-count')).toHaveText(/\(0\)|\(1\)/);
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(first.skaters));
    await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
    await expect(page.locator('#resident-library li')).toHaveCount(1);
    await expect(page.locator('#resident-library')).not.toContainText(String(second.season));
    await expect(page.locator('#load')).toBeEnabled();
    return page;
  };
  let page=await launch();
  const open=async()=>{await expect(page.locator('#load')).toBeEnabled();await page.locator('#data-library').evaluate(node=>{node.open=true;});await expect(page.locator('#catalog')).toBeVisible();};
  const load=async entry=>{await open();await page.locator('#catalog').selectOption(entry.id);await page.locator('#load').click();};
  const committed=async entry=>{await expect(page.locator('#rows')).toHaveAttribute('data-result-count',String(entry.skaters));await expect(page.locator('#rows')).toHaveJSProperty('inert',false);};
  await load(second);await committed(second);await open();await page.locator('#save').click();
  await expect(page.locator('#saved-count')).toHaveText('(1)');await expect(page.locator('#data-state')).toContainText('Saved locally');
  await context.close();context=undefined;denySecond=true;
  const outcomes=[];
  page=await launch();await expect(page.locator('#saved-count')).toHaveText('(1)');
  const reloadCatalog=async()=>{await page.evaluate(()=>document.getElementById('retry-catalog').click());await expect(page.locator('#retry-catalog')).toBeEnabled();};
  for(const change of [{season:20262027},{season_type:'playoff'},{source:'wrong catalog source'}]){
    currentCatalog={...published,packages:[first,{...second,...change}]};
    await reloadCatalog();await expect(page.locator('#resident-library li')).toHaveCount(1);
    await expect(page.locator('#resident-library')).not.toContainText(String(second.season));
    await committed(first);
    const baseline=await page.locator('#context').textContent();
    await page.evaluate(()=>{globalThis.__datasetReads=[];});
    await load(second);await expect(page.locator('#status')).toContainText(/catalog context/i);
    assert.equal(await page.locator('#context').textContent(),baseline);await committed(first);
    const reads=await page.evaluate(()=>globalThis.__datasetReads);
    assert.ok(reads.includes(second.id),'Rejected candidate must actually be read from IndexedDB');
    const [download]=await Promise.all([page.waitForEvent('download'),page.locator('#export-package').click()]);
    assert.deepEqual(await readFile(await download.path()),await readFile(resolve(root,first.url)));
    await page.locator('#query button[type="submit"]').click();await committed(first);
    outcomes.push({change,process_reopened:true,indexeddb_candidate_read:true,previous_rows:first.skaters,exact_previous_export:true,requery:true});
  }
  currentCatalog={...published,packages:[first,second]};await reloadCatalog();
  await page.evaluate(()=>{globalThis.__datasetReads=[];});await load(second);await committed(second);
  await expect(page.locator('#status')).toContainText('Loaded your saved public season');
  assert.ok((await page.evaluate(()=>globalThis.__datasetReads)).includes(second.id));
  assert.equal(deniedPayloads,0,'Saved reads must not attempt forbidden network payload');
  report.verified=true;report.scope='Retained release bytes, owned process-reopened profile and real IndexedDB, service workers blocked, unsigned two-entry catalog. Three saved-context refusals preserve previous full result/exact export/requery; correct saved retry succeeds without payload network requests. Not offline-shell, memory, production or physical-device acceptance.';
  report.evidence={process_reopens:1,outcomes,retry_rows:second.skaters,retry_saved:true,forbidden_payload_attempts:deniedPayloads};
}catch(error){failure=error;report.error=String(error);}
finally{await context?.close();await new Promise(r=>server.close(r));await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');console.log(out,JSON.stringify(report));}
if(failure)throw failure;
