// Prepared local skater preview with real Rust-projected records.
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_SKATER_CONTEXT_URL??'http://127.0.0.1:8780/';
const output=new URL('../../target/skater-attribute-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:process.env.ICELINES_BROWSER_CHANNEL??'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 const ids=[8478402,8478427,8479323];
 for(const [i,id] of ids.entries()){
  const p=await(await fetch(new URL(`profiles/${id}.json`,base))).json();
  const stats=p.stat_attribute_seasons.find(s=>s.season===20262027),context=stats.skater_context;assert.ok(context);
  await page.goto(new URL(`?player=${id}`,base).href);const block=page.locator('.rk-front .rk-skater-context');await block.waitFor();
  assert.match(await block.textContent(),/Attribute Record.*Primary assists.*Secondary assists/);
  await block.locator('summary').click();assert.match(await block.textContent(),/Even-strength assists.*Power-play assists/);await block.locator('summary').click();
  for(const [index,score] of stats.scores.entries()){
   await page.locator('.rk-front-values .rk-capability-trigger').nth(index).click();const dialog=page.locator('dialog');
   assert.deepEqual(await dialog.locator('.rk-skater-context > dl dd').allTextContents(),context.rows.filter(r=>r.attribute===score.label).map(r=>r.value));
   await dialog.getByRole('button',{name:'Close',exact:true}).click();
  }
  const attached=await page.evaluate(async({p,stats})=>{
   const w=new Worker('/src/worker.js',{type:'module'});
   try{return await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('Worker timeout')),60000);
    w.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};w.onmessage=e=>{clearTimeout(timer);e.data.error?reject(new Error(e.data.error.message)):resolve(e.data.value);};
    w.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'statAttributesForProfile',payload:{catalog:{schema:'rinkcraft.stat-attribute-catalog.v1',profiles:[stats]},profile:p}});
   });}finally{w.terminate();}
  },{p,stats});
  assert.deepEqual(attached.stat_attribute_seasons.find(s=>s.season===20262027).skater_context.rows,context.rows);
  await page.evaluate(()=>scrollTo(0,0));if(i===0)await page.screenshot({path:fileURLToPath(new URL('desktop.png',output)),fullPage:true});
  await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.evaluate(()=>scrollTo(0,0));if(i===0)await page.screenshot({path:fileURLToPath(new URL('mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1100});await page.emulateMedia({media:'print'});
  const pdf=await page.pdf({path:fileURLToPath(new URL(`skater-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);
  await page.emulateMedia({media:'screen'});
 }
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({players:ids,attributeDialogs:18,workerParity:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));
 console.log('Passed three skaters, 18 attribute dialogs, worker parity, mobile and one-page print.');
}finally{await browser.close();}
