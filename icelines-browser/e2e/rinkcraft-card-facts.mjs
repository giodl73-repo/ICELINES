// Verify factual identity fields independently from the fictional design specimens.
import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const output=resolve('target/compact-sheet/card-completion');await mkdir(output,{recursive:true});
const base=process.env.ICELINES_THEME_URL??'http://127.0.0.1:8771/theme-studio/';
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];
page.on('pageerror',e=>errors.push(e.message));let facts=0;
try{
 await page.goto(base);await expect(page.locator('[data-theme-studio]')).toHaveAttribute('data-ready','true');
 const response=await page.request.get(new URL('fixtures.json',base).href);assert.ok(response.ok());const fixtures=await response.json();
 for(const [key,p]of Object.entries(fixtures)){
  assert.equal(p.character_sheet.card_facts.length,2,'This acceptance requires the five completed factual pilots.');
  await page.locator('#sample').selectOption(key);await page.locator('.theme-choice[data-theme="ice"]').click();
  const front=page.locator('#profile-stage .rk-front');
  await expect(front.locator('[data-rating-state="unassessed"]')).toHaveCount(6);
  for(const fact of p.character_sheet.card_facts){
   const cell=front.locator(`[data-fact-field="${fact.field}"]`);
   await expect(cell.locator('dd')).toHaveText(fact.name);await expect(cell.locator(':scope > small')).toHaveText(fact.context);
   const evidence=cell.locator('details');await evidence.locator('summary').click();
   for(const id of fact.source_ids){const s=p.character_sheet.sources.find(s=>s.id===id);await expect(evidence.locator('a').filter({hasText:id})).toHaveAttribute('href',s.url);await expect(evidence).toContainText(s.published);}
   await evidence.locator('summary').click();facts++;
  }
  await page.screenshot({path:resolve(output,key+'-facts.png'),fullPage:true});
  await page.pdf({path:resolve(output,key+'-facts.pdf'),format:'Letter',printBackground:true});
  await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),key+' mobile overflow');
  for(const cell of await front.locator('[data-fact-field]').all()){await cell.locator('summary').click();assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await cell.locator('summary').click();}
  await page.setViewportSize({width:1440,height:1100});
 }
 assert.equal(facts,10);assert.deepEqual(errors,[]);
 await writeFile(resolve(output,'fact-ui-verification.json'),JSON.stringify({status:'passed',players:5,facts,source_disclosures:10,mobile_cases:5,capabilities_unrated:30,print_exports:5,page_errors:errors},null,2));
 console.log('Ten factual fields and sources, five mobile cards, thirty unrated attributes and five print exports passed.');
}finally{await browser.close();}
