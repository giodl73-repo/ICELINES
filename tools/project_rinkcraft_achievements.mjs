// Explicit full-cohort stat-feat calculation via shared Rust. No network/quotes.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [engineDirectory,reportsFile,rulesFile,baselineFile,outputDirectory]=process.argv.slice(2);
if(!outputDirectory)throw Error('Usage: node tools/project_rinkcraft_achievements.mjs WASM_PACKAGE REPORTS_JSON RULES_JSON BASE_FIXTURES_JSON OUTPUT_DIRECTORY');
const raw=await readFile(resolve(reportsFile)),rules=await readFile(resolve(rulesFile),'utf8'),base=await readFile(resolve(baselineFile),'utf8'),profiles=JSON.parse(base);
assert.ok(profiles&&!Array.isArray(profiles)&&Object.keys(profiles).length,'Explicit profiles required.');
const engineDir=resolve(engineDirectory),{BrowserEngine,initSync}=await import(pathToFileURL(join(engineDir,'icelines_wasm.js')).href);initSync({module:await readFile(join(engineDir,'icelines_wasm_bg.wasm'))});const engine=new BrowserEngine();let catalog;
try{catalog=JSON.parse(engine.achievement_catalog(raw,rules));}finally{engine.free();}
const receipts=[];
for(const [key,p]of Object.entries(profiles)){
 const prior=structuredClone(p),id=p.request?.player_id;assert.ok(id&&p.character_sheet?.player_id===id&&p.character_sheet?.player_name===p.player_name,'Actor mismatch.');
 const found=catalog.profiles.filter(a=>a.player_id===id);assert.equal(found.length,1);const a=found[0];assert.equal(a.season,p.numerical_evidence?.season);assert.equal(a.season_type,p.request.season_type);assert.equal(a.games,p.latest_available?.games);assert.equal(a.group,p.numerical_evidence?.group);
 p.achievements=a;const next=structuredClone(p);delete next.achievements;delete prior.achievements;assert.deepEqual(next,prior);receipts.push({key,player_id:id,earned:a.earned,rule_count:a.feats.length,prior_fields_unchanged:true});
}
const out=resolve(outputDirectory);await mkdir(out,{recursive:true});await writeFile(join(out,'fixtures.json'),JSON.stringify(profiles));await writeFile(join(out,'achievement-catalog.json'),JSON.stringify(catalog));
await writeFile(join(out,'projection.json'),JSON.stringify({protocol:catalog.protocol,source_input:resolve(reportsFile),source_sha256:createHash('sha256').update(raw).digest('hex'),rules_file:resolve(rulesFile),baseline:resolve(baselineFile),engine_directory:engineDir,cohort_profiles:catalog.profiles.length,receipts,limit:'Authored experimental stat conjunctions, not task mastery, classes or ability ratings. No pressure or event-chain inference.'},null,2));
console.log(JSON.stringify({cohort:catalog.profiles.length,players:receipts}));
