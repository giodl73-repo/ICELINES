import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
const out=resolve('target/compact-sheet/observation-cards');await mkdir(out,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const themes=['ice','ledger','bureau','tactics','broadcast','editorial'];
let desktop=0,mobile=0;
try{
 for(const sample of ['mcdavid','barkov','fox','makar','shesterkin']){
  await page.setViewportSize({width:1440,height:1100});await page.goto(`${base}?sample=${sample}&scope=season`);
  await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
  const front=page.locator('#profile-stage .rk-front');
  await expect(front.locator('.rk-profile-observations')).toHaveCount(sample==='shesterkin'?0:1);
  if(sample!=='shesterkin'){
   await expect(front.locator('.rk-front-limitations h4')).toHaveText('Cautions & Low Metrics');
   await expect(front.locator('.rk-observation-gaps>summary')).toBeVisible();
   await front.locator('.rk-observation-gaps>summary').click();
   await expect(front.locator('.rk-observation-gaps')).toContainText('Unmeasured here; not player weaknesses.');
   await front.locator('.rk-observation-gaps>summary').click();
  }
  if(sample==='mcdavid'){
   await expect(front.locator('.rk-observation-counts')).toHaveText('17 positive · 4 caution · 1 low');
   const low=front.locator('[data-observation-id="low-blocks"]');await expect(low.locator('summary')).toContainText('P7.3');
   await low.locator('summary').click();await expect(low).toContainText('Not proof of poor defending');await expect(low).toContainText('Analysis ');
   await low.locator('summary').click();
  }
  const original=await front.textContent();
  for(const theme of themes){
   await page.locator(`.theme-choice[data-theme="${theme}"]`).click();
   assert.equal(await front.textContent(),original);
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);desktop++;
  }
  await page.locator('.theme-choice[data-theme="ice"]').click();
  const pdf=await page.pdf({path:resolve(out,`${sample}.pdf`),preferCSSPageSize:true,printBackground:true});
  assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,sample+' Letter sheet');
  if(sample==='mcdavid')await page.screenshot({path:resolve(out,'mcdavid-desktop.png'),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  for(const theme of themes){await page.locator(`.theme-choice[data-theme="${theme}"]`).click();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));mobile++;}
  if(sample==='mcdavid'){await page.locator('.theme-choice[data-theme="ice"]').click();await page.screenshot({path:resolve(out,'mcdavid-mobile.png'),fullPage:true});}
  for(const scope of ['current','career']){
   await page.locator('[aria-label="Whole profile time scope"] select,select[aria-label="Whole profile time scope"]').selectOption(scope);
   await expect(front.locator('.rk-profile-observations')).toHaveCount(0);
  }
 }
 assert.deepEqual(errors,[]);
 await writeFile(resolve(out,'browser-verification.json'),JSON.stringify({desktop,mobile,scope_absence_checks:10,one_page_letter_exports:5,page_errors:errors},null,2));
 console.log('Observation cards: 30 desktop, 30 mobile, 10 scope-isolation checks, five single-page Letter exports passed.');
}finally{await browser.close();}
