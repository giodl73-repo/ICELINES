// Native numerical assessment acceptance against explicit local source-derived JSON.
import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const base=process.env.ICELINES_NATIVE_URL??'http://127.0.0.1:8765';
const output=new URL('../../target/rinkcraft-numbers-check/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const context=await browser.newContext({viewport:{width:1440,height:1000},javaScriptEnabled:false});const page=await context.newPage();
 const response=await page.request.get(`${base}/api/v1/rinkcraft/numbers`);expect(response.status()).toBe(200);const catalog=(await response.json()).data;
 expect(catalog.profiles).toHaveLength(25);expect(catalog.missing_players).toEqual([]);expect(catalog.season).toBe(20242025);
 expect(catalog.sources.map(s=>s.sha256)).toEqual(['467260ae6b411621fc3e736ed005588f75dd4470610416626584f2878a463fd4','a718782ffbeb8d0d05eb6921f4cbea300dbcaa86be8e068930e71be4a22a9d8d']);
 await page.goto(`${base}/rinkcraft/numbers`);await expect(page.locator('.rn-card')).toHaveCount(25);await expect(page.getByRole('heading',{name:'2024–25 regular season',exact:true})).toBeVisible();
 let measured=0;
 for(const p of catalog.profiles){const card=page.locator(`[data-player-id="${p.player_id}"]`);await expect(card.locator('h2')).toHaveText(p.name);const rows=card.locator('tbody tr');await expect(rows).toHaveCount(p.group==='Goalies'?5:15);
  expect(p.situation).toBe(p.group==='Goalies'?'all':'5on5');
  for(const [i,m]of p.metrics.entries()){
   expect(m.state).toBe('ranked');expect(m.measured_peers).toBe(p.eligible_peers);
   expect(m.value).toBeCloseTo(m.numerator/m.denominator*m.scale,10);
   const raw=Number((await rows.nth(i).locator('.rn-value').textContent()).replace('%',''));expect(Math.abs(raw-m.value*(m.unit==='Pct'?100:1))).toBeLessThanOrEqual(0.005001);
   await expect(rows.nth(i).locator('.rn-percentile')).toHaveText(`P${(Math.round(m.percentile*10)/10).toFixed(1)}`);measured++;
  }
 }
 expect(measured).toBe(345);
 const mc=catalog.profiles.find(p=>p.player_id===8478402);expect(mc.metrics[0].numerator).toBe(23);expect(mc.metrics[0].denominator).toBe(70524);expect(mc.eligible_peers).toBe(424);
 const helle=catalog.profiles.find(p=>p.player_id===8476945);expect(helle.metrics[0].numerator).toBeCloseTo(39.59,8);expect(helle.metrics[0].denominator).toBe(224482);expect(helle.eligible_peers).toBe(71);
 await page.screenshot({path:new URL('numbers-desktop.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.setViewportSize({width:390,height:844});expect(await page.locator('body').evaluate(e=>e.scrollWidth<=innerWidth)).toBe(true);
 await page.screenshot({path:new URL('numbers-mobile.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
 await page.goto(`${base}/rinkcraft`);await expect(page.getByRole('link',{name:'Open numerical assessments →'})).toBeVisible();
 await writeFile(new URL('results.json',output),JSON.stringify({pass:true,profiles:25,measured_cells:measured,source_sha256:catalog.sources.map(s=>s.sha256),peer_groups:{Forwards:424,Defense:240,Goalies:71},no_js:true,mobile_width:390},null,2));
 console.log('25 numerical profiles, 345 formula/value/percentile cells, source hashes, scope, peer counts, no-JS and mobile passed.');
}finally{await browser.close();}
