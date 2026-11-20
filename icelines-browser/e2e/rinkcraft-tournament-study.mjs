import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const url=process.env.ICELINES_TOURNAMENT_STUDY_URL??'http://127.0.0.1:8784/';
const out=process.env.ICELINES_TOURNAMENT_CHECK_DIR??'target/tournament-study-ui';
await mkdir(out,{recursive:true});const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(String(e)));
 const response=await page.request.get(new URL('comparison.json',url).href);expect(response.ok()).toBeTruthy();const r=await response.json();
 await page.goto(url,{waitUntil:'networkidle'});await expect(page.getByRole('heading',{name:'Tournament rules trial',exact:true})).toBeVisible();
 const title=page.getByRole('heading',{name:'Title frequency by rule',exact:true}).locator('..');
 await expect(title.getByRole('row')).toHaveCount(r.variants.length+1);
 const coverage=page.getByRole('heading',{name:'Attribute investment the model actually tests',exact:true}).locator('..');
 await expect(coverage.getByRole('row')).toHaveCount(1+r.variants.reduce((n,[,result])=>n+result.active_teams.length,0));
 for(const[v]of r.variants){await expect(page.getByRole('link',{name:v.label,exact:true})).toHaveAttribute('href',`${v.id}/`);}
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBeTruthy();await page.screenshot({path:`${out}/desktop.png`,fullPage:false});
 await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBeTruthy();await page.screenshot({path:`${out}/mobile.png`,fullPage:false});
 const[v]=r.variants.find(([v])=>v.id==='combined');await page.getByRole('link',{name:v.label,exact:true}).click();await expect(page.getByRole('heading',{name:'Rules 0.2 tournament',exact:true})).toBeVisible();
 expect(errors).toEqual([]);await writeFile(`${out}/verification.json`,JSON.stringify({url,variants:r.variants.length,desktop:'pass',mobile390:'pass',variantLinks:'pass',combinedPage:'pass',pageErrors:errors},null,2));
 console.log(`Rule comparison UI passed: ${r.variants.length} variants, desktop/mobile390, linked combined page`);
}finally{await browser.close();}
