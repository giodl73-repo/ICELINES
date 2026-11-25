import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_TASK_CARD_URL??'http://127.0.0.1:8780/';
const roster=await(await fetch(new URL('players.json',base))).json();
const output=new URL('../../target/task-repertoire-study/',import.meta.url);await mkdir(output,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[],results=[];page.on('pageerror',e=>errors.push(e.message));
 for(const [i,name]of ['Nathan MacKinnon','Sebastian Aho','Adam Fox','Alex Killorn'].entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json(),a=p.achievements,r=a.task_repertoire;assert.ok(r);
  await page.goto(new URL(`?player=${id}`,base).href);const box=page.locator('.rk-front-skills .rk-task-repertoire');await box.waitFor();
  assert.deepEqual(await box.locator(':scope > .rk-task-feat').evaluateAll(es=>es.map(e=>e.dataset.featId)),r.skills.map(s=>s.feat_id));
  assert.deepEqual(await box.locator(':scope > .rk-task-feat > summary > strong').allTextContents(),r.skills.map(s=>s.name));
  assert.equal(await box.locator(':scope > .rk-task-feat .rk-task-measured').count(),r.skills.length);
  assert.equal(await box.locator('.rk-stat-feat').count(),a.feats.filter(f=>f.placements.includes('task_skills')).length);
  assert.ok(r.skills.every(s=>a.feats.some(f=>f.id===s.feat_id&&f.state==='earned')));
  const projected=await page.evaluate(async a=>{const {default:init,BrowserEngine}=await import('/pkg/icelines_wasm.js');await init();const engine=new BrowserEngine();try{return JSON.parse(engine.task_repertoire(JSON.stringify(a)));}finally{engine.free();}},a);assert.deepEqual(projected,r);
  if(r.skills.length){await box.locator(':scope > .rk-task-feat > summary').first().click();await box.locator(':scope > .rk-task-feat button').first().click();await page.getByRole('dialog').waitFor();assert.match(await page.getByRole('dialog').textContent(),/Your record/);await page.getByRole('button',{name:'Close rulebook',exact:true}).click();await box.locator(':scope > .rk-task-feat > summary').first().click();}
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('desktop.png',output)),fullPage:true});
  await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);if(i===0)await page.screenshot({path:fileURLToPath(new URL('mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});const pdf=await page.pdf({path:fileURLToPath(new URL(`task-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);await page.emulateMedia({media:'screen'});
  results.push({id,name,tasks:r.skills.map(s=>s.name)});
 }
 assert.ok(new Set(results.map(p=>p.tasks.join('/'))).size>=3,'Sample repertoires should differ');assert.deepEqual(errors,[]);
 await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({results,wasmParity:true,libraryPreserved:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));console.log('Passed four distinct task repertoires, full library, rulebook, WASM parity, mobile and print.');
}finally{await browser.close();}
