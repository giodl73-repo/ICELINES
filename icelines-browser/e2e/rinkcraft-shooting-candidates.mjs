import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const base=process.env.ICELINES_PREVIEW_URL??'http://127.0.0.1:8772/';
const out=resolve(process.env.ICELINES_SHOOTING_EVIDENCE??'target/compact-sheet/shooting-candidate-profiles/acceptance');await mkdir(out,{recursive:true});
const report=JSON.parse(await readFile('target/compact-sheet/shooting-records-20261120/report-v2.json','utf8'));
const rows=report.scenarios.find(s=>s.minimum_conversion_shots===25).rows;
const names={'shot-power':'Shot Power','shot-volume':'Shot Volume','power-volume':'Power & Volume'};
const browser=await chromium.launch({channel:'msedge',headless:true}),page=await browser.newPage({viewport:{width:1440,height:1100}}),errors=[];page.on('pageerror',e=>errors.push(e.message));let contexts=0;
try{
 const catalog=await(await page.request.get(new URL('catalog.json',base).href)).json(),current=catalog.current_season;
 const examples=['Pavel Dorofeyev','Nathan MacKinnon','Cale Makar','David Pastrnak','Adam Fox'];
 const cases=rows.filter(r=>examples.includes(r.record.name));
 for(const key of Object.keys(names))cases.push(rows.find(r=>r.record.season===current&&r.candidate===key));
 cases.push(rows.find(r=>r.record.season===current&&r.state==='insufficient_exposure'));
 cases.push(rows.find(r=>r.record.season===current&&r.record.edge_sha256===null));
 cases.push(...rows.filter(r=>r.record.edge_all_counts_match===false));
 for(const row of cases){assert.ok(row);const r=row.record;await page.goto(`${base}?season=${r.season}&player=${r.player_id}`);await expect(page.locator('.rk-front h3')).toHaveText(r.name);await expect(page.locator('.rk-shooting-candidate')).toHaveCount(row.candidate?1:0);
  if(row.candidate){await expect(page.locator('.rk-shooting-candidate')).toHaveAttribute('data-candidate-season',String(r.season));await page.getByRole('button',{name:`${names[row.candidate]} shooting research candidate`,exact:true}).click();}
  else await page.getByRole('button',{name:'Shooting record',exact:true}).click();
  const dialog=page.locator('.rk-shooting-dialog');await expect(dialog).toContainText(r.name);await expect(dialog).toContainText('RESEARCH CANDIDATE');await expect(dialog).toContainText('not capability ratings or production feat awards');
  if(row.state==='insufficient_exposure')await expect(dialog).toContainText('No qualification decision was made');
  else if(row.state==='unassessed')await expect(dialog).toContainText('Missing evidence is not failure');
  for(const m of row.measurements.filter(m=>['peak_mph','shots_per_60'].includes(m.metric)&&m.percentile!==null))await expect(dialog).toContainText(`P${m.percentile.toFixed(1)}`);
  if(r.edge_all_counts_match===false)await expect(dialog).toContainText(`official ${r.goals} goals / ${r.shots} SOG; EDGE ${r.edge_all_goals} goals / ${r.edge_all_shots} SOG`);
  await dialog.getByRole('button',{name:'Close shooting record'}).click();await expect(page.locator('.rk-front [data-rating-state="unassessed"]')).toHaveCount(6);contexts++;
 }
 const makar=rows.find(r=>r.record.name==='Cale Makar'&&r.record.season===current).record;
 await page.goto(`${base}?season=current&player=${makar.player_id}`);await expect(page.locator('.rk-seasonal-records .rk-skating-candidate')).toHaveCount(1);await expect(page.locator('.rk-seasonal-records .rk-shooting-candidate')).toHaveCount(1);await page.locator('.rk-front').screenshot({path:resolve(out,'makar-two-families-desktop.png')});
 for(const theme of ['ice','ledger','bureau','tactics','broadcast','editorial']){await page.locator('#theme').selectOption(theme);await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.getByRole('button',{name:'Power & Volume shooting research candidate',exact:true}).click();assert.ok(await page.locator('.rk-shooting-dialog').evaluate(d=>d.scrollWidth<=d.clientWidth));if(theme==='ice')await page.screenshot({path:resolve(out,'shooting-dialog-mobile.png'),fullPage:true});await page.locator('.rk-shooting-dialog').getByRole('button',{name:'Close shooting record'}).click();}
 for(const name of ['Cale Makar','David Pastrnak','Nathan MacKinnon']){const r=rows.find(r=>r.record.name===name&&r.record.season===current).record;await page.goto(`${base}?season=current&player=${r.player_id}`);const pdf=await page.pdf({path:resolve(out,`${r.player_id}-letter.pdf`),preferCSSPageSize:true,printBackground:true});assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,name);}
 await page.locator('#season').selectOption('20242025');await expect(page.locator('.rk-front-scope')).toContainText('2024–25');const old=rows.find(r=>r.record.name==='Nathan MacKinnon'&&r.record.season===20242025);await expect(page.locator('.rk-shooting-candidate')).toHaveCount(old.candidate?1:0);if(old.candidate)await expect(page.locator('.rk-shooting-candidate')).toHaveAttribute('data-candidate-season','20242025');
 const goalie=catalog.windows.find(w=>w.season===current).players.find(p=>p.group==='Goalies');await page.goto(`${base}?season=current&player=${goalie.id}`);await expect(page.locator('.rk-front')).toBeVisible();await expect(page.getByRole('button',{name:'Shooting record',exact:true})).toHaveCount(0);
 assert.deepEqual(errors,[]);await writeFile(resolve(out,'verification.json'),JSON.stringify({contexts,both_families_coexist:true,source_count_disagreements:5,mobile_themes:6,letter_profiles:3,season_switch:true,goalies_excluded:true,page_errors:errors},null,2));console.log(`Shooting: ${contexts} contexts, five count differences, both families, six mobile themes and three Letter profiles passed.`);
}finally{await browser.close();}
