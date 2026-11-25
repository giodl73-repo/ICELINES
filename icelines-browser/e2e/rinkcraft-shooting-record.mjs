import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_SHOOTING_CARD_URL??'http://127.0.0.1:8780/';
const roster=await(await fetch(new URL('players.json',base))).json();
const output=new URL('../../target/shooting-card-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:process.env.ICELINES_BROWSER_CHANNEL??'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 const names=['Nathan MacKinnon','Sebastian Aho','Adam Fox','Alex Killorn'];
 for(const [i,name]of names.entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json(),stats=p.stat_attribute_seasons.find(s=>s.season===20262027),r=stats.skater_context.shooting;assert.ok(r);
  await page.goto(new URL(`?player=${id}`,base).href);const summary=page.locator('.rk-front-values .rk-shooting-summary');await summary.waitFor();assert.deepEqual(await summary.locator('span').allTextContents(),[`Shots ${r.shots}`,`Conversion ${r.conversion}`]);
  assert.equal(await page.locator('.rk-front-values [data-capability-index="3"] dd').textContent(),String(stats.scores[3].score));assert.equal(await page.locator('.rk-passing-summary').count(),1);assert.equal(await page.locator('.rk-reading-summary').count(),1);assert.equal(await page.locator('.rk-handling-summary').count(),1);assert.equal(await page.locator('.rk-response-row .rk-defense-record > strong').textContent(),stats.skater_context.defending.chances);
  await page.locator('.rk-front-values .rk-capability-trigger').nth(3).click();const detail=page.locator('dialog .rk-shooting-record');await detail.waitFor();assert.deepEqual(await detail.locator('dd').allTextContents(),r.rows.map(r=>r.value));assert.ok((await detail.textContent()).includes(r.formula));assert.match(await detail.textContent(),/not established finishing talent/);for(const note of r.source_notes)assert.ok((await detail.textContent()).includes(note));if(i===0)await page.screenshot({path:fileURLToPath(new URL('detail.png',output))});await page.getByRole('button',{name:'Close',exact:true}).click();
  const attached=await page.evaluate(async({p,stats})=>{const worker=new Worker('/src/worker.js',{type:'module'});try{return await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('Worker timeout')),60000);worker.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};worker.onmessage=e=>{clearTimeout(timer);e.data.error?reject(new Error(e.data.error.message)):resolve(e.data.value);};worker.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'statAttributesForProfile',payload:{catalog:{schema:'rinkcraft.stat-attribute-catalog.v1',profiles:[stats]},profile:p}});});}finally{worker.terminate();}},{p,stats});
  const actual=attached.stat_attribute_seasons.find(s=>s.season===20262027).skater_context.shooting;assert.deepEqual({...actual,minutes:null},{...r,minutes:null});assert.ok(Math.abs(actual.minutes-r.minutes)<=1e-9);
  await page.evaluate(()=>scrollTo(0,0));if(i===0)await page.screenshot({path:fileURLToPath(new URL('desktop.png',output)),fullPage:true});await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);if(i===0)await page.screenshot({path:fileURLToPath(new URL('mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});const pdf=await page.pdf({path:fileURLToPath(new URL(`shooting-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);await page.emulateMedia({media:'screen'});
 }
 await page.route('**/profiles/*.json',async route=>{const p=await(await route.fetch()).json();for(const stats of p.stat_attribute_seasons??[])if(stats.skater_context)delete stats.skater_context.shooting;await route.fulfill({json:p});});await page.reload();await page.locator('.rk-front').waitFor();assert.equal(await page.locator('.rk-shooting-summary').count(),0);
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({players:names,workerParity:true,legacyFallback:true,priorRecordsPreserved:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));console.log('Passed four Shooting cards, generation/finishing detail, unchanged grades, prior records, worker parity, legacy fallback, mobile and print.');
}finally{await browser.close();}
