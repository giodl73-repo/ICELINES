import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const out=resolve('target/compact-sheet/profile-current/ui');await mkdir(out,{recursive:true});const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));let pairs=0,themeCases=0;
try{
 await page.goto(new URL('?theme=ice&sample=mcdavid&scope=current',base).href);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
 const fixtures=await(await page.request.get(new URL('fixtures.json',base).href)).json();
 await expect(page.locator('.rk-front')).toHaveAttribute('data-window-kind','current');
 for(const [key,p]of Object.entries(fixtures)){
  await page.locator('#sample').selectOption(key);let scope=page.locator('select[aria-label="Whole profile time scope"]');await scope.selectOption('season');
  const front=page.locator('#profile-stage .rk-front');const historical=await front.textContent();await expect(front).toHaveAttribute('data-window-season','20242025');
  await scope.selectOption('current');const c=p.scope_variants.current;await expect(front).toHaveAttribute('data-window-kind','current');await expect(front).toHaveAttribute('data-window-season','20262027');
  await expect(front.locator('.rk-counter-values>div:first-child dd')).toHaveText(String(c.latest_available.games));await expect(front.locator('.rk-front-scope')).toContainText('Captured');await expect(front.locator('.rk-front-scope')).toContainText('all strengths');
  await expect(front.locator('.rk-class-note')).toContainText('Historical dossier');await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);
  await expect(page.locator('.rn-integrated .rk-stat-period')).toContainText('NHL public reports');await expect(page.locator('.rn-source-rules')).toContainText('20 skater GP');await expect(page.locator('.rn-source-rules')).not.toContainText('Skaters: 5-on-5');
  assert.equal(await scope.locator('option[value="career"]').evaluate(o=>o.disabled),!p.scope_variants?.career);assert.ok(new URL(page.url()).searchParams.get('scope')==='current');
  for(const facet of c.numerical_evidence.facets){const cell=front.locator(`[data-numerical-key="${facet.measurement.key}"]`);await expect(cell).toHaveCount(1);if(facet.measurement.percentile===null)await expect(cell).toContainText('Unranked');}
  if(key==='barkov'){await expect(front.locator('[data-numerical-key="points"] dd')).toHaveText('0.00');await expect(front.locator('[data-numerical-key="points"] dt')).toHaveText('Points / GP');assert.equal(c.achievements.earned,0);}
  if(key==='shesterkin'){assert.equal(c.achievements.earned,0);await expect(front.locator('[data-candidate-id]')).toHaveCount(0);}
  const currentText=await front.textContent();for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){
   await page.locator(`.theme-choice[data-theme="${theme}"]`).click();assert.equal(await front.textContent(),currentText);await expect(front).toHaveAttribute('data-window-kind','current');
   await page.setViewportSize({width:390,height:1100});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await page.setViewportSize({width:1440,height:1100});themeCases++;
  }await page.locator('.theme-choice[data-theme="ice"]').click();
  await page.screenshot({path:resolve(out,`${key}.png`),fullPage:true});await page.emulateMedia({media:'print'});await page.pdf({path:resolve(out,`${key}.pdf`),format:'Letter',printBackground:true});await page.emulateMedia({media:'screen'});
  await page.setViewportSize({width:390,height:1100});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await page.setViewportSize({width:1440,height:1100});
  await scope.selectOption('season');assert.equal(await front.textContent(),historical);pairs++;
 }
 await page.goto(new URL(`?sample=fox&scope=${fixtures.fox.scope_variants?.career?'unsupported':'career'}`,base).href);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','error');await expect(page.locator('.rk-front')).toHaveCount(0);
 assert.deepEqual(errors,[]);await writeFile(resolve(out,'verification.json'),JSON.stringify({status:'passed',profile_pairs:pairs,current_theme_desktop_cases:themeCases,current_theme_mobile_cases:themeCases,whole_frame_round_trip:true,current_deep_link:true,unsupported_scope_rejected:true,zero_preserved:true,low_gp_unranked:true,page_errors:errors},null,2));console.log('Five whole-profile scope round trips, 30 current theme/mobile cases, current deep link and print exports passed.');
}finally{await browser.close();}
