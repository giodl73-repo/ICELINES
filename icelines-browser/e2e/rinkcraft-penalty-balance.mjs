import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_PENALTY_BALANCE_URL??'http://127.0.0.1:8780/';
const output=new URL('../../target/penalty-balance-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[],results=[];
 page.on('pageerror',e=>errors.push(e.message));
 for(const [i,id]of [8477492,8482665,8479323,8478427,8473986,8478048].entries()){
  const profile=await(await fetch(new URL(`profiles/${id}.json`,base))).json();
  const stats=profile.stat_attribute_seasons.find(s=>s.season===20262027),r=stats.penalty_balance;
  await page.goto(new URL(`?player=${id}`,base).href);await page.locator('.rk-front-traits').waitFor();
  const panel=page.locator('.rk-front-traits .rk-penalty-balance');
  if(stats.group==='Goalies')assert.equal(await panel.count(),0);
  else{
   assert.ok(r);assert.equal(await panel.locator('summary').textContent(),`Penalty balance ${r.balance}`);
   assert.equal(await panel.getAttribute('open'),null);await panel.locator('summary').click();
   assert.deepEqual(await panel.locator('dd').allTextContents(),[r.drawn,r.taken,r.balance]);
   for(const t of [r.formula,r.note])assert.ok((await panel.textContent()).includes(t));
   assert.equal(await page.locator('.rk-front-methods .rk-shot-method').count(),stats.shot_methods.primary.length);
   if(id===8479323)await panel.screenshot({path:fileURLToPath(new URL('fox-expanded.png',output))});
  }
  const actual=await page.evaluate(async({profile,stats})=>{
   const worker=new Worker('/src/worker.js',{type:'module'});
   try{return await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('Worker timeout')),60000);
    worker.onerror=e=>{clearTimeout(timer);reject(Error(e.message));};
    worker.onmessage=e=>{clearTimeout(timer);e.data.error?reject(Error(e.data.error.message)):resolve(e.data.value.stat_attribute_seasons.find(s=>s.season===20262027).penalty_balance??null);};
    worker.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'statAttributesForProfile',payload:{catalog:{schema:'rinkcraft.stat-attribute-catalog.v1',profiles:[stats]},profile}});
   });}finally{worker.terminate();}
  },{profile,stats});assert.deepEqual(actual,r??null);
  assert.deepEqual(await page.locator('.rk-front-values dd').allTextContents(),stats.scores.map(s=>s.score===null?'—':String(s.score)));
  await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  if(id===8479323)await page.screenshot({path:fileURLToPath(new URL('fox-mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});
  if(r)assert.equal(await panel.isVisible(),false);
  const pdf=await page.pdf({path:fileURLToPath(new URL(`card-${i}.pdf`,output)),format:'Letter',printBackground:true});
  assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);await page.emulateMedia({media:'screen'});
  results.push({id,group:stats.group,penalty_balance:r??null});
 }
 await page.route('**/profiles/*.json',async route=>{const p=await(await route.fetch()).json();for(const s of p.stat_attribute_seasons??[])delete s.penalty_balance;await route.fulfill({json:p});});
 await page.goto(new URL('?player=8479323',base).href);await page.locator('.rk-front-traits').waitFor();assert.equal(await page.locator('.rk-penalty-balance').count(),0);
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({results,workerParity:true,unchangedGrades:true,goalieWithholding:true,legacyFallback:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));
 console.log('Passed six penalty-balance cards, exact Rust values, worker parity, goalie/legacy withholding, mobile and print.');
}finally{await browser.close();}
