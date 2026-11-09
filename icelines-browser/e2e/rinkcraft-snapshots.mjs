// Actual shared WASM/native selection, source periods and missing-player behavior.
import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const output=new URL('../../target/rinkcraft-snapshot-check/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
 const web=await browser.newContext({viewport:{width:1440,height:1100}});const app=await web.newPage();await app.goto('http://127.0.0.1:8766');await expect(app.locator('#rinkcraft-numeric-season')).toHaveValue('20252026',{timeout:15000});
 const projected=await app.evaluate(async()=>{
  const b=await import('/pkg/icelines_wasm.js');await b.default();const e=new b.BrowserEngine();const text=await(await fetch('/rinkcraft-history.json')).text();const h=JSON.parse(text);
  const ids=h.catalogs.find(c=>c.season===20242025).profiles.map(p=>p.player_id);
  const cards=ids.map(id=>({id,...JSON.parse(e.numerical_snapshot(text,id,undefined,20262027,undefined))}));
  const exact=JSON.parse(e.numerical_snapshot(text,8477493,undefined,20262027,20252026));
  const bounded=JSON.parse(e.numerical_snapshot(text,8478402,undefined,20242025,undefined));
  const legacy=await(await fetch('/rinkcraft-numbers.json')).text();const legacyCard=JSON.parse(e.numerical_snapshot(legacy,8478402,undefined,20262027,undefined));
  const legacyOutside=JSON.parse(e.numerical_snapshot(legacy,8478402,20252026,undefined,undefined));
  const latest=JSON.parse(e.numerical_catalog(text,undefined));const old=JSON.parse(e.numerical_catalog(text,20242025));e.free();
  return {cards,exact,bounded,legacyCard,legacyOutside,seasons:latest.seasons,latest:latest.catalog.season,old:old.catalog.season};
 });
 expect(projected.cards).toHaveLength(25);expect(projected.cards.filter(c=>c.selection.state==='latest')).toHaveLength(24);expect(projected.seasons).toEqual([20232024,20242025,20252026]);
 expect(projected.latest).toBe(20252026);expect(projected.old).toBe(20242025);
 expect(projected.exact.card).toBeNull();expect(projected.exact.selection.state).toBe('unavailable');expect(projected.bounded.card.season).toBe(20242025);
 expect(projected.legacyCard.card.season).toBe(20242025);expect(projected.legacyOutside.card).toBeNull();
 for(const c of projected.cards){
  const response=await app.request.get(`http://127.0.0.1:8765/api/v1/player/${c.id}/rinkcraft`);expect(response.status()).toBe(200);const p=(await response.json()).data;
  expect(p.numerical_evidence).toEqual(c.card);expect(p.numerical_snapshot).toEqual(c.selection);
  if(c.id===8477493){expect(c.card.season).toBe(20242025);expect(c.selection.state).toBe('older_fallback');expect(p.numerical_history.assessment.recent).toBeNull();}
  else expect(c.card.season).toBe(20252026);
 }
 const noJs=await browser.newContext({javaScriptEnabled:false,viewport:{width:1440,height:1100}});const page=await noJs.newPage();
 await page.goto('http://127.0.0.1:8765/player/8478402/rinkcraft');await expect(page.locator('[data-snapshot-state="latest"]')).toContainText('2025–26');await expect(page.locator('.rn-integrated')).toContainText('2025–26');await expect(page.getByRole('link',{name:'Browse same-season comparisons →'})).toHaveAttribute('href',/season=20252026/);
 await page.goto('http://127.0.0.1:8765/player/8477493/rinkcraft');await expect(page.locator('[data-snapshot-state="older_fallback"]')).toContainText('showing 2024–25');await expect(page.getByRole('link',{name:'Browse same-season comparisons →'})).toHaveAttribute('href',/season=20242025/);
 await page.screenshot({path:new URL('native-fallback.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 const exact=(await(await page.request.get('http://127.0.0.1:8765/api/v1/player/8477493/rinkcraft?season=20252026')).json()).data;expect(exact.numerical_evidence).toBeNull();expect(exact.numerical_snapshot.state).toBe('unavailable');
 for(const season of [20232024,20242025,20252026]){const c=(await(await page.request.get(`http://127.0.0.1:8765/api/v1/rinkcraft/numbers/compare?left=8478402&right=8477492&season=${season}`)).json()).data;expect(c.season).toBe(season);expect(c.left.season).toBe(season);expect(c.right.season).toBe(season);}
 expect((await page.request.get('http://127.0.0.1:8765/api/v1/rinkcraft/numbers/compare?left=8478402&right=8477493&season=20252026')).status()).toBe(400);
 expect((await page.request.get('http://127.0.0.1:8765/api/v1/rinkcraft/numbers?season=20222023')).status()).toBe(400);
 await page.goto('http://127.0.0.1:8765/rinkcraft/numbers?left=8478402&right=8477492');await expect(page.locator('select[name="season"]')).toHaveValue('20252026');await expect(page.locator('[data-style-comparison-key]')).toHaveCount(2);
 await page.locator('select[name="season"]').selectOption('20242025');await page.getByRole('button',{name:'Use season'}).click();await expect(page.locator('h2').first()).toHaveText('2024–25 regular season');
 await expect(app.locator('#rinkcraft-numeric-season')).toHaveValue('20252026');await app.locator('#rinkcraft-atlas').evaluate(e=>e.open=true);await app.locator('#rinkcraft-numeric-controls').evaluate(e=>e.open=true);
 await app.locator('#rinkcraft-numeric-left').selectOption('8478402');await app.locator('#rinkcraft-numeric-right').selectOption('8477492');await app.locator('#rinkcraft-numeric-compare').click();await expect(app.locator('#rinkcraft-numeric-result [data-style-comparison-key]')).toHaveCount(2);
 await app.locator('#rinkcraft-numeric-season').selectOption('20242025');await expect(app.locator('#rinkcraft-numeric-result')).toContainText('Choose players and compare');await app.locator('#rinkcraft-numeric-compare').click();await expect(app.locator('#rinkcraft-numeric-result [data-style-comparison-key]')).toHaveCount(5);
 // A right-side missing player also stays selected; season requests cannot silently switch peers.
 await app.locator('#rinkcraft-numeric-right').selectOption('8477493');await app.locator('#rinkcraft-numeric-season').selectOption('20252026');await expect(app.locator('#rinkcraft-numeric-right option:checked')).toContainText('unavailable or incompatible');
 await app.locator('#rinkcraft-numeric-season').selectOption('20242025');await expect(app.locator('#rinkcraft-numeric-right')).toBeEnabled();await expect(app.locator('#rinkcraft-numeric-right')).toHaveValue('8477493');
 // Keep the requested player visible as unavailable rather than silently replacing him.
 await app.locator('#rinkcraft-numeric-left').selectOption('8477493');await app.locator('#rinkcraft-numeric-season').selectOption('20252026');await expect(app.locator('#rinkcraft-numeric-left option:checked')).toContainText('unavailable this season');await expect(app.locator('#rinkcraft-numeric-right option:checked')).toContainText('unavailable or incompatible');
 await app.locator('#data-library').evaluate(e=>e.open=true);await app.locator('#catalog').selectOption('20252026-regular');await app.locator('#load').click();await expect(app.locator('#context')).toHaveText('20252026 · regular');
 await app.getByRole('button',{name:/Connor McDavid · Pathfinder/}).click();await expect(app.locator('#player-dialog [data-snapshot-state="latest"]')).toContainText('2025–26');await expect(app.locator('#player-dialog .rn-integrated')).toContainText('2025–26');
 await app.screenshot({path:new URL('wasm-latest.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});await app.locator('.rn-card-comparison select').selectOption('8477492');await app.getByRole('button',{name:'Compare this numerical profile',exact:true}).click();await expect(app.locator('#rinkcraft-numeric-season')).toHaveValue('20252026');await expect(app.locator('#rinkcraft-numeric-result')).toContainText('Connor McDavid');
 await app.locator('#data-library').evaluate(e=>e.open=true);await app.locator('#catalog').selectOption('20242025-regular');await app.locator('#load').click();await expect(app.locator('#context')).toHaveText('20242025 · regular');await app.getByRole('button',{name:/Connor McDavid · Pathfinder/}).click();await expect(app.locator('#player-dialog .rn-integrated')).toContainText('2024–25');await app.locator('.rn-card-comparison select').selectOption('8477492');await app.getByRole('button',{name:'Compare this numerical profile',exact:true}).click();await expect(app.locator('#rinkcraft-numeric-season')).toHaveValue('20242025');await expect(app.locator('#rinkcraft-numeric-result [data-style-comparison-key]')).toHaveCount(5);
 await web.setOffline(true);await app.locator('#rinkcraft-numeric-season').selectOption('20252026');await expect(app.locator('#rinkcraft-numeric-result')).toContainText('Choose players and compare');await app.locator('#rinkcraft-numeric-right').selectOption('8477492');await app.locator('#rinkcraft-numeric-compare').click();await expect(app.locator('#rinkcraft-numeric-result [data-style-comparison-key]')).toHaveCount(2);await app.locator('#rinkcraft-numeric-season').selectOption('20242025');await expect(app.locator('#rinkcraft-numeric-result')).toContainText('Choose players and compare');await app.locator('#rinkcraft-numeric-compare').click();await expect(app.locator('#rinkcraft-numeric-result [data-style-comparison-key]')).toHaveCount(5);
 await app.setViewportSize({width:390,height:844});expect(await app.locator('body').evaluate(e=>e.scrollWidth<=innerWidth)).toBe(true);
 await writeFile(new URL('results.json',output),JSON.stringify({pass:true,exact_native_wasm_snapshots:25,latest:24,explicit_fallback:1,exact_missing:true,bounds:true,legacy:true,comparison_seasons:3,missing_comparison_rejected:true,card_comparison_period:true,no_js:true,offline:true,mobile_width:390},null,2));console.log('Snapshot selection: 25 exact native/WASM, latest/fallback/exact/bounds, three comparison seasons, missing-player refusal, no-JS/offline/mobile passed.');
}finally{await browser.close();}
