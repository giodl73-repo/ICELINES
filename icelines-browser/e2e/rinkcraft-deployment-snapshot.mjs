import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_DEPLOYMENT_CARD_URL??'http://127.0.0.1:8780/';
const roster=await(await fetch(new URL('players.json',base))).json(),output=new URL('../../target/deployment-card-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[],results=[];page.on('pageerror',e=>errors.push(e.message));
 for(const [i,name]of ['Alex Killorn','Nathan MacKinnon','Adam Fox','Sebastian Aho'].entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json(),d=p.deployment_evidence,stats=p.stat_attribute_seasons.find(s=>s.season===20262027);assert.ok(d);
  await page.goto(new URL(`?player=${id}`,base).href);const usage=page.locator('.rk-front-deployment .rk-usage');await usage.waitFor();assert.deepEqual(await usage.locator('.rk-usage-grid dd').allTextContents(),d.rows.map(r=>r.per_game_display));
  assert.match(await usage.textContent(),/Unit \/ station undocumented/);const pp=d.rows.find(r=>r.key==='pp'),pk=d.rows.find(r=>r.key==='sh');assert.ok((await page.locator('.rk-front-role-coverage').textContent()).includes(`PP ${pp.per_game_display} / SH ${pk.per_game_display} per GP`));
  assert.deepEqual(await page.locator('.rk-task-repertoire > .rk-task-feat').evaluateAll(es=>es.map(e=>e.dataset.featId)),p.achievements.task_repertoire.skills.map(s=>s.feat_id));
  const actual=await page.evaluate(async({p,stats})=>{const worker=new Worker('/src/worker.js',{type:'module'});try{return await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('Worker timeout')),60000);worker.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};worker.onmessage=e=>{clearTimeout(timer);e.data.error?reject(new Error(e.data.error.message)):resolve(e.data.value.deployment_evidence);};worker.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'statAttributesForProfile',payload:{catalog:{schema:'rinkcraft.stat-attribute-catalog.v1',profiles:[stats]},profile:{...p,deployment_evidence:null}}});});}finally{worker.terminate();}},{p,stats});assert.deepEqual(actual,d);
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('desktop.png',output)),fullPage:true});await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);if(i===0)await page.screenshot({path:fileURLToPath(new URL('mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});const pdf=await page.pdf({path:fileURLToPath(new URL(`deployment-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);await page.emulateMedia({media:'screen'});results.push({id,name,pp:pp.per_game_display,pk:pk.per_game_display});
 }
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({results,workerFallbackParity:true,tasksPreserved:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));console.log('Passed four deployment cards, Role Coverage, recovered worker usage, retained task repertoires, mobile and print.');
}finally{await browser.close();}
