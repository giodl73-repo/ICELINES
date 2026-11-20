import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const base=process.env.ICELINES_PREVIEW_URL??'http://127.0.0.1:8772/';
const out=resolve(process.env.ICELINES_CANDIDATE_EVIDENCE??'target/compact-sheet/edge-candidate-profiles/acceptance');await mkdir(out,{recursive:true});
const reference=JSON.parse(await readFile('target/compact-sheet/edge-profiles/feat-candidates-v1.json','utf8')).scenarios.find(s=>s.min_games===10).rows;
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
const names={'two-gears':'Two Gears','top-gear':'Top Gear','burst-engine':'Burst Engine'};
let contexts=0;
try{
 const catalog=await(await page.request.get(new URL('catalog.json',base).href)).json();
 const cases=[];
 for(const w of catalog.windows)for(const id of [8478402,8479323,8481789,8480069])if(w.players.some(p=>p.id===id))cases.push(reference.find(r=>r.season===w.season&&r.player_id===id));
 for(const candidate of Object.keys(names))cases.push(reference.find(r=>r.season===catalog.current_season&&r.candidate===candidate));
 cases.push(reference.find(r=>r.season===catalog.current_season&&r.state==='insufficient_exposure'));
 for(const r of cases){assert.ok(r);await page.goto(`${base}?season=${r.season}&player=${r.player_id}`);await expect(page.locator('.rk-front h3')).toHaveText(r.name);
  const badges=page.locator('.rk-skating-candidate');await expect(badges).toHaveCount(r.candidate?1:0);
  if(r.candidate){await expect(badges).toContainText(names[r.candidate]);await expect(badges).toHaveAttribute('data-candidate-season',String(r.season));await badges.getByRole('button').click();}
  else await page.getByRole('button',{name:'Skating record',exact:true}).click();
  const dialog=page.locator('.rk-skating-dialog');await expect(dialog).toBeVisible();await expect(dialog).toContainText('Research'.toUpperCase());await expect(dialog).toContainText('No production feat award');
  if(r.state==='insufficient_exposure')await expect(dialog).toContainText('No qualification decision was made');
  else{for(const m of r.measurements.filter(m=>['peak_mph','bursts_per_60'].includes(m.metric)))await expect(dialog).toContainText(`P${m.percentile.toFixed(1)}`);}
  await dialog.getByRole('button',{name:'Close skating record'}).click();await expect(page.locator('.rk-front [data-rating-state="unassessed"]')).toHaveCount(6);contexts++;
 }
 await page.goto(base+'?season=current&player=8478402');await page.getByRole('button',{name:'Two Gears skating research candidate'}).click();await expect(page.locator('.rk-skating-dialog')).toContainText('205 bursts × 60 / 541.33 minutes = 22.72');await page.screenshot({path:resolve(out,'mcdavid-candidate-desktop.png'),fullPage:true});await page.locator('.rk-skating-dialog').getByRole('button',{name:'Close skating record'}).click();
 for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){await page.locator('#theme').selectOption(theme);await page.setViewportSize({width:390,height:844});await expect(page.locator('.rk-skating-candidate')).toContainText('Two Gears');assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.getByRole('button',{name:'Two Gears skating research candidate'}).click();assert.ok(await page.locator('.rk-skating-dialog').evaluate(d=>d.scrollWidth<=d.clientWidth));if(theme==='ice')await page.screenshot({path:resolve(out,'mcdavid-candidate-mobile.png'),fullPage:true});await page.locator('.rk-skating-dialog').getByRole('button',{name:'Close skating record'}).click();}
 const pdf=await page.pdf({path:resolve(out,'mcdavid-letter.pdf'),preferCSSPageSize:true,printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1);
 // The actual season control must replace the whole scoped candidate context.
 await page.locator('#season').selectOption('20242025');await expect(page.locator('.rk-front-scope')).toContainText('2024–25');const old=reference.find(r=>r.season===20242025&&r.player_id===8478402);await expect(page.locator('.rk-skating-candidate')).toHaveCount(old.candidate?1:0);if(old.candidate)await expect(page.locator('.rk-skating-candidate')).toHaveAttribute('data-candidate-season','20242025');
 await page.goto(base+'?season=20242025&player=8478048');await expect(page.locator('.rk-front h3')).toContainText('Shesterkin');await expect(page.locator('.rk-skating-candidate')).toHaveCount(0);await expect(page.getByRole('button',{name:'Skating record',exact:true})).toHaveCount(0);
 assert.deepEqual(errors,[]);await writeFile(resolve(out,'verification.json'),JSON.stringify({contexts,exclusive_candidates:true,season_control_updates:true,mobile_themes:6,one_page_letter:true,goalies_excluded:true,page_errors:errors},null,2));console.log(`Skating candidates: ${contexts} contexts, season change, six mobile themes and Letter print passed.`);
}finally{await browser.close();}
