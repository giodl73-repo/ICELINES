import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_CARRYOVER_URL??'http://127.0.0.1:8780/calibration/history/';
const data=await(await fetch(new URL('carryover.json',base))).json();
const prefix=data.comparisons[0].attribute==='Defending'?'defending-':'';
const output=new URL('../../target/attribute-calibration/history-screenshots/',import.meta.url);
await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:process.env.ICELINES_BROWSER_CHANNEL??'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1000}}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await page.goto(base);await page.locator('#rows tr').first().waitFor();
 const pairs=[...new Set(data.comparisons.map(c=>c.from))];
 assert.equal(await page.locator('#pair option').count(),pairs.length);
 for(const from of pairs)for(const sample of [10,40])for(const attribute of ['',...new Set(data.comparisons.map(c=>c.attribute))])for(const group of ['','Forwards','Defense']){
  await page.selectOption('#pair',String(from));await page.selectOption('#sample',String(sample));await page.selectOption('#attribute',attribute);await page.selectOption('#group',group);
  const rows=data.comparisons.filter(c=>c.from===from&&c.min_games===sample&&(!attribute||c.attribute===attribute)&&(!group||c.group===group));
  assert.equal(await page.locator('#rows tr').count(),rows.length);
  const fmt=v=>v===null?'—':v.toFixed(3);
  for(const [i,r] of rows.entries()){
   const cells=await page.locator('#rows tr').nth(i).locator('td').allTextContents();
   if(r.attribute==='Defending'){assert.match(cells[0],/5v5 minutes/);assert.equal(r.outcome_higher_is_better,false);}
   assert.equal(cells[2],`${fmt(r.baseline_repeatability)} → ${fmt(r.candidate_repeatability)}`);
   assert.equal(cells[3],`${fmt(r.baseline_next_production)} → ${fmt(r.candidate_next_production)}`);
  }
 }
 await page.selectOption('#attribute',data.comparisons[0].attribute);await page.selectOption('#group','');
 await page.screenshot({path:fileURLToPath(new URL(`${prefix}desktop.png`,output))});
 await page.setViewportSize({width:390,height:844});
 assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 await page.screenshot({path:fileURLToPath(new URL(`${prefix}mobile.png`,output))});
 assert.deepEqual(errors,[]);
 console.log(`Passed ${pairs.length} season pairs, both sample filters, all attribute/position filters, displayed correlations and mobile overflow.`);
}finally{await browser.close();}
