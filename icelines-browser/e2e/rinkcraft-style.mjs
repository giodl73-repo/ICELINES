// Real retained source components, native cards and actual WASM selection.
import {chromium,expect} from '@playwright/test';
import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
const output=new URL('../../target/rinkcraft-style-check/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
 const old=JSON.parse(await readFile(new URL('../../target/rinkcraft-numerical-pilot/20242025/assessments.json',import.meta.url),'utf8'));
 const current=JSON.parse(await readFile(new URL('../public/rinkcraft-numbers.json',import.meta.url),'utf8'));
 const ctx=await browser.newContext({javaScriptEnabled:false,viewport:{width:1440,height:1100}});const page=await ctx.newPage();
 const native=[];let measured=0;
 for(const p of current.profiles){
  assert.deepEqual(p.metrics,old.profiles.find(x=>x.player_id===p.player_id).metrics);
  const peer=current.profiles.find(x=>x.group===p.group&&x.player_id!==p.player_id);
  // Pure numerical API avoids unrelated career hydration for this source check.
  const evidence=(await(await page.request.get(`http://127.0.0.1:8765/api/v1/rinkcraft/numbers/compare?left=${p.player_id}&right=${peer.player_id}`)).json()).data.left;
  native.push(evidence);const s=evidence.style_evidence;expect(s.season).toBe(20242025);
  expect(s.measured).toHaveLength(p.group==='Goalies'?0:p.player_id===8478402?5:2);measured+=s.measured.length;
  for(const m of s.measured)expect(m.value).toBeCloseTo(m.numerator/m.denominator,10);
  expect(s.gaps.length).toBeGreaterThanOrEqual(3);
  if(p.group!=='Goalies')expect(s.gaps.some(g=>g.axis==='North–south / east–west')).toBe(true);
 }
 expect(measured).toBe(47);
 await page.goto('http://127.0.0.1:8765/player/8478402/rinkcraft');await expect(page.locator('.rs-measure')).toHaveCount(5);
 await expect(page.locator('[data-style-key="max-skating-speed"]')).toContainText('23.97 mph');
 await expect(page.locator('[data-style-key="speed-bursts-per-game"]')).toContainText('6.60 /game');
 await page.locator('.rs-style').screenshot({path:new URL('native-mcdavid-style.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.setViewportSize({width:390,height:844});expect(await page.locator('body').evaluate(e=>e.scrollWidth<=innerWidth)).toBe(true);
 await page.goto('http://127.0.0.1:8765/player/8476945/rinkcraft');await expect(page.locator('.rs-measure')).toHaveCount(0);await expect(page.locator('.rs-style')).toContainText('Depth & movement');
 const web=await browser.newContext();const app=await web.newPage();await app.goto('http://127.0.0.1:8766');
 const actual=await app.evaluate(async()=>{const b=await import('/pkg/icelines_wasm.js');await b.default();const e=new b.BrowserEngine();const c=await(await fetch('/rinkcraft-numbers.json')).json();const cards=c.profiles.map(p=>JSON.parse(e.numerical_evidence(JSON.stringify(c),p.player_id,20252026)));e.free();return cards;});
 expect(actual).toEqual(native);
 await app.locator('#data-library').evaluate(e=>e.open=true);await app.locator('#catalog').selectOption('20252026-regular');await app.locator('#load').click();await expect(app.locator('#context')).toHaveText('20252026 · regular');
 await app.locator('#rinkcraft-atlas').evaluate(e=>e.open=true);await app.getByRole('button',{name:/Connor McDavid · Pathfinder/}).click();await expect(app.locator('#player-dialog .rs-measure')).toHaveCount(5);
 await web.setOffline(true);await expect(app.locator('#player-dialog .rs-style')).toContainText('Unmeasured');
 await app.locator('#player-dialog .rs-style').scrollIntoViewIfNeeded();await app.screenshot({path:new URL('wasm-mcdavid-style.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await app.setViewportSize({width:390,height:844});expect(await app.locator('#player-dialog').evaluate(e=>e.scrollWidth<=e.clientWidth)).toBe(true);
 await writeFile(new URL('results.json',output),JSON.stringify({pass:true,players:25,skaters:22,goalies:3,tracking_players:1,style_readouts:47,original_measurements_unchanged:345,exact_native_wasm_cards:25,no_js:true,offline:true,mobile_width:390},null,2));
 console.log('Style: 25 exact native/WASM cards, 47 readouts, 345 unchanged measurements, one tracking pilot, goalie gaps, no-JS/offline/mobile passed.');
} finally {await browser.close();}
