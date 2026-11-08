// Local acceptance against an already running native IceLines server.
import { chromium, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
const base=process.env.ICELINES_TEST_URL ?? 'http://127.0.0.1:8765';
const output=new URL('../../target/rinkcraft-ui-check/',import.meta.url);
await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
const context=await browser.newContext({viewport:{width:1440,height:1000},javaScriptEnabled:false});
const page=await context.newPage();
try {
  const failures=[];page.on('pageerror',e=>failures.push(String(e)));
  const cases=[['fox',8479323,''],['kartye',8481789,''],['shesterkin',8478048,''],['fox-window',8479323,'?first_season=20232024&last_season=20252026'],['shesterkin-playoff',8478048,'?type=playoff']];
  const results=[];
  for(const [name,id,query] of cases){
    const response=await page.goto(`${base}/player/${id}/rinkcraft${query}`);
    expect(response.status()).toBe(200);
    await expect(page.getByRole('heading',{level:1})).not.toBeEmpty();
    await expect(page.getByRole('heading',{name:'Six job families'})).toBeVisible();
    expect(await page.locator('.rc-family').count()).toBe(6);
    const api=await context.request.get(`${base}/api/v1/player/${id}/rinkcraft${query}`);
    expect(api.status()).toBe(200);const profile=(await api.json()).data;
    expect(profile.schema).toBe('icelines.rinkcraft-stat-profile.v1');
    results.push({name,player:profile.player_name,seasons:profile.included_seasons,latest:profile.latest_available?.season});
    await page.screenshot({path:new URL(`${name}.png`,output).pathname.replace(/^\/([A-Z]:)/,'$1'),fullPage:false});
  }
  await page.goto(`${base}/player/8479323/rinkcraft`);
  await page.getByLabel('Display',{exact:true}).selectOption('20262027');
  await page.getByRole('button',{name:'Update profile'}).click();
  await expect(page.getByRole('heading',{name:'2026–27',exact:true})).toBeVisible();
  await page.setViewportSize({width:390,height:844});
  await page.screenshot({path:new URL('fox-mobile.png',output).pathname.replace(/^\/([A-Z]:)/,'$1'),fullPage:false});
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  const bad=await context.request.get(`${base}/api/v1/player/8479323/rinkcraft?type=typo`);expect(bad.status()).toBe(400);
  const missing=await context.request.get(`${base}/api/v1/player/99999999/rinkcraft`);expect(missing.status()).toBe(404);
  expect(failures).toEqual([]);
  await writeFile(new URL('results.json',output),JSON.stringify({base,no_js:true,cases:results},null,2));
  console.log(JSON.stringify({pass:true,no_js:true,cases:results,mobile_width:390}));
} finally {await browser.close();}
