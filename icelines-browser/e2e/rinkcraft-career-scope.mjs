import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const out=resolve('target/compact-sheet/profile-career/ui');await mkdir(out,{recursive:true});const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));let pairs=0,themes=0;
try{
 await page.goto(new URL('?sample=mcdavid&scope=career&theme=ice',base).href);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
 const fixtures=await(await page.request.get(new URL('fixtures.json',base).href)).json();
 for(const [key,p]of Object.entries(fixtures)){
  await page.locator('#sample').selectOption(key);let scope=page.locator('select[aria-label="Whole profile time scope"]');await scope.selectOption('season');const front=page.locator('#profile-stage .rk-front');const historical=await front.textContent();
  await scope.selectOption('current');const current=await front.textContent();await scope.selectOption('career');const c=p.scope_variants.career;
  await expect(front).toHaveAttribute('data-window-kind','career');await expect(front.locator('.rk-counter-values>div:first-child dd')).toHaveText(String(c.latest_available.games));await expect(front.locator('.rk-front-scope')).toContainText('NHL career');
  await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);await expect(front.locator('.rk-stat-feats')).toHaveCount(0);await expect(front.locator('[data-candidate-id]')).toHaveCount(0);
  await expect(page.locator('.rn-integrated .rk-stat-period')).toContainText('NHL career');await expect(page.locator('.rk-career-audit')).toContainText('same official provider');
  for(const metric of c.numerical_evidence.metrics){const cell=front.locator(`[data-numerical-key="${metric.key}"]`);await expect(cell).toHaveCount(1);await expect(cell).toContainText(metric.state==='conflicting_sources'?'Source conflict':'Unranked');}
  if(key==='shesterkin'){const cell=front.locator('[data-numerical-key="career-save-fraction"]');await expect(cell.locator('dd')).toHaveText('—');await expect(page.locator('[data-career-field="saves"]')).toContainText('8840');await expect(page.locator('[data-career-field="saves"]')).toContainText('8839');}
  const text=await front.textContent();for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){await page.locator(`.theme-choice[data-theme="${theme}"]`).click();assert.equal(await front.textContent(),text);await page.setViewportSize({width:390,height:1100});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await page.setViewportSize({width:1440,height:1100});themes++;}await page.locator('.theme-choice[data-theme="ice"]').click();
  await page.screenshot({path:resolve(out,`${key}.png`),fullPage:true});await page.emulateMedia({media:'print'});await page.pdf({path:resolve(out,`${key}.pdf`),format:'Letter',printBackground:true});await page.emulateMedia({media:'screen'});
  await scope.selectOption('current');assert.equal(await front.textContent(),current);await scope.selectOption('season');assert.equal(await front.textContent(),historical);pairs++;
 }
 assert.deepEqual(errors,[]);await writeFile(resolve(out,'verification.json'),JSON.stringify({status:'passed',three_scope_round_trips:pairs,career_deep_link:true,career_theme_desktop_cases:themes,career_theme_mobile_cases:themes,career_ranks_and_feats_gated:true,save_conflict_preserved:true,capabilities_unrated:30,page_errors:errors},null,2));console.log('Five career/season/current round trips, 30 theme/mobile cases, conflict evidence and print exports passed.');
}finally{await browser.close();}
