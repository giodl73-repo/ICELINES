import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const base=process.env.ICELINES_PREVIEW_URL??'http://127.0.0.1:8772/';
const out=resolve(process.env.ICELINES_EDGE_EVIDENCE??'target/compact-sheet/edge-profiles/acceptance');await mkdir(out,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
let cases=0;
try{
 const catalog=await (await page.request.get(new URL('catalog.json',base).href)).json();
 const coverage=await (await page.request.get(new URL('edge-coverage.json',base).href)).json();assert.equal(coverage.profiles,3941);assert.equal(coverage.available,3933);
 for(const w of catalog.windows)for(const id of [8478402,8479323,8481789,8473986,8480834,8478048]){
  const actor=w.players.find(p=>p.id===id);if(!actor)continue;
  const profile=await(await page.request.get(new URL(actor.url,base).href)).json(),edge=profile.numerical_evidence.edge_evidence;assert.ok(edge);
  await page.goto(`${base}?season=${w.season}&player=${id}`);await expect(page.locator('.rk-front h3')).toHaveText(actor.name);await expect(page.locator('#data-notice')).toContainText('NHL EDGE observations included');
  const attribute=actor.group==='Goalies'?'Positioning':'Skating';await page.getByRole('button',{name:`How ${attribute} is scored`,exact:true}).click();
  const dialog=page.locator('.rk-capability-dialog');await expect(dialog.locator('.rk-edge-evidence')).toHaveAttribute('data-edge-season',String(w.season));await expect(dialog).toContainText(`${actor.games} GP · all strengths`);await expect(dialog.locator('.rk-capability-grade')).toHaveText('— / 20 · Unrated');
  if(actor.group==='Goalies')await expect(dialog).toContainText('Shot-stopping context · not a technique grade');
  else{const max=edge.measures.find(m=>m.key==='max-skating-speed');if(max)await expect(dialog.locator('.rk-edge-evidence')).toContainText(`${max.value.toFixed(2)} mph`);}
  await dialog.getByRole('button',{name:'Close rulebook'}).click();await expect(page.locator('.rk-front [data-rating-state="unassessed"]')).toHaveCount(6);cases++;
 }
 await page.goto(base+'?season=current&player=8478402');await expect(page.locator('.rk-front-movement strong')).toHaveText('24.01 mph');
 await page.getByRole('button',{name:'How Shooting is scored',exact:true}).click();await expect(page.locator('.rk-capability-dialog')).toContainText('Season maximum shot speed');await expect(page.locator('.rk-capability-dialog')).toContainText('Shot-location context · not target accuracy');await page.locator('.rk-capability-dialog').getByRole('button',{name:'Close rulebook'}).click();
 await page.getByRole('button',{name:'How Skating is scored',exact:true}).click();await expect(page.locator('.rk-capability-dialog')).toContainText('205');await expect(page.locator('.rk-capability-dialog')).toContainText('NHL percentile P99.8');await page.screenshot({path:resolve(out,'mcdavid-skating-desktop.png'),fullPage:true});await page.locator('.rk-capability-dialog').getByRole('button',{name:'Close rulebook'}).click();
 for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){await page.locator('#theme').selectOption(theme);await page.setViewportSize({width:390,height:844});await page.getByRole('button',{name:'How Skating is scored',exact:true}).click();assert.ok(await page.locator('.rk-capability-dialog').evaluate(d=>d.scrollWidth<=d.clientWidth));assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.locator('.rk-capability-dialog').getByRole('button',{name:'Close rulebook'}).click();}
 await page.goto(base+'?season=20242025&player=8478048');await page.getByRole('button',{name:'How Positioning is scored',exact:true}).click();await expect(page.locator('.rk-edge-location-scroll')).toBeVisible();assert.ok(await page.locator('.rk-capability-dialog').evaluate(d=>d.scrollWidth<=d.clientWidth));await page.screenshot({path:resolve(out,'goalie-locations-mobile.png'),fullPage:true});await page.locator('.rk-capability-dialog').getByRole('button',{name:'Close rulebook'}).click();
 const pdf=await page.pdf({path:resolve(out,'goalie-letter.pdf'),preferCSSPageSize:true,printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);
 for(const r of coverage.records.filter(r=>r.state!=='available')){await page.goto(`${base}?season=${r.season}&player=${r.player_id}`);await expect(page.locator('.rk-front')).toBeVisible();await expect(page.locator('#source-detail')).toContainText(r.reason);await expect(page.locator('#data-notice')).not.toContainText('EDGE observations included');}
 assert.deepEqual(errors,[]);await writeFile(resolve(out,'verification.json'),JSON.stringify({player_season_context_cases:cases,missing_source_cases:8,mobile_themes:6,one_page_letter:true,source_scope_bound:true,capability_grades_unrated:true,page_errors:errors},null,2));console.log('EDGE: 24 player-season contexts, eight explicit exclusions, six mobile themes and one-page print passed.');
}finally{await browser.close();}
