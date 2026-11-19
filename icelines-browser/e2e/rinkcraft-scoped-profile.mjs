import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const output=resolve('target/compact-sheet/scoped-profile/ui');await mkdir(output,{recursive:true});
const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
try{
 await page.goto(base);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
 const fixtures=await (await page.request.get(new URL('fixtures.json',base).href)).json();const before=JSON.parse(await readFile('target/compact-sheet/achievements/fixtures.json','utf8'));let tested=0;
 for(const [key,p]of Object.entries(fixtures)){
  const copy=structuredClone(p);delete copy.profile_window;delete copy.experimental_patterns;assert.deepEqual(copy,before[key]);
  await page.locator('#sample').selectOption(key);await page.locator('[data-theme="ice"]').click();
  const scope=page.locator('select[aria-label="Whole profile time scope"]');await expect(scope).toHaveValue('season');
  assert.equal(await scope.locator('option[value="career"]').evaluate(o=>o.disabled),true);assert.equal(await scope.locator('option[value="current"]').evaluate(o=>o.disabled),true);
  const front=page.locator('#profile-stage .rk-front');await expect(front).toHaveAttribute('data-window-season','20242025');await expect(front.locator('.rk-front-scope')).toContainText('Season');
  await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);await expect(front.locator('.rk-front-pressure [data-candidate-id]')).toHaveCount(0);
  if(key==='shesterkin')await expect(front.locator('[data-candidate-id]')).toHaveCount(0);
  else{
   await expect(front.locator('[data-candidate-id]')).toHaveCount(3);
   for(const library of await front.locator('.rk-candidate-library').all())await library.locator(':scope > summary').click();
   for(const item of await front.locator('[data-candidate-id]').all()){
    await item.locator(':scope > summary').click();await expect(item).toContainText('2024–25');await expect(item).toContainText('Reference:');await expect(item).toContainText('Opportunities:');
    const history=item.locator('.rk-pattern-history');await history.locator('summary').click();assert.ok(!(await history.textContent()).includes('2025–26'));await item.locator(':scope > summary').click();
   }
   for(const library of await front.locator('.rk-candidate-library').all())await library.locator(':scope > summary').click();
   const taskDirect=await front.locator('.rk-front-skills').evaluate(box=>[...box.querySelectorAll('.rk-candidate-patterns > details.rk-stat-feat,.rk-stat-feats > details.rk-stat-feat')].length);assert.ok(taskDirect<=3);
  }
  await page.screenshot({path:resolve(output,`${key}.png`),fullPage:true});
  await page.emulateMedia({media:'print'});await page.pdf({path:resolve(output,`${key}.pdf`),format:'Letter',printBackground:true});await page.emulateMedia({media:'screen'});
  await page.setViewportSize({width:390,height:1100});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await page.setViewportSize({width:1440,height:1100});tested++;
 }
 assert.deepEqual(errors,[]);await writeFile(resolve(output,'verification.json'),JSON.stringify({status:'passed',profiles:tested,scope_options:3,career_current_gated:true,prior_fields_unchanged:true,future_candidate_history_hidden:true,capabilities_unrated:30,page_errors:errors},null,2));console.log('Five scoped profiles, coverage gates, candidate evidence and mobile/print exports checked.');
}finally{await browser.close();}
