import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { chromium, expect } from '@playwright/test';

// Exercise the ordinary player dialog through its worker and real WASM binding.
// Overrides allow a local debug binding without modifying publication assets.
const repo=fileURLToPath(new URL('../../',import.meta.url));
const root=resolve(process.env.ICELINES_BROWSER_DIST??resolve(repo,'icelines-browser/dist'));
const pkg=process.env.ICELINES_WASM_PKG?resolve(process.env.ICELINES_WASM_PKG):undefined;
const source=resolve(process.env.ICELINES_STAT_CATALOG??resolve(root,'rinkcraft-stat-attributes.json'));
const catalog=JSON.parse(await readFile(source,'utf8'));
const output=resolve(repo,'target/attribute-runtime');await mkdir(output,{recursive:true});
const server=createServer(async(req,res)=>{
  try {
    const path=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    const base=pkg&&path.startsWith('/pkg/')?pkg:root;
    const local=resolve(base,'.'+(base===pkg?path.slice(4):path==='/'?'/index.html':path));
    if(!local.startsWith(base+sep))throw Error('unsafe path');
    const bytes=await readFile(path==='/rinkcraft-stat-attributes.json'?source:local);
    res.writeHead(200,{'Content-Type':({'.html':'text/html','.js':'application/javascript','.css':'text/css','.json':'application/json','.wasm':'application/wasm'})[extname(local)]??'application/octet-stream'}).end(bytes);
  } catch {res.writeHead(404).end();}
});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const browser=await chromium.launch({channel:process.env.ICELINES_TEST_BROWSER??'msedge',headless:true});
const context=await browser.newContext({serviceWorkers:'block',viewport:{width:1280,height:900}});
const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
const base=`http://127.0.0.1:${server.address().port}/`;
const load=async id=>{
  await page.locator('#data-library').evaluate(e=>e.open=true);
  await page.locator('#catalog').selectOption(id);await page.locator('#load').click();
  await expect(page.locator('#context')).toHaveText(id.replace('-',' · '));
  await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
};
const open=async name=>{
  await page.locator('#rows td:nth-child(2) button').filter({hasText:name}).first().click();
  await expect(page.locator('#player-dialog')).toBeVisible();
};
const front=page.locator('#player-dialog .rk-front');
const grades=front.locator('[data-capability-index] dd');
const checked=[];
try {
  await page.goto(base);await expect(page.locator('#catalog option').first()).toBeAttached();
  for(const season of [20242025,20232024]){
    await load(`${season}-regular`);
    for(const [id,name] of [[8478402,'Connor McDavid'],[8480069,'Cale Makar']]){
      const expected=catalog.profiles.find(p=>p.player_id===id&&p.season===season);assert.ok(expected);
      await open(name);
      await expect(grades).toHaveText(expected.scores.map(s=>String(s.score??'—')));
      await expect(front.locator('[data-rating-state="stat_estimate"]')).toHaveCount(6);
      await front.getByRole('button',{name:'How Passing is scored',exact:true}).click();
      const formula=page.locator('.rk-capability-dialog');await expect(formula).toContainText(`${expected.games} GP`);
      await expect(formula).toContainText('Assists');await expect(formula).toContainText('stat points ÷ 5');
      await formula.getByRole('button',{name:'Close',exact:true}).click();
      checked.push({player:id,season,games:expected.games,scores:expected.scores.map(s=>s.score)});
      await page.locator('#close-player').click();
    }
  }
  // A newer resident window must not leak back into the selected older season.
  await load('20242025-regular');await load('20232024-regular');
  await context.setOffline(true);await open('Connor McDavid');
  const expected=catalog.profiles.find(p=>p.player_id===8478402&&p.season===20232024);
  await expect(grades).toHaveText(expected.scores.map(s=>String(s.score??'—')));
  await page.setViewportSize({width:390,height:844});
  assert.ok(await page.locator('#player-dialog').evaluate(e=>e.scrollWidth<=e.clientWidth));
  await page.screenshot({path:resolve(output,'ordinary-profile-mobile.png')});
  await page.locator('#close-player').click();await context.setOffline(false);
  await page.locator('#kind').selectOption('goalies');await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
  await page.locator('#rows td:nth-child(2) button').first().click();await expect(page.locator('#player-dialog')).toBeVisible();
  await expect(front.locator('[data-rating-state="stat_estimate"]')).toHaveCount(0);
  await page.setViewportSize({width:1280,height:900});
  const absentCases=[
    {name:'missing',status:404,body:'',note:undefined},
    {name:'invalid',status:200,body:'{}',note:'Stat estimates unavailable:'},
    {name:'stale GP',status:200,body:JSON.stringify({...catalog,profiles:catalog.profiles.map(p=>p.player_id===8478402&&p.season===20242025?{...p,games:p.games+1}:p)}),note:'Stat estimates for 20242025 withheld:'},
  ];
  for(const scenario of absentCases){
    await page.route('**/rinkcraft-stat-attributes.json',route=>route.fulfill({status:scenario.status,contentType:'application/json',body:scenario.body}));
    await page.goto(base);await expect(page.locator('#catalog option').first()).toBeAttached();
    await load('20242025-regular');await open('Connor McDavid');
    await expect(grades).toHaveText(Array(6).fill('—'));
    await expect(front).toContainText('Connor McDavid');
    if(scenario.note)await expect(page.locator('#rinkcraft-profile')).toContainText(scenario.note);
    await page.unroute('**/rinkcraft-stat-attributes.json');
  }
  assert.deepEqual(errors,[]);
  await writeFile(resolve(output,'results.json'),JSON.stringify({passed:true,checked,offline:true,mobile:390,goaliesExcluded:true,unavailableCases:absentCases.map(s=>s.name),debugBindingOverride:!!pkg},null,2));
  console.log('Ordinary browser attributes passed: two skaters × two seasons, formulas, resident-window isolation, offline reopen, mobile, goalie exclusion and missing/invalid/stale catalogs.');
} finally {await browser.close();await new Promise(done=>server.close(done));}
