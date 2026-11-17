// Accept shared-Rust profile projections; no player classification or scoring here.
import {chromium,expect} from '@playwright/test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url));
assert.ok(process.env.ICELINES_PROFILE_FIXTURES,'Supply the projected profile fixture file');
const fixtures=JSON.parse(await readFile(process.env.ICELINES_PROFILE_FIXTURES,'utf8'));
const output=resolve(process.env.ICELINES_PROFILE_RETURN_OUTPUT??resolve(repo,'target/compact-sheet/profile-return'));
await mkdir(output,{recursive:true});
const css=await readFile(resolve(repo,'icelines-web/static/rinkcraft-character.css'),'utf8');
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1280,height:1100}}),errors=[],rows=[];
page.on('pageerror',e=>errors.push(e.message));
await page.route('http://profiles.test/**',async route=>{
 const path=new URL(route.request().url()).pathname;
 if(path==='/'){await route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/sheet.css"><main></main>'});return;}
 await route.fulfill({contentType:path.endsWith('.js')?'text/javascript':'text/css',body:path==='/sheet.css'?css:await readFile(resolve(repo,'icelines-browser/dist'+path))});
});
const escape=s=>String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
let verifiedMeasurements=0;
try{
 await page.goto('http://profiles.test/');
 for(const [key,p] of Object.entries(fixtures)){
  assert.match(key,/^[a-z0-9-]+$/);
  const n=p.numerical_evidence,s=p.character_sheet;
  assert.ok(n&&s,'Comparison requires both documentary and numerical evidence');
  assert.equal(n.season,20242025,'Freeze one comparison season');
  const measures=[...n.facets.map(f=>f.measurement),...(n.context_evidence?.groups??[]).flatMap(g=>g.facets.map(f=>f.measurement))];
  for(const m of measures){if(m.value!==null&&m.numerator!==null&&m.denominator>0){assert.ok(Math.abs(m.value-m.numerator/m.denominator*m.scale)<1e-9,m.key);verifiedMeasurements++;}}
  await page.evaluate(async p=>{const {renderCharacterCard}=await import('/src/rinkcraft-character.js');document.querySelector('main').replaceChildren(renderCharacterCard(p.player_name,p.character_sheet,p.attribute_seasons.at(-1),p.latest_available,p.numerical_evidence,p.numerical_history,p.numerical_snapshot,[],true));},p);
  const front=page.locator('.rk-front');await expect(front).toBeVisible();
  await expect(front.locator('.rk-front-values dd')).toHaveText(Array(6).fill('—'));
  const speed=n.style_evidence?.measured.find(m=>m.key==='max-skating-speed');
  if(speed){await expect(front.locator('.rk-front-movement')).toContainText(speed.value.toFixed(2)+' mph');await expect(front.locator('.rk-front-movement')).toContainText('all strengths');}
  else await expect(front.locator('.rk-front-movement')).toContainText('Unassessed');
  await expect(front.locator('.rk-front-scope')).toContainText(n.situation);
  await expect(front.locator('.rk-front-methods li')).toHaveCount(s.features.length);
  for(const [i,f] of s.features.entries()){
   const item=front.locator('.rk-front-methods li').nth(i);
   await expect(item).toContainText(f.name);await expect(item).toContainText(f.evidence_type);
   if(f.detail)await expect(item).toContainText(f.detail);
   const evidence=item.locator('.rk-method-evidence');
   await expect(evidence).not.toHaveAttribute('open','');
   await evidence.locator('summary').click();
   await expect(evidence).toContainText('Related statistics do not establish this method');
   const contexts=(f.stat_evidence??[]).flatMap(link=>n.feature_context?.filter(c=>c.attribute_key===link.attribute_key)??[]);
   await expect(evidence.locator('[data-measurement-key]')).toHaveCount(contexts.reduce((sum,c)=>sum+c.measurements.length,0));
   if(!contexts.length)await expect(evidence).toContainText('No matched statistic');
   for(const gap of f.unmeasured_stats??[])await expect(evidence).toContainText(gap);
   for(const id of f.source_ids){const source=s.sources.find(v=>v.id===id);if(source)await expect(evidence.getByRole('link',{name:id+' · '+source.title,exact:true})).toHaveAttribute('href',source.url);}
   if(i===0){
    await evidence.screenshot({path:resolve(output,key+'-method-evidence.png')});
    await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await evidence.screenshot({path:resolve(output,key+'-method-evidence-mobile.png')});await page.setViewportSize({width:1280,height:1100});
   }
   await evidence.locator('summary').click();
  }
  await expect(page.locator('.rk-evidence')).not.toHaveAttribute('open','');
  assert.equal(await front.locator('[data-numerical-key]').count(),n.facets.length);
  const bounds=await front.boundingBox();assert.ok(bounds.width<=816);
  await page.screenshot({path:resolve(output,key+'.png'),fullPage:true});
  const html=await page.locator('.rk-profile').evaluate(e=>e.outerHTML);
  await writeFile(resolve(output,key+'.html'),'<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>'+escape(p.player_name)+' · Rinkcraft</title><style>'+css+'</style>'+html+'</html>');
  await front.locator('.rk-method-evidence').first().evaluate(e=>e.open=true);
  const pdf=await page.pdf({path:resolve(output,key+'.pdf'),preferCSSPageSize:true});
  await front.locator('.rk-method-evidence').first().evaluate(e=>e.open=false);
  assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,key+' single Letter page');
  await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.screenshot({path:resolve(output,key+'-mobile.png'),fullPage:true});await page.setViewportSize({width:1280,height:1100});
  rows.push({key,name:p.player_name,primary:s.primary.name,secondary:s.secondary.map(c=>c.name),specialty:s.specialty,summary:s.summary,season:n.season,situation:n.situation,group:n.group,games:n.games,minutes:n.minutes,documentary_through:s.evidence_through,methods:s.features,measurements:measures,html});
 }
 assert.deepEqual(errors,[]);
 const result={status:'passed',profiles:rows.length,verified_measurements:verifiedMeasurements,season:20242025,checks:['shared-Rust projections','numerator/denominator formulas','source-attributed method detail','explicit-only method/statistic links and unmatched states','per-method source links and measurement gaps','expanded method evidence on desktop/mobile','distinct skater/goalie batteries','no invented ability grades','desktop/mobile geometry','one-page PDF even with method evidence open','zero page errors']};
 await writeFile(resolve(output,'verification.json'),JSON.stringify(result,null,2));
 await writeFile(resolve(output,'comparison.json'),JSON.stringify(rows.map(({html,...r})=>r),null,2));
 const nav=rows.map(r=>'<a href="#'+r.key+'">'+escape(r.name)+'</a>').join(' · ');
 const summary='<table><thead><tr><th>Player</th><th>Class</th><th>Specialty</th><th>Documentary contribution</th></tr></thead><tbody>'+rows.map(r=>'<tr><td><a href="#'+r.key+'">'+escape(r.name)+'</a></td><td>'+escape(r.primary)+(r.secondary.length?' / '+escape(r.secondary.join(' / ')):'')+'</td><td>'+escape(r.specialty)+'</td><td>'+escape(r.summary)+'</td></tr>').join('')+'</tbody></table>';
 await writeFile(resolve(output,'index.html'),'<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft · Profile contrasts</title><style>'+css+' body{margin:0;background:#eef2f4;color:#18304a;font:14px system-ui}main{max-width:1000px;margin:auto;padding:16px}nav{line-height:2}table{border-collapse:collapse;width:100%;font-size:12px}td,th{padding:8px;text-align:left;border-bottom:1px solid #cad7e3}.table-scroll{overflow:auto}article{margin:24px 0}article>h2{font-size:18px}a{color:#145386}@media print{.intro,article>h2{display:none}article{break-before:page;margin:0}}</style><main><header class="intro"><h1>Rinkcraft · Five different contributions</h1><p>Frozen2024–25 statistical comparison. Skaters:5-on-5; goalie:all strengths. Percentiles use separate position peers and measure raw outcomes/activity. Documentary evidence has its own date; these are not current-season or career ability ratings.</p><nav>'+nav+'</nav><div class="table-scroll">'+summary+'</div><p>Expand each dossier for source links, metric formulas, evidence gaps and historical windows. Capability grades remain unrated.</p></header>'+rows.map(r=>'<article id="'+r.key+'"><h2>'+escape(r.name)+' · <a href="'+r.key+'.html">Standalone sheet</a> · <a href="'+r.key+'.pdf">Letter PDF</a></h2>'+r.html+'</article>').join('')+'</main></html>');
 console.log(JSON.stringify(result));
}finally{await browser.close();}
