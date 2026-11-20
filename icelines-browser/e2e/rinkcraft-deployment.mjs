import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const out=resolve('target/compact-sheet/deployment/ui');await mkdir(out,{recursive:true});
const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];
page.on('pageerror',e=>errors.push(e.message));let rows=0;
try{
 await page.goto(base);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
 const response=await page.request.get(new URL('fixtures.json',base).href);assert.ok(response.ok());const fixtures=await response.json();
 for(const [key,p]of Object.entries(fixtures)){
  const d=p.deployment_evidence;assert.ok(d?.source_sha256&&d?.source_url);
  await page.locator('#sample').selectOption(key);await page.locator('.theme-choice[data-theme="ice"]').click();
  const front=page.locator('#profile-stage .rk-front'),usage=front.locator('.rk-usage');await expect(usage).toHaveAttribute('data-usage-season',String(d.season));
  for(const row of d.rows){await expect(usage.locator(`[data-usage-key="${row.key}"] dd`)).toHaveText(row.per_game_display);rows++;}
  await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);
  await expect(usage).toContainText('Unit / station undocumented');
  const details=usage.locator('details');await details.locator('summary').click();await expect(details.locator('a')).toHaveAttribute('href',d.source_url);await expect(details).toContainText(d.source_sha256);
  await expect(details).toContainText('not team opportunities');if(!d.goalie)await expect(details).toContainText('not exclusively 5v5');
  await details.locator('summary').click();await page.screenshot({path:resolve(out,key+'.png'),fullPage:true});await page.pdf({path:resolve(out,key+'.pdf'),format:'Letter',printBackground:true});
  await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await details.locator('summary').click();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await details.locator('summary').click();await page.setViewportSize({width:1440,height:1100});
 }
 assert.equal(rows,17);assert.deepEqual(errors,[]);
 await writeFile(resolve(out,'verification.json'),JSON.stringify({status:'passed',players:5,workload_rows:rows,source_links:5,source_hashes:5,mobile_cases:5,print_exports:5,capabilities_unrated:30,page_errors:errors},null,2));
 console.log('Five workload cards / seventeen measured rows / source disclosures / mobile / five print exports passed.');
}finally{await browser.close();}
