import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_STAT_IDENTITY_URL??'http://127.0.0.1:8780/';
const output=new URL('../../target/stat-identity-study/',import.meta.url);await mkdir(output,{recursive:true});
const roster=await(await fetch(new URL('players.json',base))).json();
const names=['Connor McDavid','Nathan MacKinnon','Adam Fox','Matty Beniers','Tye Kartye','Alex Killorn','Pavel Dorofeyev','Braden Schneider','Adam Larsson','Igor Shesterkin','Juuse Saros','Philipp Grubauer'];
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[],results=[];page.on('pageerror',e=>errors.push(e.message));
 for(const [i,name]of names.entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json();
  const a=p.achievements,r=a.task_repertoire;assert.equal(r.schema,'rinkcraft.task-repertoire.v2');
  await page.goto(new URL(`?player=${id}`,base).href);await page.locator('.rk-front').waitFor();
  const specialty=page.locator('.rk-stat-specialties');const highlights=r.highlights??[];
  if(highlights.length){assert.equal(await specialty.count(),1);assert.deepEqual(await specialty.locator('[data-specialty-feat]').allTextContents(),highlights.map(h=>`${h.name} · ${h.tier}`));assert.ok((await specialty.textContent()).includes(`${a.games} GP`));}
  else assert.equal(await specialty.count(),0);
  const identity=await page.locator('.rk-front-identity').textContent();
  if(p.character_sheet){assert.ok(identity.includes(p.character_sheet.primary.name));assert.ok(identity.includes(p.character_sheet.specialty));}else assert.match(identity,/Unscouted|Class dossier unavailable/);
  const tasks=page.locator('.rk-front-skills .rk-task-repertoire');assert.deepEqual(await tasks.locator(':scope > .rk-task-feat').evaluateAll(es=>es.map(e=>e.dataset.featId)),r.skills.map(s=>s.feat_id));
  assert.equal(await tasks.locator('.rk-stat-feat').count(),a.feats.filter(f=>f.placements.includes('task_skills')).length);
  if(name==='Matty Beniers')assert.equal(r.skills[0].feat_id,'puck-thief');
  if(name==='Adam Fox')assert.ok(r.skills.some(s=>s.feat_id==='assist-output'));
  const actual=await page.evaluate(async a=>{const {default:init,BrowserEngine}=await import('/pkg/icelines_wasm.js');await init();const engine=new BrowserEngine();try{return JSON.parse(engine.task_repertoire(JSON.stringify(a)));}finally{engine.free();}},a);
  assert.deepEqual(actual,r);
  if(name==='Matty Beniers'||name==='Adam Fox')await page.screenshot({path:fileURLToPath(new URL(`desktop-${id}.png`,output)),fullPage:true});
  await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});
  const pdf=await page.pdf({path:fileURLToPath(new URL(`card-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,`${name} must print on one page`);await page.emulateMedia({media:'screen'});
  results.push({id,name,highlights,tasks:r.skills.map(s=>s.name)});
 }
 await page.route('**/profiles/*.json',async route=>{const p=await(await route.fetch()).json();const r=p.achievements?.task_repertoire;if(r){delete r.highlights;r.schema='rinkcraft.task-repertoire.v1';}await route.fulfill({json:p});});
 await page.goto(new URL('?player=8479323',base).href);await page.locator('.rk-front').waitFor();assert.equal(await page.locator('.rk-stat-specialties').count(),0);assert.ok(await page.locator('.rk-task-repertoire').count());
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({results,wasmParity:true,classesPreserved:true,fullLibrary:true,legacyFallback:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));console.log('Passed twelve differentiated stat identities, task ordering, class/library preservation, WASM parity, legacy, mobile and print.');
}finally{await browser.close();}
