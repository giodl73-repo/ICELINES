// Render only validated Rust exports. No task assignment or score computation here.
import {chromium,expect} from '@playwright/test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url));
assert.ok(process.env.ICELINES_MEASUREMENT_PLAN,'Supply the Rust-exported plan JSON');
const c=JSON.parse(await readFile(process.env.ICELINES_MEASUREMENT_PLAN,'utf8'));
assert.equal(c.schema,'icelines.measurement-plan-catalog.v1');
const output=resolve(process.env.ICELINES_MEASUREMENT_PLAN_OUTPUT??resolve(repo,'target/compact-sheet/measurement-plan'));
await mkdir(output,{recursive:true});const css=await readFile(resolve(repo,'icelines-web/static/rinkcraft-character.css'),'utf8');
const escape=s=>String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1200,height:1000}}),errors=[],panels=[];
page.on('pageerror',e=>errors.push(e.message));
try{
 await page.route('http://plans.test/**',async route=>{const path=new URL(route.request().url()).pathname;await route.fulfill({contentType:path==='/'?'text/html':'text/javascript',body:path==='/'?'<!doctype html><meta charset="utf-8"><style>'+css+'</style><main></main>':await readFile(resolve(repo,'icelines-browser/dist'+path))});});await page.goto('http://plans.test/');
 for(const p of c.players){
  await page.evaluate(async({c,id})=>{const {renderMeasurementPlan}=await import('/src/rinkcraft-measurement-plan.js');document.querySelector('main').replaceChildren(renderMeasurementPlan(c,id));},{c,id:p.player_id});
  const root=page.locator('.rk-measurement-plan');await expect(root).toHaveAttribute('data-plan-player',String(p.player_id));await expect(root.locator('.rk-plan-task')).toHaveCount(p.tasks.length);
  assert.deepEqual(await root.locator('[data-plan-task]').evaluateAll(es=>es.map(e=>e.dataset.planTask)),p.tasks.map(t=>t.definition.task_id));
  for(const t of p.tasks){const task=root.locator('[data-plan-task="'+t.definition.task_id+'"]');assert.equal(t.definition.grade_release,false);await expect(task).toContainText(t.definition.denominator_scope);await expect(task).toContainText(t.definition.current_gap);await task.locator('summary').click();await expect(task).toContainText(t.definition.missingness_rule);await expect(task).toContainText('Grade released: no');await task.locator('summary').click();}
  await page.screenshot({path:resolve(output,p.player_id+'.png'),fullPage:true});await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:resolve(output,p.player_id+'-mobile.png'),fullPage:true});await page.setViewportSize({width:1200,height:1000});
  panels.push('<article data-player="'+p.player_id+'" hidden>'+await root.evaluate(e=>e.outerHTML)+'</article>');
 }
 await page.evaluate(async c=>{const {renderMeasurementPlan}=await import('/src/rinkcraft-measurement-plan.js');document.querySelector('main').replaceChildren(renderMeasurementPlan(c,0));},c);await expect(page.locator('main')).toContainText('No authored measurement plan');await expect(page.locator('.rk-plan-task')).toHaveCount(0);
 const html='<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft · next measurements</title><style>'+css+' body{margin:0;background:#eef2f4;color:#183247;font:14px system-ui}main{max-width:1050px;margin:auto;padding:16px}select{font:inherit;padding:8px}article[hidden]{display:none!important}</style><main><header><h1>Rinkcraft · next measurements</h1><p>Research companion: authored tasks, not player assessments. <a href="/profile-next/">Player evidence profiles</a></p><label>Player <select id="player">'+c.players.map(p=>'<option value="'+p.player_id+'">'+escape(p.name)+'</option>').join('')+'</select></label></header><p id="missing-plan" hidden>No authored measurement plan for this requested player; no fallback shown.</p>'+panels.join('')+'<script>const select=document.querySelector("#player"),requested=new URLSearchParams(location.search).get("player");if(requested!==null){if([...select.options].some(o=>o.value===requested))select.value=requested;else select.selectedIndex=-1;}function show(){document.querySelector("#missing-plan").hidden=select.selectedIndex>=0;for(const p of document.querySelectorAll("article[data-player]"))p.hidden=p.dataset.player!==select.value;}select.addEventListener("change",show);show();</script></main></html>';
 await writeFile(resolve(output,'index.html'),html);await page.unrouteAll();await page.route('http://plans.test/**',r=>r.fulfill({contentType:'text/html',body:html}));await page.goto('http://plans.test/');
 for(const p of c.players){await page.locator('#player').selectOption(String(p.player_id));await expect(page.locator('article[data-player]:not([hidden])')).toHaveCount(1);await expect(page.locator('article[data-player]:not([hidden])')).toContainText(p.name);}
 await page.goto('http://plans.test/?player='+c.players.at(-1).player_id);await expect(page.locator('#player')).toHaveValue(String(c.players.at(-1).player_id));await page.goto('http://plans.test/?player=0');await expect(page.locator('#missing-plan')).toBeVisible();await expect(page.locator('article[data-player]:not([hidden])')).toHaveCount(0);
 assert.deepEqual(errors,[]);const report={status:'passed',players:c.players.length,task_references:c.players.reduce((n,p)=>n+p.tasks.length,0),checks:['authored task order and attribute branch preserved','explicit denominator and current gaps','missingness/boundaries expandable','no released grades','unknown player has no fallback','desktop/mobile geometry','all player controls','zero page errors']};await writeFile(resolve(output,'verification.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
}finally{await browser.close();}
