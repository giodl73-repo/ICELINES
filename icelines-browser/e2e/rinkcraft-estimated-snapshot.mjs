import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_ESTIMATED_CARD_URL??'http://127.0.0.1:8780/';
const roster=await(await fetch(new URL('players.json',base))).json(),output=new URL('../../target/estimated-card-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[],results=[];page.on('pageerror',e=>errors.push(e.message));
 for(const [i,name]of ['Alex Killorn','Nathan MacKinnon','Adam Fox','Sebastian Aho','Matty Beniers'].entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json(),stats=p.stat_attribute_seasons.find(s=>s.season===20262027),r=stats.estimated_record;assert.ok(r);
  await page.goto(new URL(`?player=${id}`,base).href);const box=page.locator('.rk-front-record');await box.waitFor();assert.ok((await box.textContent()).includes(r.basis));assert.doesNotMatch(await box.textContent(),/Unknown/);
  const expected=r.metrics.map(m=>m.value===null?'—':m.projected_total?Math.round(m.value).toLocaleString('en-US'):m.unit==='Pct'?`${(m.value*100).toFixed(2)}%`:m.value.toFixed(2));assert.deepEqual(await box.locator('.rk-record-values dd').allTextContents(),expected);
  assert.deepEqual(await box.locator('.rk-record-values .rk-field > small').allTextContents(),r.metrics.map(m=>m.per_game===null?(m.value===null?'Unavailable':'Current rate'):`${m.per_game.toFixed(2)} /game`));
  assert.equal(await page.locator('.rk-usage').count(),1);assert.deepEqual(await page.locator('.rk-task-repertoire > .rk-task-feat').evaluateAll(es=>es.map(e=>e.dataset.featId)),p.achievements.task_repertoire.skills.map(s=>s.feat_id));
  const actual=await page.evaluate(async({p,stats})=>{const worker=new Worker('/src/worker.js',{type:'module'});try{return await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('Worker timeout')),60000);worker.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};worker.onmessage=e=>{clearTimeout(timer);e.data.error?reject(new Error(e.data.error.message)):resolve(e.data.value.stat_attribute_seasons.find(s=>s.season===20262027).estimated_record);};worker.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'statAttributesForProfile',payload:{catalog:{schema:'rinkcraft.stat-attribute-catalog.v1',profiles:[{...stats,estimated_record:undefined}]},profile:p}});});}finally{worker.terminate();}},{p,stats});
  assert.equal(actual.basis,r.basis);assert.equal(actual.target_games,r.target_games);for(const [j,m]of r.metrics.entries()){for(const k of ['key','label','unit','projected_total'])assert.equal(actual.metrics[j][k],m[k]);for(const k of ['value','per_game'])m[k]===null?assert.equal(actual.metrics[j][k],null):assert.ok(Math.abs(actual.metrics[j][k]-m[k])<=1e-9);}
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('desktop.png',output)),fullPage:true});await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);if(i===0)await page.screenshot({path:fileURLToPath(new URL('mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});const pdf=await page.pdf({path:fileURLToPath(new URL(`estimated-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);await page.emulateMedia({media:'screen'});results.push({id,name,basis:r.basis,totals:expected});
 }
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({results,workerRecovery:true,priorSectionsPreserved:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));console.log('Passed five projected records, totals/rates, worker recovery from older catalog, retained tasks/workload, mobile and print.');
}finally{await browser.close();}
