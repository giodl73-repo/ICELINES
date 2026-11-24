// Acceptance against a prepared local renderer preview and matching WASM bindings.
// The preview exposes players.json, profiles/ID.json, and the captured ledger.
import assert from 'node:assert/strict';
import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const base=process.env.ICELINES_GOALIE_PUCK_URL??'http://127.0.0.1:8777/';
const output=new URL('../../target/goalie-puck-profile/',import.meta.url);
await mkdir(output,{recursive:true});
const read=async path=>{const r=await fetch(new URL(path,base));assert.equal(r.status,200,path);return r.json();};
const players=await read('players.json'),profiles=[];
for(const p of players)profiles.push(await read(`profiles/${p.id}.json`));
assert.ok(profiles.length>0&&profiles.every(p=>p.goalie_puck_events),'Prepare the Rust-projected profiles first.');
const ledger=await read('rinkcraft-goalie-puck-events.json');
// Pick contrasting actual records rather than pinning time-sensitive player counts.
const candidates=[profiles.find(p=>p.goalie_puck_events.metrics.find(m=>m.label==='Goals')?.count>0),
 profiles.reduce((a,b)=>a.goalie_puck_events.plays.length>b.goalie_puck_events.plays.length?a:b),
 profiles.find(p=>p.goalie_puck_events.plays.length===0)].filter(Boolean);
assert.ok(candidates.length>=2,'Need contrasting samples.');
const browser=await chromium.launch({channel:process.env.ICELINES_BROWSER_CHANNEL??'msedge',headless:true});
try {
 const page=await browser.newPage({viewport:{width:1280,height:1100}}),errors=[];let shotCases=0;
 page.on('pageerror',e=>errors.push(e.message));
 for(const [i,p] of candidates.entries()){
  const id=p.request.player_id,evidence=p.goalie_puck_events;
  await page.goto(new URL(`?player=${id}`,base).href);
  const block=page.locator('.rk-puck-record');await block.waitFor();
  assert.equal(await block.getAttribute('data-puck-player'),String(id));
  assert.equal(await block.getAttribute('data-puck-season'),String(evidence.season));
  assert.deepEqual(await block.locator('.rk-puck-values dd').allTextContents(),evidence.metrics.map(m=>String(m.count)));
  assert.equal(await block.locator('details').getAttribute('open'),null);
  await block.locator('summary').click();
  assert.equal(await block.locator('details a').count(),evidence.plays.length*2);
  assert.match(await block.textContent(),/not projected totals or handling success rates/);
  // Exercise the real worker operation and WASM export, not a browser-side counter.
  const projected=await page.evaluate(async({ledger,profile})=>{
   const worker=new Worker('/src/worker.js',{type:'module'});
   try{return await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('Worker timeout')),60000);
    worker.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};
    worker.onmessage=e=>{clearTimeout(timer);e.data.error?reject(new Error(e.data.error.message)):resolve(e.data.value);};
    worker.postMessage({schema_version:1,request_id:1,context_generation:1,operation:'goaliePuckForProfile',payload:{ledger,profile}});
   });}finally{worker.terminate();}
  },{ledger,profile:p});
  assert.deepEqual(projected,evidence);
  await block.locator('summary').click();
  const stats=p.stat_attribute_seasons?.find(s=>s.season===p.latest_available?.season&&s.shot_context);
  if(stats){
   const shot=page.locator('.rk-front .rk-shot-context');await shot.waitFor();
   assert.deepEqual(await shot.locator(':scope > dl dd').allTextContents(),stats.shot_context.headline.map(r=>r.value));
   await shot.locator('summary').click();assert.match(await shot.textContent(),/after missed shots/);await shot.locator('summary').click();
   for(const i of [2,3,4]){await page.locator('.rk-front-values .rk-capability-trigger').nth(i).click();const dialog=page.locator('dialog');assert.match(await dialog.textContent(),/Observed shot contexts/);await dialog.getByRole('button',{name:'Close',exact:true}).click();}
   shotCases++;
  }
  await page.evaluate(()=>scrollTo(0,0));
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('puck-desktop.png',output)),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  const grid=await block.locator('.rk-puck-values').boundingBox();assert.ok(grid&&grid.width<=390);
  await page.evaluate(()=>scrollTo(0,0));
  if(i===0)await page.screenshot({path:fileURLToPath(new URL('puck-mobile.png',output)),fullPage:true});
  await page.setViewportSize({width:1280,height:1100});
  await page.emulateMedia({media:'print'});
  const pdf=await page.pdf({path:fileURLToPath(new URL(`puck-${i}.pdf`,output)),format:'Letter',printBackground:true});
  assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,'Primary sheet must fit one page.');
  await page.emulateMedia({media:'screen'});
 }
 assert.deepEqual(errors,[]);
 await writeFile(new URL('browser-acceptance.json',output),JSON.stringify({players:profiles.length,cases:candidates.map(p=>p.request.player_id),shotCases,workerParity:true,mobile:true,pageErrors:errors},null,2));
 console.log(`Passed ${candidates.length} goalie cases; real worker/WASM parity and 390px layout.`);
}finally{await browser.close();}
