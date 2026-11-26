import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_GOALIE_TRIAL_URL??'http://127.0.0.1:8780/goalie-feat-trial/';
const output=new URL('../../target/goalie-feat-trial/',import.meta.url);await mkdir(output,{recursive:true});
const roster=await(await fetch(new URL('players.json',base))).json();const original=[];
for(const {id} of roster)original.push([id,await(await fetch(new URL(`../profiles/${id}.json`,base))).text()]);
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1200}}),errors=[],results=[];page.on('pageerror',e=>errors.push(e.message));
 for(const [i,name]of ['Linus Ullmark','Darcy Kuemper','Juuse Saros','Igor Shesterkin'].entries()){
  const {id}=roster.find(p=>p.name===name),p=await(await fetch(new URL(`profiles/${id}.json`,base))).json();
  await page.goto(new URL(`?player=${id}`,base).href);await page.locator('.rk-front').waitFor();
  assert.match(await page.locator('[data-experimental-goalie-feats]').textContent(),/EXPERIMENTAL.*active registry unchanged/);
  assert.equal(new URL(await page.locator('#active-card').getAttribute('href'),base).searchParams.get('player'),String(id));
  const a=p.achievements;assert.equal(a.feats.length,5);const candidates=a.feats.filter(f=>f.id.startsWith('goalie-'));
  assert.equal(await page.locator('.rk-front .rk-stat-feat').count(),5);
  if(name==='Linus Ullmark'){assert.equal(candidates.find(f=>f.id==='goalie-save-percentage').tier,'Gold');assert.match(await page.locator('.rk-stat-specialties').textContent(),/Save percentage.*Gold/);}
  if(name==='Darcy Kuemper')assert.match(await page.locator('.rk-front-role-coverage').textContent(),/Shot workload.*Bronze/);
  if(name==='Igor Shesterkin')assert.ok(candidates.every(f=>f.state==='ineligible'));
  const actual=await page.evaluate(async a=>{const {default:init,BrowserEngine}=await import('./pkg/icelines_wasm.js');await init();const e=new BrowserEngine();try{return JSON.parse(e.task_repertoire(JSON.stringify(a)));}finally{e.free();}},a);assert.deepEqual(actual,a.task_repertoire);
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('trial-ullmark.png',output)),fullPage:true});
  await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.setViewportSize({width:1280,height:1200});await page.emulateMedia({media:'print'});const pdf=await page.pdf({path:fileURLToPath(new URL(`trial-${i}.pdf`,output)),format:'Letter',printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,`${name} print`);await page.emulateMedia({media:'screen'});results.push({name,id,candidates:candidates.map(f=>({id:f.id,state:f.state,tier:f.tier}))});
 }
 await page.goto(new URL('study.html',base).href);assert.match(await page.textContent('body'),/Two goalie feat candidates.*103.*2026/s);await page.setViewportSize({width:390,height:844});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
 for(const [id,text]of original)assert.equal(await(await fetch(new URL(`../profiles/${id}.json`,base))).text(),text);
 assert.deepEqual(errors,[]);await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({results,activeProfilesUnchanged:true,experimentalLabels:true,wasmParity:true,mobile:true,onePagePrint:true,pageErrors:errors},null,2));console.log('Passed four opt-in goalie cards, experimental labels, active-profile preservation, WASM parity, mobile and print.');
}finally{await browser.close();}
