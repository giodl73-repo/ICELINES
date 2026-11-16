import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
const base=process.env.ICELINES_GOAL_ANALYSIS_URL||'http://127.0.0.1:8786';
const out=process.env.ICELINES_GOAL_ANALYSIS_ARTIFACTS||'F:/src/ICELINES-worktrees/rinkcraft-sheet/target/goal-analysis-feature-2026-11-15';
await mkdir(out,{recursive:true});const browser=await chromium.launch({channel:'msedge',headless:true});const page=await browser.newPage({viewport:{width:1440,height:1100}});let errors=[];page.on('pageerror',e=>errors.push(e.message));
await page.goto(base+'/goal-analysis');assert.equal(await page.locator('#ga-report').isHidden(),true);assert.ok(await page.getByRole('link',{name:'Analyze a goal',exact:true}).count());
await page.goto(base+'/goal-analysis?game_id=2025021167');assert.equal(await page.locator('.ga-choices a').count(),4);
for(const [game,event,folder] of [[2025021167,907,'offline-907'],[2025021167,996,'offline-996'],[2025021167,154,'offline-154'],[2025021167,607,'offline-607'],[2024020151,172,'new-live-goal']]){
 const target=`${game}/${event}`;await page.goto(base+'/goal-analysis?target='+encodeURIComponent(target));assert.equal(await page.locator('#ga-report').isVisible(),true);
 const api=await page.request.get(base+'/api/v1/goal-analysis?target='+encodeURIComponent(target));assert.equal(api.status(),200);const data=await api.json();const exported=JSON.parse(await readFile(out+'/'+folder+'/analysis.json','utf8'));assert.deepEqual(data.audit,exported.audit);assert.deepEqual(data.summary,exported.summary);assert.deepEqual(data.geometry_changes,exported.geometry_changes);assert.deepEqual(data.goal_geometry,exported.goal_geometry);
 assert.equal(data.goal_geometry.status,'available');assert.equal(await page.locator('[data-goal-reference="attacking"]').count(),1);assert.ok((await page.locator('#ga-goal-geometry').innerText()).includes('ft to the fixed goal reference'));

 assert.ok((await page.locator('#ga-comparison').innerText()).includes('no earlier comparison'));
 assert.equal(data.geometry_changes.frames.length,data.audit.events[0].frames.length);
 const max=await page.locator('#ga-frame').getAttribute('max');await page.locator('#ga-frame').evaluate((n,v)=>{n.value=v;n.dispatchEvent(new Event('input'))},max);assert.ok((await page.locator('#ga-clock').innerText()).includes(`Frame ${max}/${max}`));
 const comparison=data.geometry_changes.frames[+max];
 assert.equal(await page.locator('#ga-movement tr').count(),comparison.matched_players.length);
 assert.ok((await page.locator('#ga-comparison').innerText()).includes(`Frames ${comparison.reference_frame} → ${comparison.frame}`));
 assert.ok(await page.locator('[data-motion="endpoint"]').count()>0);
 await page.locator('#ga-trails').uncheck();assert.equal(await page.locator('[data-motion="endpoint"]').count(),0);
 await page.locator('#ga-trails').check();assert.equal(await page.locator('[data-motion="endpoint"]').count(),comparison.matched_players.length);
 assert.ok(await page.locator('#ga-rink circle').count()>0);assert.equal(await page.locator('#ga-runs button').count(),data.nearest_observation_runs.length);
 await page.locator('details').first().evaluate(n=>n.open=true);const runIndex=Math.min(1,data.nearest_observation_runs.length-1);await page.locator('#ga-runs button').nth(runIndex).click();assert.equal(+(await page.locator('#ga-frame').inputValue()),data.nearest_observation_runs[runIndex].first_frame);await page.locator('details').first().evaluate(n=>n.open=false);
 if(event===154)assert.ok((await page.locator('#ga-goalie').innerText()).includes('empty-net'));
 await page.locator('#ga-frame').evaluate(n=>{n.value='0';n.dispatchEvent(new Event('input'))});await page.click('#ga-play');await page.waitForTimeout(350);await page.click('#ga-play');assert.ok(+(await page.locator('#ga-frame').inputValue())>0);
}
await page.goto(base+'/goal-analysis?target=2025021167%2F996');assert.ok((await page.locator('#ga-defense').innerText()).includes('not observed'));
await page.locator('#ga-frame').evaluate(n=>{n.value='70';n.dispatchEvent(new Event('input'))});await page.screenshot({path:out+'/native-desktop.png',fullPage:true});
assert.equal(await page.locator('#ga-play').evaluate(n=>getComputedStyle(n).color),'rgb(23, 40, 61)');
await page.setViewportSize({width:390,height:844});await page.locator('details').last().evaluate(n=>n.open=true);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'Expanded source overflow');await page.locator('details').last().evaluate(n=>n.open=false);await page.screenshot({path:out+'/native-mobile.png',fullPage:true});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'Mobile overflow');
await page.setViewportSize({width:1280,height:1000});await page.goto(pathToFileURL(out+'/offline-996/analysis.html').href);assert.equal(await page.locator('#ga-report').isVisible(),true);assert.equal(await page.locator('.ga-form').count(),0);
await page.locator('#ga-frame').evaluate(n=>{n.value='70';n.dispatchEvent(new Event('input'))});assert.ok(await page.locator('#ga-movement tr').count()>0);assert.ok(await page.locator('[data-motion="endpoint"]').count()>0);
await page.goto(base+'/goal-analysis');await page.fill('#ga-target','https://www.nhl.com/ppt-replay/goal/2025021167/996');await page.getByRole('button',{name:'Analyze goal',exact:true}).click();assert.equal(await page.locator('#ga-report').isVisible(),true);
assert.deepEqual(errors,[]);const report={status:'passed',goals:5,surfaces:['native','json','portable export'],checks:['goal picker','URL form','all-frame access','playback','nearest-run controls','empty-net context','unknown scorer observation','desktop/mobile','zero page errors','same-player geometry','nearest-identity switch metadata','endpoint overlay toggle','portable geometry']};await writeFile(out+'/ui-verification.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report));await browser.close();
