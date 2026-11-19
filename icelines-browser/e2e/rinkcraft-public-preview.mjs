import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const base=process.env.ICELINES_PREVIEW_URL??'http://127.0.0.1:8772/';
const out=resolve(process.env.ICELINES_PREVIEW_EVIDENCE??'target/compact-sheet/public-explorer/acceptance');await mkdir(out,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];
page.on('pageerror',e=>errors.push(e.message));
let desktop=0,mobile=0;
try{
 await page.goto(base);await expect(page.locator('[data-rinkcraft-preview]')).toHaveAttribute('data-ready','true');
 const catalog=await (await page.request.get(new URL('catalog.json',base).href)).json();
 await expect(page.locator('#season')).toHaveValue(String(catalog.current_season));await expect(page.locator('#data-notice')).toContainText('Current season to date');
 for(const window of catalog.windows){
  const ids=[8478402,8479323,8478048,8473986,8480834].filter(id=>window.players.some(p=>p.id===id));
  await page.locator('#season').selectOption(String(window.season));
  for(const id of ids){
   await page.locator('#player').selectOption(String(id));const p=window.players.find(p=>p.id===id);
   await expect(page.locator('.rk-front h3')).toHaveText(p.name);await expect(page.locator('#data-notice')).toContainText(`${p.games} GP`);
   await expect(page.locator('.rk-front [data-rating-state="unassessed"]')).toHaveCount(6);
   await expect(page.locator('.rk-record-values .rk-field')).toHaveCount(p.group==='Goalies'?4:6);
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));desktop++;
   if(window.season===20242025&&[8473986,8480834,8478048].includes(id)){
    await page.screenshot({path:resolve(out,`${id}-desktop.png`),fullPage:true});
    const pdf=await page.pdf({path:resolve(out,`${id}.pdf`),preferCSSPageSize:true,printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);
   }
  }
 }
 await page.locator('#season').selectOption('20242025');await page.locator('#search').fill('killorn');await expect(page.locator('#player option')).toHaveCount(1);await page.locator('#player').selectOption('8473986');await expect(page.locator('.rk-front h3')).toHaveText('Alex Killorn');
 await page.locator('#search').fill('');await page.locator('#position').selectOption('Goalies');
 const displayed=await page.locator('#player option').evaluateAll(es=>es.map(e=>Number(e.value)));assert.ok(displayed.length>0);assert.ok(displayed.every(id=>catalog.windows.find(w=>w.season===20242025).players.find(p=>p.id===id).group==='Goalies'));
 await page.locator('#team').selectOption('NYR');assert.ok(await page.locator('#player option').count()>0);
 await page.locator('#search').fill('no such player qqq');await expect(page.locator('#player option')).toHaveCount(0);await expect(page.locator('#selection-status')).toContainText('No players match');
 await page.goto(base+'?player=8473986&season=20242025&theme=ice');await expect(page.locator('.rk-front h3')).toHaveText('Alex Killorn');
 for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){
  await page.locator('#theme').selectOption(theme);await page.setViewportSize({width:390,height:844});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await expect(page.locator('.rk-front h3')).toHaveText('Alex Killorn');mobile++;
 }
 await page.locator('#theme').selectOption('ice');await page.screenshot({path:resolve(out,'killorn-mobile.png'),fullPage:true});
 await page.reload();await expect(page.locator('.rk-front h3')).toHaveText('Alex Killorn');await expect(page.locator('#season')).toHaveValue('20242025');
 await page.goto(base+'?season=20242025&player=99999999');await expect(page.locator('#data-notice')).toContainText('No captured record');await expect(page.locator('.rk-front')).toHaveCount(0);await expect(page.locator('#source-detail a')).toHaveCount(0);
 assert.deepEqual(errors,[]);
 const result={windows:catalog.windows.map(w=>({season:w.season,players:w.players.length})),desktop,mobile,filters:['name','position','team','empty search'],deep_links:true,unknown_player_no_fallback:true,letter_exports:3,page_errors:errors};await writeFile(resolve(out,'verification.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
}finally{await browser.close();}
