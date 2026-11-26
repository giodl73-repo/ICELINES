// Prepare isolated review cards from a Rust trial; never rewrite active profiles.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,access,cp} from 'node:fs/promises';
import {resolve,join,dirname} from 'node:path';
import {pathToFileURL} from 'node:url';
const [studyPath,basePath,outputPath,wasmPath,reportsPath]=process.argv.slice(2);
assert.ok(reportsPath,'Usage: STUDY_JSON BASE_SITE NEW_OUTPUT WASM_DIR CURRENT_REPORTS');
const study=JSON.parse(await readFile(studyPath,'utf8')),base=resolve(basePath),out=resolve(outputPath);
try{await access(out);throw Error('Choose a new output directory');}catch(e){if(e.code!=='ENOENT')throw e;}
assert.equal(dirname(out),base,'Output must be a direct child of the base preview site');
const seasonLabel=s=>String(s).slice(0,4)+'\u2013'+String(s).slice(-2);
const current=study.reports.at(-1),input=study.inputs.find(i=>i.season===current.season),names=new Map(input.players.map(p=>[p.player_id,p.name]));
const {default:init,BrowserEngine}=await import(pathToFileURL(join(resolve(wasmPath),'icelines_wasm.js')));
await init({module_or_path:await readFile(join(wasmPath,'icelines_wasm_bg.wasm'))});const engine=new BrowserEngine();
const plans=[];try{
 const wasm=JSON.parse(engine.achievement_catalog(await readFile(reportsPath),JSON.stringify(study.rules)));
 for(const a of current.profiles){
  const p=JSON.parse(await readFile(join(base,`profiles/${a.player_id}.json`),'utf8'));
  const old=p.achievements;assert.equal(old.games,a.games);assert.equal(old.season,a.season);assert.equal(old.group,'Goalies');
  for(const f of old.feats)assert.deepEqual(a.feats.find(n=>n.id===f.id),f,`Source snapshot differs for ${a.player_id}/${f.id}`);
  assert.deepEqual(wasm.profiles.find(n=>n.player_id===a.player_id).feats,a.feats);
  assert.deepEqual(JSON.parse(engine.task_repertoire(JSON.stringify(a))),a.task_repertoire);
  plans.push({id:a.player_id,name:names.get(a.player_id),newlyRecognized:old.earned===0&&a.earned>0,profile:{...p,achievements:a}});
 }
}finally{engine.free();}
await mkdir(join(out,'profiles'),{recursive:true});await cp(wasmPath,join(out,'pkg'),{recursive:true});
for(const p of plans)await writeFile(join(out,`profiles/${p.id}.json`),JSON.stringify(p.profile));
await writeFile(join(out,'players.json'),JSON.stringify(plans.map(({id,name,newlyRecognized})=>({id,name,newlyRecognized}))));
const first=plans.find(p=>p.newlyRecognized)??plans[0];
let html=await readFile(join(base,'index.html'),'utf8');html=html.replace(/<title>.*?<\/title>/,'<title>Rinkcraft goalie feat trial</title>').replace(/<main><p>.*?<\/p>/s,`<main><p>Frozen ${seasonLabel(current.season)} goalie sample · ${plans.length} profiles · NHL summary observations. Candidate feats are experimental.</p>`);html=html.replaceAll('href="appearance.css"','href="../appearance.css"').replaceAll('href="style.css"','href="../style.css"').replaceAll('href="rinkcraft-character.css"','href="../rinkcraft-character.css"').replace("from './src/rinkcraft-profile.js'","from '../src/rinkcraft-profile.js'");
html=html.replace("document.querySelector('#profile').replaceChildren(renderRinkcraftProfile(profile));","const card=renderRinkcraftProfile(profile),front=card.querySelector('.rk-front');if(front){const marker=document.createElement('p');marker.dataset.experimentalGoalieFeats='true';marker.style.cssText='font-size:9px;margin:0;font-weight:700;color:#183247;border-left:2px solid #a36d00;padding-left:6px';marker.textContent='EXPERIMENTAL GOALIE FEAT TRIAL · active registry unchanged';front.prepend(marker);}document.querySelector('#profile').replaceChildren(card);");
html=html.replace('RINKCRAFT / PLAYER CARDS','RINKCRAFT / GOALIE FEAT TRIAL').replace('<main>','<main><aside style="padding:12px;border:2px solid #d3a94e;margin-bottom:16px"><strong>Experimental goalie feat trial</strong><p>Two candidate definitions: Save percentage and Shot workload. The active 34-rule registry and ordinary cards are unchanged.</p><a id="active-card" href="../">Compare active card</a> · <a href="study.html">Four-season results</a></aside>').replace("??'8478402'",`??'${first.id}'`).replace("history.replaceState(null,'','?player='+id);","history.replaceState(null,'','?player='+id);document.querySelector('#active-card').href='../?player='+id;");
await writeFile(join(out,'index.html'),html);
const escape=s=>String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const rows=study.reports.map(r=>`<tr><td>${seasonLabel(r.season)}</td><td>${r.players}</td><td>${r.eligible}</td><td>${r.baseline_recognized}</td><td>${r.trial_recognized}</td><td>${r.newly_recognized}</td></tr>`).join('');
const newly=plans.filter(p=>p.newlyRecognized).map(p=>`<li><a href="?player=${p.id}">${escape(p.name)}</a></li>`).join('');
const transitions=study.transitions.map(t=>`<tr><td>${seasonLabel(t.from)} → ${seasonLabel(t.to)}</td><td>${escape(t.id)}</td><td>${t.eligible_pairs}</td><td>${t.prior_qualifiers_all}</td><td>${t.prior_qualifiers}</td><td>${t.repeat_qualifiers}</td><td>${t.without_next_eligible}</td></tr>`).join('');
await writeFile(join(out,'study.html'),`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Goalie feat trial</title><style>body{max-width:1000px;margin:30px auto;padding:16px;font:16px/1.5 system-ui;color:#183247}table{border-collapse:collapse;width:100%}th,td{padding:8px;border-bottom:1px solid #ccd5de;text-align:left}.scroll{overflow:auto}a{color:#176581}</style><h1>Two goalie feat candidates</h1><p>Opt-in trial. Active registry unchanged. Save percentage = saves / shots against. Shot workload = shots against / appearances. Existing 10-GP, 20-peer and Bronze/Silver/Gold rules remain.</p><div class="scroll"><table><tr><th>Season</th><th>Players</th><th>Eligible</th><th>Old recognized</th><th>Trial recognized</th><th>New</th></tr>${rows}</table></div><p>The 2026–27 window is partial. Recognition means at least one earned stat feat, not an ability score. Workload is deployment, not saving quality. Saving outcomes include team and shot-difficulty effects.</p><h2>Newly recognized in current sample</h2><ul>${newly}</ul><h2>Adjacent-season repeat</h2><div class="scroll"><table><tr><th>Windows</th><th>Candidate</th><th>Eligible pairs</th><th>Prior qualifiers total</th><th>Prior qualifiers in pairs</th><th>Repeated</th><th>Without next eligible row</th></tr>${transitions}</table></div><p>Changing annual references and eligible pairs only; these are descriptive repeats, not frozen predictions or validated traits. <a href="./">Open trial cards</a></p>`);
await writeFile(join(out,'validation.json'),JSON.stringify({players:plans.length,baselineFeatsPreserved:true,currentAwardWasmParity:true,repertoireWasmParity:true,activeProfilesUnchanged:true,defaultPlayer:first},(k,v)=>k==='profile'?undefined:v,2));
console.log({players:plans.length,newlyRecognized:plans.filter(p=>p.newlyRecognized).length,defaultPlayer:first.name,output:out});
