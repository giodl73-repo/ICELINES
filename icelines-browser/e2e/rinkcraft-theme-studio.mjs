import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const output=resolve('target/compact-sheet/theme-studio-check');await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];
page.on('pageerror',e=>errors.push(e.message));
const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const themes=['ice','ledger','bureau','tactics','broadcast','editorial'],samples=['design-defense','design-goalie','fox','mcdavid','barkov','makar','shesterkin'];
const results=[];
try{
 await page.goto(base);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
 await expect(page.locator('.theme-choice')).toHaveCount(6);await expect(page.locator('#sample option')).toHaveCount(7);
 for(const sample of samples){
  await page.locator('#sample').selectOption(sample);
  const front=page.locator('#profile-stage .rk-front');await expect(front).toHaveCount(1);
  const original=await front.textContent();
  await expect(front.locator('[data-capability-index]')).toHaveCount(6);
  await expect(front).toHaveAttribute('data-profile-kind',sample.startsWith('design-')?'fictional_design_sample':'nhl_evidence');
  await expect(front.locator('[data-rating-state="'+(sample.startsWith('design-')?'illustrative':'unassessed')+'"]')).toHaveCount(6);
  for(const theme of themes){
   await page.locator(`.theme-choice[data-theme="${theme}"]`).click();
   await expect(page.locator('html')).toHaveAttribute('data-rinkcraft-theme',theme);
   assert.equal(await front.textContent(),original,'Themes must preserve all profile content');
   assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   const positions=await front.locator('[data-capability-index]').evaluateAll(es=>es.map(e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};}));
   assert.equal(new Set(positions.map(p=>Math.round(p.y))).size,theme==='tactics'?1:6);
   if(sample==='design-defense')await page.screenshot({path:resolve(output,theme+'-desktop.png'),fullPage:true});
   results.push({sample,theme,content_unchanged:true,horizontal_attributes:theme==='tactics'});
  }
 }
 // Actual differences include type, panel treatment and attribute geometry.
 await page.locator('#sample').selectOption('design-defense');
 await page.locator('.theme-choice[data-theme="ice"]').click();await page.locator('#next-theme').click();await expect(page.locator('html')).toHaveAttribute('data-rinkcraft-theme','ledger');
 await page.locator('#previous-theme').click();await expect(page.locator('html')).toHaveAttribute('data-rinkcraft-theme','ice');
 await page.locator('#theme-dial').fill('3');await page.locator('#theme-dial').dispatchEvent('input');await expect(page.locator('html')).toHaveAttribute('data-rinkcraft-theme','tactics');
 await page.locator('body').click({position:{x:2,y:2}});await page.keyboard.press('ArrowRight');await expect(page.locator('html')).toHaveAttribute('data-rinkcraft-theme','broadcast');
 await page.reload();await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');await expect(page.locator('html')).toHaveAttribute('data-rinkcraft-theme','broadcast');await expect(page.locator('#sample')).toHaveValue('design-defense');
 await page.setViewportSize({width:390,height:844});
 for(const sample of ['design-defense','design-goalie','fox']){
  await page.locator('#sample').selectOption(sample);
  for(const theme of themes){await page.locator(`.theme-choice[data-theme="${theme}"]`).click();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),sample+' '+theme+' mobile');}
 }
 await page.locator('#sample').selectOption('design-defense');await page.locator('.theme-choice[data-theme="ledger"]').click();await page.screenshot({path:resolve(output,'ledger-mobile.png'),fullPage:true});
 await page.setViewportSize({width:1440,height:1100});
 for(const theme of themes){await page.locator(`.theme-choice[data-theme="${theme}"]`).click();await page.pdf({path:resolve(output,theme+'.pdf'),format:'Letter',printBackground:true});}
 assert.deepEqual(errors,[]);
 await writeFile(resolve(output,'verification.json'),JSON.stringify({status:'passed',desktop_cases:results,mobile_cases:18,controls:['buttons','previous/next','range','keyboard','URL reload'],print_exports:6,page_errors:errors},null,2));
 console.log('Theme studio: 42 desktop / 18 mobile cases, controls, URL state and six print exports passed.');
}finally{await browser.close();}
