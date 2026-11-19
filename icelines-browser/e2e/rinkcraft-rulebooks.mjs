import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const base=process.env.ICELINES_PREVIEW_URL??'http://127.0.0.1:8772/';
const out=resolve(process.env.ICELINES_RULEBOOK_EVIDENCE??'target/compact-sheet/rulebooks');await mkdir(out,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
let recipes=0;
try{
 for(const [id,names]of [[8473986,['Skating','Handling','Passing','Shooting','Reading','Defending']],[8478048,['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play']]]){
  await page.goto(`${base}?season=20242025&player=${id}`);await expect(page.locator('.rk-front')).toBeVisible();
  const before=await page.locator('.rk-front').textContent();
  for(const name of names){
   const trigger=page.getByRole('button',{name:`How ${name} is scored`,exact:true});await trigger.click();const dialog=page.locator('.rk-capability-dialog');await expect(dialog).toBeVisible();await expect(dialog.locator('h2')).toHaveText(name);
   await expect(dialog.locator('.rk-capability-grade')).toHaveText('— / 20 · Unrated');
   await expect(dialog).toContainText('How the score is built');await expect(dialog.locator('.rk-capability-facets>section')).toHaveCount(name==='Reading'||id===8478048?3:4);
   await dialog.locator('.rk-capability-recipe>details>summary').click();await expect(dialog).toContainText('Balanced index H');await expect(dialog).toContainText('does not certify the measurements or release a player grade');
   if(name==='Skating')await page.screenshot({path:resolve(out,'skating-recipe.png'),fullPage:true});
   if(name==='Puck Play'){await page.setViewportSize({width:390,height:844});assert.ok(await dialog.evaluate(d=>d.scrollWidth<=d.clientWidth));await page.screenshot({path:resolve(out,'goalie-puck-mobile.png'),fullPage:true});await page.setViewportSize({width:1440,height:1100});}
   await dialog.getByRole('button',{name:'Close rulebook'}).click();await expect(page.locator('.rk-capability-dialog')).toHaveCount(0);assert.equal(await page.locator('.rk-front').textContent(),before);recipes++;
  }
 }
 await page.goto(`${base}?season=20242025&player=8473986`);await expect(page.locator('.rk-front h3')).toHaveText('Alex Killorn');
 const feat=page.locator('[data-feat-id="goal-output"]').first();await feat.locator(':scope>summary').click();
 await feat.getByRole('button',{name:'Open feat rulebook'}).click();const featDialog=page.locator('.rk-feat-dialog');await expect(featDialog).toContainText('Goals / GP ≥ P60');await expect(featDialog).toContainText('19 ÷ 82 = 0.232 / GP');await expect(featDialog).toContainText('Earned Bronze');await expect(featDialog).toContainText('Task Skills');
 await page.screenshot({path:resolve(out,'killorn-feat-formula.png'),fullPage:true});
 await featDialog.getByRole('button',{name:'Close rulebook'}).click();
 const flag=page.locator('.rk-front-limitations [data-feat-id="costly-events"]').first();await flag.locator(':scope>summary').click();await flag.getByRole('button',{name:'Open feat rulebook'}).click();await expect(page.locator('.rk-feat-dialog')).toContainText('Caution Flags');await expect(page.locator('.rk-feat-dialog')).toContainText('Flag bands');await page.locator('.rk-feat-dialog').getByRole('button',{name:'Close rulebook'}).click();
 for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){await page.locator('#theme').selectOption(theme);await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.setViewportSize({width:1440,height:1100});}
 await page.locator('#theme').selectOption('ice');const pdf=await page.pdf({path:resolve(out,'expanded-feat-letter.pdf'),preferCSSPageSize:true,printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);
 assert.deepEqual(errors,[]);await writeFile(resolve(out,'verification.json'),JSON.stringify({attribute_recipes:recipes,capability_grades_unchanged:true,feat_arithmetic:true,caution_rule:true,mobile_themes:6,expanded_feat_print_pages:1,page_errors:errors},null,2));console.log('12 capability recipes, feat arithmetic/category/tier, caution rules, six mobile themes and one-page print passed.');
}finally{await browser.close();}
