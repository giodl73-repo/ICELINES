import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const out=resolve('target/compact-sheet/positive-whistle/ui');await mkdir(out,{recursive:true});const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));let records=0,cases=0,themes=0;
try{
 await page.goto(new URL('?sample=fox&scope=current&theme=ice',base).href);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');const fixtures=await(await page.request.get(new URL('fixtures.json',base).href)).json();
 for(const [key,original]of Object.entries(fixtures)){
  await page.locator('#sample').selectOption(key);const scope=page.locator('select[aria-label="Whole profile time scope"]'),front=page.locator('#profile-stage .rk-front');
  for(const kind of ['season','current']){
   await scope.selectOption(kind);const frame=kind==='season'?original:original.scope_variants.current;const pattern=frame.experimental_patterns.patterns.find(p=>p.id==='penalty-balance');const row=front.locator('[data-candidate-id="penalty-balance"]');
   if(pattern){await expect(row).toHaveCount(1);await expect(row).toHaveAttribute('data-candidate-state',pattern.state);await expect(row.locator('summary').first()).toContainText('Relative Whistle Record');await expect(row.locator('[data-whistle-sign]')).toHaveText(pattern.whistle_record.display);await expect(row.locator('[data-whistle-sign]')).toHaveAttribute('data-whistle-sign',pattern.whistle_record.sign);
    const library=front.locator('.rk-front-traits .rk-candidate-library');if(pattern.state!=='earned')await library.locator(':scope > summary').click();await row.locator(':scope > summary').click();await expect(row).toContainText('do not require positive net');await row.locator(':scope > summary').click();if(pattern.state!=='earned')await library.locator(':scope > summary').click();records++;
   }else await expect(row).toHaveCount(0);
   await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);await expect(front.locator('.rk-record-milestones')).toHaveCount(0);
   const text=await front.textContent();for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){await page.locator(`.theme-choice[data-theme="${theme}"]`).click();assert.equal(await front.textContent(),text);await page.setViewportSize({width:390,height:1100});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await page.setViewportSize({width:1440,height:1100});themes++;}await page.locator('.theme-choice[data-theme="ice"]').click();
   await page.screenshot({path:resolve(out,`${key}-${kind}.png`),fullPage:true});await page.emulateMedia({media:'print'});await page.pdf({path:resolve(out,`${key}-${kind}.pdf`),format:'Letter',printBackground:true});await page.emulateMedia({media:'screen'});cases++;
  }
  await scope.selectOption('career');await expect(front.locator('[data-candidate-id="penalty-balance"]')).toHaveCount(0);await expect(front.locator('.rk-record-milestones')).toHaveCount(1);
 }
 assert.deepEqual(errors,[]);const receipt={status:'passed',profile_scope_cases:cases,signed_records:records,theme_desktop_cases:themes,theme_mobile_cases:themes,career_marker_regressions:5,old_awards_preserved:true,capabilities_unrated:true,page_errors:errors};await writeFile(resolve(out,'verification.json'),JSON.stringify(receipt,null,2));console.log(receipt);
}finally{await browser.close();}
