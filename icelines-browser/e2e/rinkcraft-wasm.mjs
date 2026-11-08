// Local full-engine/browser acceptance; serve the built distribution first.
import { chromium, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
const base=process.env.ICELINES_TEST_URL ?? 'http://127.0.0.1:8766';
const output=new URL('../../target/rinkcraft-wasm-check/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
const context=await browser.newContext({viewport:{width:1280,height:900}});const page=await context.newPage();
try {
  await page.goto(base);await expect(page.locator('#catalog option').first()).toBeAttached();
  for(const id of ['20242025-regular','20242025-playoff','20252026-regular']) {
    await page.locator('#data-library').evaluate(e=>e.open=true);
    await page.locator('#catalog').selectOption(id);await page.locator('#load').click();
    await expect(page.locator('#context')).toHaveText(id.replace('-',' · '));
    await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
  }
  // Independent engine instance proves the dialog displays the exported WASM view.
  const expected=await page.evaluate(async()=>{
    const bindings=await import('/pkg/icelines_wasm.js');await bindings.default();const engine=new bindings.BrowserEngine();
    const catalog=await (await fetch('/catalog.json')).json();
    for(const id of ['20242025-regular','20242025-playoff','20252026-regular']){
      const p=catalog.packages.find(p=>p.id===id);engine.load_package(new Uint8Array(await (await fetch('/'+p.url)).arrayBuffer()));
    }
    const profile=JSON.parse(engine.rinkcraft_profile(JSON.stringify({player_id:8479323,season_type:'regular',first_season:null,last_season:null})));
    engine.free();return profile;
  });
  expect(expected.included_seasons).toEqual([20242025,20252026]);
  await page.locator('#filter').fill('team=NYR');await page.getByRole('button',{name:'Run query',exact:true}).click();
  await page.getByRole('button',{name:'Adam Fox',exact:true}).click();
  const profile=page.locator('#rinkcraft-profile');await expect(profile).toBeVisible();
  await expect(profile).toContainText('2 loaded seasons');
  expect(await profile.locator('.rinkcraft-families > details').count()).toBe(6);
  const goals=profile.locator('tbody tr').filter({has:page.getByText('Goals · goals',{exact:true})});
  const latest=expected.latest_available.metrics.find(m=>m.key==='goals').value;
  await expect(goals.locator('td').nth(1)).toHaveText(String(latest));
  await context.setOffline(true);
  await profile.getByLabel('Rinkcraft season view').selectOption('summary');
  await expect(goals.locator('td').nth(1)).toHaveText(String(expected.summary.find(m=>m.key==='goals').value));
  await profile.getByLabel('Find metric').fill('takeaways');
  await expect(profile.locator('tbody tr')).toHaveCount(0);
  await profile.getByLabel('Show unavailable metrics').check();
  await expect(profile.locator('tbody')).toContainText('—');
  await profile.getByLabel('Find metric').fill('');
  await page.setViewportSize({width:390,height:844});await profile.scrollIntoViewIfNeeded();
  expect(await page.locator('#player-dialog').evaluate(e=>e.scrollWidth<=e.clientWidth)).toBe(true);
  await page.screenshot({path:new URL('fox-mobile.png',output).pathname.replace(/^\/([A-Z]:)/,'$1')});
  await writeFile(new URL('results.json',output),JSON.stringify({pass:true,offline_controls:true,resident_regular_seasons:expected.included_seasons,engine_schema:expected.schema,mobile_width:390},null,2));
  console.log('WASM Rinkcraft: actual engine/dialog parity, competition isolation, offline selectors, gaps and mobile layout passed.');
} finally {await browser.close();}
