import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const native=process.env.ICELINES_NATIVE_URL??'http://127.0.0.1:8772';
const wasm=process.env.ICELINES_BROWSER_URL??'http://127.0.0.1:8773';
const output=new URL('../../target/rinkcraft-live-sheet-check/',import.meta.url);await mkdir(output,{recursive:true});
const path=name=>new URL(name,output).pathname.replace(/^\/([A-Z]:)/,'$1');
const skater=['Skating','Handling','Passing','Shooting','Reading','Defending'];
const goalie=['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play'];
const b=await chromium.launch({channel:'msedge',headless:true});
try{
const web=await b.newContext({viewport:{width:1440,height:1200}}),app=await web.newPage(),errors=[];app.on('pageerror',e=>errors.push(String(e)));
await app.goto(wasm);await expect(app.locator('#rinkcraft-numeric-season')).toHaveValue('20252026',{timeout:20000});
const expected=await app.evaluate(async()=>{const m=await import('/pkg/icelines_wasm.js');await m.default();const e=new m.BrowserEngine();const h=await(await fetch('/rinkcraft-history.json')).text();const archive=JSON.parse(h);const ids=archive.catalogs.find(c=>c.season===20242025).profiles.map(p=>p.player_id);const cards=ids.map(id=>JSON.parse(e.numerical_snapshot(h,id,undefined,20262027,undefined)).card);const {numericalValue}=await import('/src/rinkcraft-numbers.js');e.free();return cards.map(c=>({card:c,formatted:c.facets.map(f=>numericalValue(f.measurement))}));});
expect(expected).toHaveLength(25);
const nojs=await b.newContext({javaScriptEnabled:false,viewport:{width:1440,height:1200}}),page=await nojs.newPage();const results=[];
let next=0;await Promise.all(Array.from({length:4},async()=>{const sample=await nojs.newPage();while(next<expected.length){const {card:c,formatted}=expected[next++];
const r=await sample.request.get(`${native}/api/v1/player/${c.player_id}/rinkcraft`);expect(r.status()).toBe(200);const p=(await r.json()).data;expect(p.numerical_evidence).toEqual(c);
await sample.goto(`${native}/player/${c.player_id}/rinkcraft`);const front=sample.locator('.rk-front');await expect(front).toBeVisible();await expect(front.locator('.rk-front-attributes dt')).toHaveText(c.group==='Goalies'?goalie:skater);await expect(front.locator('.rk-front-attributes dd')).toHaveText(Array(6).fill('—'));await expect(front.locator('.rk-record-values [data-numerical-key] dd')).toHaveText(formatted);expect(await sample.locator('.rk-evidence').evaluate(e=>e.open)).toBe(false);
results.push({id:c.player_id,name:c.name,source:c.season,values:formatted});
if([8479323,8478402,8478048].includes(c.player_id))await front.screenshot({path:path(`native-${c.player_id}.png`)});
}await sample.close();}));
results.sort((a,b)=>a.id-b.id);await writeFile(path('native-results.json'),JSON.stringify({pass:true,native_profiles:results.length,results},null,2));console.log('Native phase: 25 real profiles passed.');
await page.goto(`${native}/player/8479323/rinkcraft`);const trigger=page.locator('.rk-evidence > summary');await trigger.focus();await page.keyboard.press('Enter');await expect(page.locator('.rk-evidence')).toHaveAttribute('open','');await page.getByRole('combobox',{name:'Compare numerical profile with'}).selectOption('8480069');await page.getByRole('button',{name:'Compare this numerical profile',exact:true}).click();expect(new URL(page.url()).searchParams.get('season')).toBe('20252026');await expect(page.locator('.rn-identity')).toHaveCount(2);
await app.locator('#data-library').evaluate(e=>e.open=true);await app.locator('#catalog').selectOption('20252026-regular');await app.locator('#load').click();await expect(app.locator('#context')).toHaveText('20252026 · regular');await app.locator('#rinkcraft-atlas').evaluate(e=>e.open=true);
for(const {card:c,formatted}of expected){
if(c.player_id===8477493){await app.locator('#data-library').evaluate(e=>e.open=true);await app.locator('#catalog').selectOption('20242025-regular');await app.locator('#load').click();await expect(app.locator('#context')).toHaveText('20242025 · regular');}
else if(await app.locator('#catalog').inputValue()!=='20252026-regular'){await app.locator('#data-library').evaluate(e=>e.open=true);await app.locator('#catalog').selectOption('20252026-regular');await app.locator('#load').click();await expect(app.locator('#context')).toHaveText('20252026 · regular');}
await app.getByRole('button',{name:new RegExp(`^${c.name.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')} ·`)}).click();const front=app.locator('#player-dialog .rk-front');await expect(front).toBeVisible();await expect(front.locator('.rk-front-attributes dt')).toHaveText(c.group==='Goalies'?goalie:skater);await expect(front.locator('.rk-front-attributes dd')).toHaveText(Array(6).fill('—'));await expect(front.locator('.rk-record-values [data-numerical-key] dd')).toHaveText(formatted);expect(await app.locator('#player-dialog .rk-evidence').evaluate(e=>e.open)).toBe(false);
if([8479323,8478402,8478048].includes(c.player_id))await front.screenshot({path:path(`wasm-${c.player_id}.png`)});
await app.locator('#player-dialog').getByRole('button',{name:'Close',exact:true}).click();
}
await web.setOffline(true);await app.setViewportSize({width:390,height:844});await app.getByRole('button',{name:/^Adam Fox ·/}).click();await expect(app.locator('#player-dialog .rk-front')).toBeVisible();expect(await app.locator('#player-dialog').evaluate(e=>e.scrollWidth<=e.clientWidth)).toBe(true);await app.screenshot({path:path('wasm-mobile.png')});
for(const theme of ['night-game','fresh-ice','hockey-club']){await app.evaluate(theme=>{document.documentElement.dataset.theme=theme;},theme);const minimum=await app.locator('.rk-front').evaluate(e=>{const rgb=s=>s.match(/[\d.]+/g).slice(0,3).map(Number);const lum=c=>c.map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((a,v,i)=>a+v*[.2126,.7152,.0722][i],0);const bg=lum([245,249,252]);return Math.min(...[...e.querySelectorAll('dt,dd,h2,h4,small,p')].map(n=>{const v=lum(rgb(getComputedStyle(n).color));return (Math.max(bg,v)+.05)/(Math.min(bg,v)+.05);}));});expect(minimum).toBeGreaterThanOrEqual(4.5);}
await app.locator('#player-dialog .rk-evidence > summary').click();await app.locator('#player-dialog .rn-card-comparison select').selectOption('8480069');await app.locator('#player-dialog .rn-card-comparison button').click();await expect(app.locator('.rn-identity')).toHaveCount(2);await expect(app.locator('#rinkcraft-numeric-result')).toContainText('Cale Makar');expect(errors).toEqual([]);
await writeFile(path('results.json'),JSON.stringify({pass:true,native_profiles:25,browser_profiles:25,exact_api_engine_cards:true,capabilities_unrated:true,source_values:results,native_no_js_keyboard_comparison:true,offline_mobile_comparison:true,themes:3},null,2));console.log('Live sheet: 25 native and 25 browser profiles, exact engine/API cards, separate unrated capabilities, no-JS keyboard/native comparison and offline/mobile/theme/browser comparison passed.');
}finally{await b.close();}
