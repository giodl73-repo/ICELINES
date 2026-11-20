// Historical preview and acceptance checks. All selections and estimates come from Rust.
import {chromium,expect} from '@playwright/test';
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url));
assert.ok(process.env.ICELINES_PROFILE_FIXTURES,'Supply season-aware Rust projections');
const fixtures=JSON.parse(await readFile(process.env.ICELINES_PROFILE_FIXTURES,'utf8'));
const output=resolve(process.env.ICELINES_PROFILE_WINDOW_OUTPUT??resolve(repo,'target/compact-sheet/profile-windows'));
await mkdir(output,{recursive:true});
const css=await readFile(resolve(repo,'icelines-web/static/rinkcraft-character.css'),'utf8');
const planUrl=process.env.ICELINES_MEASUREMENT_PLAN_URL;
if(planUrl)assert.match(planUrl,/^\/[a-z0-9-]+\/$/);
const escape=s=>String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1280,height:1100}}),errors=[],panels=[],results=[];
page.on('pageerror',e=>errors.push(e.message));
try{
 await page.route('http://windows.test/**',async route=>{
  const path=new URL(route.request().url()).pathname;
  await route.fulfill({contentType:path==='/'?'text/html':'text/javascript',body:path==='/'?'<!doctype html><meta charset="utf-8"><style>'+css+'</style><main></main>':await readFile(resolve(repo,'icelines-browser/dist'+path))});
 });
 await page.goto('http://windows.test/');
 for(const [selection,{key,id,mode,profile:p}] of Object.entries(fixtures)){
  assert.equal(selection,key+':'+mode);assert.match(key,/^[a-z]+$/);
  const n=p.numerical_evidence,s=p.numerical_snapshot;
  assert.ok(['20242025','20252026','latest'].includes(mode));
  if(mode!=='latest')assert.equal(s.requested_season,Number(mode));
  const status='<p class="window-state" data-state="'+escape(s.state)+'">'+escape(s.note)+'</p>';
  let html;
  if(!n){
   assert.equal(s.state,'unavailable');assert.equal(s.selected_season,null);assert.equal(p.latest_available,null);
   html='<section class="missing-window"><h2>'+escape(p.player_name)+'</h2><h3>No numerical record in this supplied window</h3>'+status+'<p>This is a data-coverage gap, not evidence of no games played or low ability. No older record is displayed as this season.</p><button type="button" data-use-latest>View latest available</button></section>';
  }else{
   assert.equal(n.season,s.selected_season);assert.equal(p.latest_available.season,n.season);
   if(mode!=='latest')assert.equal(n.season,Number(mode));
   await page.evaluate(async p=>{const {renderCharacterCard}=await import('/src/rinkcraft-character.js');document.querySelector('main').replaceChildren(renderCharacterCard(p.player_name,p.character_sheet,p.attribute_seasons.at(-1),p.latest_available,p.numerical_evidence,p.numerical_history,p.numerical_snapshot,[],true));},p);
   const front=page.locator('.rk-front'),note=front.locator('.rk-front-snapshot');
   await expect(note).toHaveCount(1);await expect(note).toHaveAttribute('data-snapshot-state',s.state);
   assert.ok(await note.evaluate(e=>e.nextElementSibling?.classList.contains('rk-front-columns')),'Selection note precedes all attribute columns');
   await expect(front.locator('.rk-front-values dd')).toHaveText(Array(6).fill('—'));
   if(process.env.ICELINES_REQUIRE_HISTORY_CHANGES==='1'){
    const h=p.numerical_history;assert.ok(h&&h.changes.length);assert.ok(h.seasons.every(s=>s.season<=n.season),'No later history in an earlier selected profile');
    await page.locator('.rk-evidence').evaluate(e=>e.open=true);
    const history=page.locator('.rh-history');await history.evaluate(e=>e.open=true);
    const changes=history.locator('.rh-changes');await changes.locator(':scope > summary').click();
    await expect(changes.locator('[data-change-window]')).toHaveCount(h.changes.length);
    for(const c of h.changes){const window=changes.locator('[data-change-window="'+c.to_season+'"]');for(const r of c.rows){const row=window.locator('[data-change-key="'+r.key+'"]');await expect(row).toHaveAttribute('data-change-state',r.state);if(r.state==='unavailable'||r.state==='incompatible')assert.equal(r.difference,null);}}
    if(mode==='20252026'){
     await changes.screenshot({path:resolve(output,key+'-changes.png')});
     await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await changes.screenshot({path:resolve(output,key+'-changes-mobile.png')});await page.setViewportSize({width:1280,height:1100});
    }
    await changes.locator(':scope > summary').click();await history.evaluate(e=>e.open=false);await page.locator('.rk-evidence').evaluate(e=>e.open=false);
   }
   if(process.env.ICELINES_REQUIRE_EVIDENCE_BRIEF==='1'){
    await page.locator('.rk-evidence').evaluate(e=>e.open=true);
    const brief=page.locator('.rk-brief');await expect(brief).toHaveCount(1);await expect(brief).toBeVisible();
    await expect(front.locator('.rk-brief')).toHaveCount(0);
    await expect(brief.locator('[data-brief-key]')).toHaveCount(n.leading.length);
    await expect(brief.locator('[data-brief-history]')).toHaveAttribute('data-brief-history','matched');
    for(const key of n.leading){const m=n.metrics.find(m=>m.key===key);await expect(brief.locator('[data-brief-key="'+key+'"]')).toContainText(m.label);}
    for(const gap of new Set(p.character_sheet.features.flatMap(f=>f.unmeasured_stats??[])))await expect(brief).toContainText(gap);
    if(n.finishing_research)await expect(brief.locator('[data-brief-finishing]')).toHaveAttribute('data-brief-finishing',n.finishing_research.status);
    if(mode==='20252026'||key==='barkov'&&mode==='latest'){
     await brief.screenshot({path:resolve(output,key+'-brief.png')});
     await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await brief.screenshot({path:resolve(output,key+'-brief-mobile.png')});await page.setViewportSize({width:1280,height:1100});
    }
    await page.locator('.rk-evidence').evaluate(e=>e.open=false);
   }
   const f=n.finishing_research;
   if(n.group==='Goalies'){assert.equal(f.status,'not_applicable');assert.ok(!f.support);}
   else{
    assert.equal(f.status,'experimental');assert.ok(f.history_seasons.every(y=>y<n.season));
    assert.deepEqual(f.history_seasons,n.season===20242025?[20232024]:[20232024,20242025]);
    await page.locator('.rk-evidence').evaluate(e=>e.open=true);
    await expect(page.locator('.rk-finishing')).toContainText('not a tested ability change');
    if(process.env.ICELINES_REQUIRE_FINISHING_SUPPORT==='1')assert.ok(f.support,'Require the rebuilt support-enabled engine');
    if(f.support){
     const support=f.support;
     assert.deepEqual(support.rows.map(r=>r.season),f.history_seasons);
     assert.ok(Math.abs(support.posterior_shape/support.posterior_rate-f.mean)<1e-12);
     assert.ok(Math.abs(support.adjusted_history_xg/support.posterior_rate-support.history_weight)<1e-12);
     const ledger=page.locator('.rk-finishing-support');await ledger.locator('summary').click();
     await expect(ledger.locator('tbody tr')).toHaveCount(support.rows.length);
     await expect(ledger).toContainText((support.history_weight*100).toFixed(1)+'%');
     if(mode==='20252026'){
      await ledger.screenshot({path:resolve(output,key+'-support.png')});
      await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
      await ledger.screenshot({path:resolve(output,key+'-support-mobile.png')});await page.setViewportSize({width:1280,height:1100});
     }
     await ledger.locator('summary').click();
    }
    await page.locator('.rk-evidence').evaluate(e=>e.open=false);
   }
   await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.setViewportSize({width:1280,height:1100});
   html=await page.locator('.rk-profile').evaluate(e=>e.outerHTML);
   if(mode!=='latest'||s.state==='older_fallback'){
    if(process.env.ICELINES_REQUIRE_EVIDENCE_BRIEF==='1')await page.locator('.rk-evidence').evaluate(e=>e.open=true);
    const pdf=await page.pdf({path:resolve(output,key+'-'+mode+'.pdf'),preferCSSPageSize:true});
    if(process.env.ICELINES_REQUIRE_EVIDENCE_BRIEF==='1')await page.locator('.rk-evidence').evaluate(e=>e.open=false);
    assert.equal(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length,1,selection+' single Letter page');
   }
  }
  const date=p.character_sheet.evidence_through;
  if(planUrl)html='<p class="rk-next-measurements"><a href="'+planUrl+'?player='+id+'">Authored next measurements · current research plan</a></p>'+html;
  panels.push('<article data-player="'+key+'" data-mode="'+mode+'" hidden><p class="documentary-date">Documentary identity: evidence through '+escape(date)+'. This numerical selector does not rewind the class or method dossier.</p>'+html+'</article>');
  results.push({key,mode,state:s.state,selected_season:s.selected_season,history_seasons:n?.finishing_research?.history_seasons??[],mean:n?.finishing_research?.mean??null,support:n?.finishing_research?.support??null,changes:p.numerical_history?.changes??[]});
 }
 assert.equal(results.length,15);assert.equal(results.filter(r=>r.state==='unavailable').length,1);assert.equal(results.filter(r=>r.state==='older_fallback').length,1);
 const options=Object.values(fixtures).filter(v=>v.mode==='latest').map(v=>'<option value="'+v.key+'">'+escape(v.profile.player_name)+'</option>').join('');
 const script=`const player=document.querySelector('#player'),mode=document.querySelector('#mode');function show(){for(const panel of document.querySelectorAll('article[data-player]'))panel.hidden=panel.dataset.player!==player.value||panel.dataset.mode!==mode.value;}player.addEventListener('change',show);mode.addEventListener('change',show);document.querySelectorAll('[data-use-latest]').forEach(b=>b.addEventListener('click',()=>{mode.value='latest';show();}));show();`;
 const document='<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft · Historical profile windows</title><style>'+css+' body{margin:0;background:#eef2f4;color:#18304a;font:14px system-ui}main{max-width:900px;margin:auto;padding:16px}.controls{display:flex;gap:16px;flex-wrap:wrap}label{display:grid;gap:4px}select,button{font:inherit;padding:8px}.documentary-date,.intro p{font-size:12px;line-height:1.5}.missing-window{background:white;border:2px solid #93672c;padding:20px}.window-state{font-weight:bold}article[hidden]{display:none!important}@media print{.intro,.documentary-date,.rk-next-measurements{display:none}main{padding:0}}</style><main><header class="intro"><h1>Rinkcraft · Historical profile windows</h1><p>Frozen supplied data through 2025–26, not a live 2026–27 profile. Skaters: 5-on-5; goalies: all strengths. Exact requests never borrow an older numerical record.</p><div class="controls"><label>Player<select id="player">'+options+'</select></label><label>Numerical window<select id="mode"><option value="20242025">2024–25 exactly</option><option value="20252026">2025–26 exactly</option><option value="latest">Latest supplied through 2025–26</option></select></label></div><p>Finishing pools earlier history under a constant multiplier. A revised estimate after adding evidence does not establish that ability changed. Capability attributes remain unrated. Expand the dossier for experimental intervals and source details.</p></header>'+panels.join('')+'<script>'+script+'</script></main></html>';
 await writeFile(resolve(output,'index.html'),document);
 await page.unrouteAll();await page.route('http://windows.test/**',r=>r.fulfill({contentType:'text/html',body:document}));await page.goto('http://windows.test/');
 for(const row of results){
  await page.locator('#player').selectOption(row.key);await page.locator('#mode').selectOption(row.mode);
  const active=page.locator('article[data-player]:not([hidden])');await expect(active).toHaveCount(1);
  if(planUrl)await expect(active.locator('.rk-next-measurements a')).toHaveAttribute('href',planUrl+'?player='+fixtures[row.key+':'+row.mode].id);
  if(row.state==='unavailable')await expect(active.locator('.rk-front')).toHaveCount(0);
  else await expect(active.locator('.rk-front-snapshot')).toHaveAttribute('data-snapshot-state',row.state);
 }
 await page.locator('#player').selectOption('barkov');await page.locator('#mode').selectOption('20252026');
 await page.screenshot({path:resolve(output,'unavailable.png'),fullPage:true});
 await page.getByRole('button',{name:'View latest available'}).click();await expect(page.locator('#mode')).toHaveValue('latest');
 await expect(page.locator('article[data-player]:not([hidden]) .rk-front-snapshot')).toHaveAttribute('data-snapshot-state','older_fallback');
 await page.screenshot({path:resolve(output,'fallback.png'),fullPage:true});
 await page.setViewportSize({width:390,height:844});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:resolve(output,'fallback-mobile.png'),fullPage:true});
 assert.deepEqual(errors,[]);
 const report={status:'passed',selections:results,checks:['exact missing never renders older sheet','explicit latest action reveals labeled fallback','all controls select one profile','documentary identity date independent of numerical window','prior-only one/two-season history','pooling assumption visible in dossier','six unrated capabilities','ten single-page Letter PDFs','desktop/mobile overflow','zero page errors']};
 if(process.env.ICELINES_REQUIRE_EVIDENCE_BRIEF==='1')report.checks.push('brief preserves supplied Rust selection and source-catalogue gaps','brief matched to player and selected season','brief desktop/mobile with unchanged compact front');
 if(process.env.ICELINES_REQUIRE_HISTORY_CHANGES==='1')report.checks.push('Rust snapshot differences and context gates','history stops at selected numerical season','expanded difference table desktop/mobile');
 if(process.env.ICELINES_REQUIRE_FINISHING_SUPPORT==='1')report.checks.push('posterior support ledger and algebraic identities');
 await writeFile(resolve(output,'verification.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({status:report.status,selections:results.length,checks:report.checks}));
}finally{await browser.close();}
