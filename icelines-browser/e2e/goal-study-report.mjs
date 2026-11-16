import {chromium} from 'playwright';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
const root=process.env.ICELINES_GOAL_STUDY_ARTIFACTS;
assert.ok(root,'Set ICELINES_GOAL_STUDY_ARTIFACTS to the generated study directory');
const summary=JSON.parse(await readFile(root+'/summary.json','utf8'));
const review=JSON.parse(await readFile(root+'/review_records.json','utf8')).records;
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1400,height:1000}});const errors=[];
page.on('pageerror',e=>errors.push(e.message));
await page.goto(process.env.ICELINES_GOAL_STUDY_URL||'http://127.0.0.1:8787/');
assert.equal(await page.locator('#cases article').count(),summary.planned_cases);
assert.equal(await page.locator('#summary tr').count(),summary.players.length);
assert.equal(await page.locator('#cases article[data-status="captured"]').count(),summary.captured_cases);
await page.selectOption('#status','unavailable');
assert.equal(await page.locator('#cases article').count(),summary.planned_cases-summary.captured_cases);
await page.selectOption('#status','');
for(const p of summary.players){await page.selectOption('#player',p.player);assert.equal(await page.locator('#cases article').count(),p.planned_cases);}
for(const r of review.filter(r=>r.status==='captured'))assert.equal(r.goal_event_windows.exact_goal_frame_verified,false);
for(const r of review.filter(r=>r.status==='captured'&&r.goal_event_windows.candidate_count!==1)){
 await page.selectOption('#player',r.selection.player.name);const c=page.locator('#cases article').filter({hasText:r.selection.game_date});await c.locator('.event-windows').evaluate(n=>n.open=true);
 assert.equal(await c.locator('.crossing-candidate').count(),Math.min(4,r.goal_event_windows.candidate_count));
 if(!r.goal_event_windows.candidate_count){assert.equal(await c.locator('.crossing-diagnostic').count(),1);assert.ok((await c.innerText()).includes('official goal and touch order remain recorded'));}
}
await page.selectOption('#player','Matthew Tkachuk');
const annotated=review.find(r=>r.selection.player.name==='Matthew Tkachuk'&&r.documentary_annotation);
assert.ok(annotated);const card=page.locator('#cases article').filter({hasText:annotated.selection.game_date});
assert.ok((await card.innerText()).includes('Delivery plus receiver redirect'));
assert.ok((await card.innerText()).includes('slap'));
assert.equal(await card.getByRole('link',{name:'SRC-132',exact:true}).count(),1);
assert.ok((await card.locator('dt[data-credit="secondary assist"]').innerText()).includes('Mikkola'));
assert.ok((await card.locator('.contributors dd').nth(0).innerText()).includes('Action unassessed'));
assert.ok((await card.locator('dt[data-credit="primary assist"]').innerText()).includes('Kulikov'));
assert.ok((await card.locator('.contributors dd').nth(1).innerText()).includes('Shot delivery'));
assert.ok((await card.locator('.contributors dd').nth(2).innerText()).includes('Receiver redirect'));
assert.ok((await card.locator('.touch-order').innerText()).includes('N. Mikkola → D. Kulikov → M. Tkachuk'));
await card.locator('.touch-anchors').evaluate(n=>n.open=true);
assert.ok((await card.locator('.anchor-candidate').first().innerText()).includes('N. Mikkola frames 54–67 → D. Kulikov frames 68–70 → M. Tkachuk frames 73–73'));
assert.equal(await card.locator('.goal-relative-anchors li:not(.goalie-reference)').count(),3);
assert.equal(await card.locator('.goalie-reference').count(),3);
assert.ok((await card.locator('.goalie-reference').first().innerText()).includes('J. Daccord'));
assert.ok((await card.locator('.goalie-reference').first().innerText()).includes('reference line moves with the puck'));
await card.locator('.event-windows').evaluate(n=>n.open=true);
assert.ok((await card.locator('.crossing-candidate').first().innerText()).includes('Frames 82 → 83'));
assert.ok((await card.locator('.pre-crossing-chain').first().innerText()).includes('N. Mikkola 54–67 → D. Kulikov 68–70 → M. Tkachuk 73–73'));


assert.ok((await card.locator('.goal-relative-anchors').innerText()).includes('ft from goal'));



const native=await card.getByRole('link',{name:'Open native IceLines sequence'}).getAttribute('href');
await card.screenshot({path:root+'/assist-card.png'});
await page.screenshot({path:root+'/study-desktop.png',fullPage:true});
await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
await page.screenshot({path:root+'/study-mobile.png',fullPage:true});
await page.goto(native);assert.equal(await page.locator('#ga-report').isVisible(),true);
assert.ok((await page.locator('#ga-heading').innerText()).includes('Tkachuk'));
assert.deepEqual(errors,[]);await browser.close();
const result={status:'passed',planned_cases:summary.planned_cases,captured_cases:summary.captured_cases,checks:['all cases and missing cases','per-player filter','attributed annotation and original shot label','official source link','native sequence link','desktop/mobile','zero page errors','official assist slots','source-supported actions and unresolved secondary contribution','credited touch order and ordered tracking anchors','fixed-goal distance and bearing at candidate intervals','recorded goalie marker reference without a position grade','2D crossing brackets and pre-crossing credited anchors']};
await writeFile(root+'/ui-verification.json',JSON.stringify(result,null,2));console.log(JSON.stringify(result));
