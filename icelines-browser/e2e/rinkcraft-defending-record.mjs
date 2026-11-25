import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_DEFENDING_CARD_URL??'http://127.0.0.1:8780/';
const roster=await(await fetch(new URL('players.json',base))).json();
const output=new URL('../../target/defending-card-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:process.env.ICELINES_BROWSER_CHANNEL??'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 const names=['Adam Fox','Jaccob Slavin','Matty Beniers'];
 for(const [i,name] of names.entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json();
  assert.equal(p.player_name,name);
  const stats=p.stat_attribute_seasons.find(s=>s.season===20262027),r=stats.skater_context.defending;assert.ok(r);
  await page.goto(new URL(`?player=${id}`,base).href);const record=page.locator('.rk-response-row .rk-defense-record');await record.waitFor();
  assert.match(await record.textContent(),/Chance Control.*Defensive Work/);
  assert.ok((await record.textContent()).includes(r.activity));assert.ok((await record.textContent()).includes(r.chances));
  assert.equal(await record.getAttribute('data-defense-coverage'),name==='Matty Beniers'?'unavailable':'measured');
  await record.locator('summary').click();assert.deepEqual(await record.locator('dd').allTextContents(),r.rows.map(r=>r.value));await record.locator('summary').click();
  await page.locator('.rk-front-values .rk-capability-trigger').nth(5).click();
  assert.ok((await page.locator('dialog .rk-defense-record').textContent()).includes(r.activity));
  await page.getByRole('button',{name:'Close',exact:true}).click();
  const attached=await page.evaluate(async({p,stats})=>{
   const w=new Worker('/src/worker.js',{type:'module'});
   try{return await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('Worker timeout')),60000);
    w.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};w.onmessage=e=>{clearTimeout(timer);e.data.error?reject(new Error(e.data.error.message)):resolve(e.data.value);};
    w.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'statAttributesForProfile',payload:{catalog:{schema:'rinkcraft.stat-attribute-catalog.v1',profiles:[stats]},profile:p}});
   });}finally{w.terminate();}
  },{p,stats});
  assert.deepEqual(attached.stat_attribute_seasons.find(s=>s.season===20262027).skater_context.defending,r);
  await page.evaluate(()=>scrollTo(0,0));if(i===0)await page.screenshot({path:fileURLToPath(new URL('desktop.png',output)),fullPage:true});
  await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});
  const pdf=await page.pdf({path:fileURLToPath(new URL(`defending-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);await page.emulateMedia({media:'screen'});
 }
 // An older profile without the optional record retains its original fallback.
 await page.route('**/profiles/*.json',async route=>{const p=await(await route.fetch()).json();for(const s of p.stat_attribute_seasons??[])if(s.skater_context)delete s.skater_context.defending;await route.fulfill({json:p});});
 await page.reload();await page.locator('.rk-front').waitFor();assert.equal(await page.locator('.rk-response-row .rk-defense-record').count(),0);
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({players:names,workerParity:true,missingWindow:true,legacyFallback:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));
 console.log('Passed three cards, worker parity, missing/legacy records, dialogs, mobile and one-page print.');
}finally{await browser.close();}
