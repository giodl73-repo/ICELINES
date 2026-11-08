// Local optional research-catalog acceptance; player data remains external.
import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const native=process.env.ICELINES_NATIVE_URL??'http://127.0.0.1:8765';
const wasm=process.env.ICELINES_BROWSER_URL??'http://127.0.0.1:8766';
const output=new URL('../../target/rinkcraft-atlas-check/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1440,height:1000}});
 const catalog=await(await fetch(`${wasm}/rinkcraft-characters.json`)).json();
 const featured=catalog.assessments.filter(a=>a.featured);expect(featured).toHaveLength(25);
 await page.goto(`${native}/rinkcraft`);await expect(page.locator('.rk-atlas-card')).toHaveCount(26);
 const records=[];
 for(const a of featured){
  const response=await page.request.get(`${native}/api/v1/player/${a.player_id}/rinkcraft`);expect(response.status(),a.player_name).toBe(200);
  const p=(await response.json()).data;expect(p.character_sheet?.primary.id,a.player_name).toBe(a.primary.class_id);
  await writeFile(new URL(`${a.player_id}.json`,output),JSON.stringify(p,null,2));
  await page.goto(`${native}/player/${a.player_id}/rinkcraft`);await expect(page.locator('.rk-class-name').first()).toHaveText(p.character_sheet.primary.name);
  expect(await page.locator('.rk-stat-cell').count()).toBe(9);expect(await page.locator('#trait-evolution').evaluate(e=>e.open)).toBe(false);
  const assessment=p.attribute_seasons.at(-1);
  expect(await page.locator('.rk-ability .rk-pctl').allTextContents()).toEqual(assessment.attributes.map(a=>a.percentile==null?'—':`P${(Math.round(a.percentile*10)/10).toFixed(1)}`));
  const metric=p.latest_available.metrics.find(m=>m.key===(p.latest_available.position==='Goalie'?'saves':'points'));
  await expect(page.locator(`[data-stat-key="${metric.key}"] strong`)).toHaveText(String(metric.value));
  if(['Connor McDavid','Cale Makar','Connor Hellebuyck','Auston Matthews'].includes(a.player_name))await page.screenshot({path:new URL(`${a.player_id}.png`,output).pathname.replace(/^\/([A-Z]:)/,'$1')});
  records.push({id:a.player_id,name:a.player_name,class:p.character_sheet.primary.id,season:p.latest_available.season,evidence:p.character_sheet.evidence_through,catalog_sha256:p.character_sheet.catalog_sha256});
 }
 await page.setViewportSize({width:390,height:844});await page.goto(`${native}/player/8478402/rinkcraft`);
 expect(await page.locator('body').evaluate(e=>e.scrollWidth<=innerWidth)).toBe(true);
 await page.screenshot({path:new URL('mcdavid-mobile.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.goto(wasm);await expect(page.locator('#rinkcraft-atlas-cards button')).toHaveCount(26);
 const binding=await page.evaluate(async()=>{
  const b=await import('/pkg/icelines_wasm.js');await b.default();const engine=new b.BrowserEngine();
  const c=await(await fetch('/rinkcraft-characters.json')).json();const result=c.assessments.filter(a=>a.featured).map(a=>({id:a.player_id,class:JSON.parse(engine.character_sheet(JSON.stringify(c),a.player_id,20252026)).primary.id}));engine.free();return result;
 });
 expect(binding).toEqual(featured.map(a=>({id:a.player_id,class:a.primary.class_id})));
 await page.locator('#data-library').evaluate(e=>e.open=true);await page.locator('#catalog').selectOption('20252026-regular');await page.locator('#load').click();await expect(page.locator('#context')).toHaveText('20252026 · regular');
 await page.locator('#rinkcraft-atlas').evaluate(e=>e.open=true);await page.getByRole('button',{name:/Connor McDavid · Pathfinder/}).click();await expect(page.locator('.rk-class-name')).toHaveText('Pathfinder');await expect(page.locator('.rk-feature-stat')).not.toHaveCount(0);
 await page.locator('#close-player').click();await page.getByRole('button',{name:/Connor Hellebuyck · Warden/}).click();await expect(page.locator('.rk-class-name')).toHaveText('Warden');await expect(page.locator('[data-stat-key="saves"] strong')).not.toHaveText('—');
 await writeFile(new URL('results.json',output),JSON.stringify({pass:true,records,wasm_classes:binding},null,2));
 console.log('25 native classes/stat cards, 26 atlas links, mobile, actual WASM selection and skater/goalie atlas opening passed.');
}finally{await browser.close();}
