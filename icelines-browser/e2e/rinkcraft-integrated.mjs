// Actual native/WASM prepared numerical projection and matched comparisons.
import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const native='http://127.0.0.1:8765',wasm='http://127.0.0.1:8766';
const output=new URL('../../target/rinkcraft-integrated-check/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000}});const page=await context.newPage();
 for(const [id,facets]of [[8478402,6],[8480069,6],[8476945,4]]){
  const p=(await(await page.request.get(`${native}/api/v1/player/${id}/rinkcraft`)).json()).data;expect(p.numerical_evidence.season).toBe(20252026);
  await page.goto(`${native}/player/${id}/rinkcraft`);await expect(page.locator('.rn-facet')).toHaveCount(facets);await expect(page.locator('.rn-integrated')).toContainText('2025–26');expect(await page.locator('.rk-ability:visible').count()).toBe(0);expect(await page.locator('.rk-statline:visible').count()).toBe(0);
  await page.screenshot({path:new URL(`native-${id}.png`,output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 }
 for(const query of ['?type=playoff','?first_season=20262027']){const p=(await(await page.request.get(`${native}/api/v1/player/8478402/rinkcraft${query}`)).json()).data;expect(p.numerical_evidence).toBeNull();}
 await page.goto(`${native}/player/8478483/rinkcraft?season=20242025`);await expect(page.locator('.rk-class-name')).toHaveText('Unscouted');await expect(page.locator('.rn-facet')).toHaveCount(6);
 const comparisons=[];
 for(const [left,right]of [[8478402,8477492],[8479323,8480069],[8478048,8476945]]){
  const response=await page.request.get(`${native}/api/v1/rinkcraft/numbers/compare?season=20242025&left=${left}&right=${right}`);expect(response.status()).toBe(200);comparisons.push((await response.json()).data);
 }
 for(const pair of ['left=8478402&right=8479323','left=8478402&right=8478402'])expect((await page.request.get(`${native}/api/v1/rinkcraft/numbers/compare?${pair}`)).status()).toBe(400);
 await page.goto(`${native}/rinkcraft/numbers?left=8478402&right=8477492`);await expect(page.locator('.rn-comparison')).toContainText('Connor McDavid');await expect(page.locator('.rn-comparison')).toContainText('Nathan MacKinnon');await expect(page.locator('.rn-dumbbell')).toHaveCount(6);
 await page.screenshot({path:new URL('native-comparison.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.setViewportSize({width:390,height:844});expect(await page.locator('body').evaluate(e=>e.scrollWidth<=innerWidth)).toBe(true);await page.screenshot({path:new URL('native-comparison-mobile.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.goto(wasm);await expect(page.locator('#catalog option').first()).toBeAttached();
 const actual=await page.evaluate(async()=>{
  const b=await import('/pkg/icelines_wasm.js');await b.default();const e=new b.BrowserEngine();const c=await(await fetch('/rinkcraft-numbers.json')).json();
  const cards=c.profiles.map(p=>JSON.parse(e.numerical_evidence(JSON.stringify(c),p.player_id,20252026)));
  const comparisons=[[8478402,8477492],[8479323,8480069],[8478048,8476945]].map(([a,z])=>JSON.parse(e.numerical_comparison(JSON.stringify(c),a,z)));
  const earlier=JSON.parse(e.numerical_evidence(JSON.stringify(c),8478402,20232024));e.free();return {cards,comparisons,earlier};
 });expect(actual.cards).toHaveLength(25);expect(actual.comparisons).toEqual(comparisons);expect(actual.earlier).toBeNull();
 await page.locator('#data-library').evaluate(e=>e.open=true);await page.locator('#catalog').selectOption('20252026-regular');await page.locator('#load').click();await expect(page.locator('#context')).toHaveText('20252026 · regular');
 await page.locator('#rinkcraft-atlas').evaluate(e=>e.open=true);await page.getByRole('button',{name:/Connor McDavid · Pathfinder/}).click();await expect(page.locator('#player-dialog .rn-facet')).toHaveCount(6);await expect(page.locator('#player-dialog .rn-integrated')).toContainText('2025–26');
 await page.setViewportSize({width:1280,height:1000});await page.screenshot({path:new URL('wasm-mcdavid.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.locator('.rk-evidence > summary').click();await page.locator('.rn-card-comparison select').selectOption('8477492');await page.getByRole('button',{name:'Compare this numerical profile',exact:true}).click();await expect(page.locator('#rinkcraft-numeric-result')).toContainText('Connor McDavid');
 await page.locator('#rinkcraft-numeric-right').selectOption('8477492');await page.locator('#rinkcraft-numeric-compare').click();await expect(page.locator('#rinkcraft-numeric-result')).toContainText('Nathan MacKinnon');
 await context.setOffline(true);await page.locator('#rinkcraft-numeric-left').selectOption('8478048');await page.locator('#rinkcraft-numeric-right').selectOption('8476945');await page.locator('#rinkcraft-numeric-compare').click();await expect(page.locator('#rinkcraft-numeric-result')).toContainText('Connor Hellebuyck');await expect(page.locator('#rinkcraft-numeric-result .rn-dumbbell')).toHaveCount(4);
 await page.setViewportSize({width:390,height:844});expect(await page.locator('body').evaluate(e=>e.scrollWidth<=innerWidth)).toBe(true);
 await writeFile(new URL('results.json',output),JSON.stringify({pass:true,wasm_cards:25,matched_comparisons:3,temporal_and_competition_guards:true,unscouted_numeric_card:true,offline_comparisons:true,mobile_width:390},null,2));console.log('Integrated native/WASM cards, exact matched comparison parity, temporal/competition guards, unscouted numerical evidence, offline comparisons and mobile passed.');
}finally{await browser.close();}
