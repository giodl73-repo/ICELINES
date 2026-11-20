// Captured public NHL reports -> shared Rust/WASM workload evidence. No network.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,dirname,join,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [engineDirectory,inputFile,baselineFile,outputDirectory]=process.argv.slice(2);
if(!outputDirectory)throw Error('Usage: node tools/project_rinkcraft_deployment.mjs WASM_PACKAGE_DIRECTORY CAPTURE_MANIFEST_JSON BASE_FIXTURES_JSON OUTPUT_DIRECTORY');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const manifestRaw=await readFile(resolve(inputFile)),baseRaw=await readFile(resolve(baselineFile));
const input=JSON.parse(manifestRaw),profiles=JSON.parse(baseRaw),engineDir=resolve(engineDirectory);
assert.equal(input.schema,'icelines.rinkcraft-deployment-input.v1');assert.ok(Array.isArray(input.captures)&&input.captures.length);
assert.ok(profiles&&typeof profiles==='object'&&!Array.isArray(profiles)&&Object.keys(profiles).length,'Nonempty explicit profile map required.');
const {BrowserEngine,initSync}=await import(pathToFileURL(join(engineDir,'icelines_wasm.js')).href);
initSync({module:await readFile(join(engineDir,'icelines_wasm_bg.wasm'))});const engine=new BrowserEngine(),before=engine.resident_revisions(),receipts=[];
try{
 for(const [key,p]of Object.entries(profiles)){
  const old=structuredClone(p),id=p.request?.player_id;
  assert.ok(id&&p.character_sheet?.player_id===id&&p.character_sheet?.player_name===p.player_name,'Explicit consistent actor identity required.');
  const season=p.numerical_evidence?.season;assert.ok(season&&p.latest_available?.season===season,'Exact numerical/record season required.');
  const gameType=p.request.season_type==='regular'?2:p.request.season_type==='playoff'?3:null;assert.ok(gameType);
  const goalie=['Goalie','Goalies','G'].includes(p.latest_available.position);
  const captures=input.captures.filter(c=>c.player_ids?.includes(id)&&c.season===season&&c.game_type===gameType&&c.goalie===goalie);
  assert.equal(captures.length,1,'Exactly one matching captured report required.');const capture=captures[0];
  const file=isAbsolute(capture.path)?capture.path:resolve(dirname(resolve(inputFile)),capture.path),raw=await readFile(file);assert.equal(hash(raw),capture.sha256,'Capture hash mismatch.');
  const card=JSON.parse(engine.deployment_report(raw,id,season,gameType,goalie,capture.url));
  assert.equal(card.player_id,id);assert.equal(card.season,season);assert.equal(card.season_type,p.request.season_type);assert.equal(card.games,p.latest_available.games,'Usage denominator must match the displayed season record.');
  assert.equal(card.source_sha256,capture.sha256);p.deployment_evidence=card;
  const next=structuredClone(p);delete next.deployment_evidence;delete old.deployment_evidence;assert.deepEqual(next,old);
  receipts.push({key,player_id:id,season,game_type:gameType,source_sha256:card.source_sha256,prior_fields_unchanged:true});
 }
 assert.equal(engine.resident_revisions(),before,'Stateless workload projection must not mutate resident packages.');
}finally{engine.free();}
const out=resolve(outputDirectory);await mkdir(out,{recursive:true});
await writeFile(join(out,'fixtures.json'),JSON.stringify(profiles));
await writeFile(join(out,'projection.json'),JSON.stringify({schema:'icelines.deployment-projection.v1',input_manifest:resolve(inputFile),input_sha256:hash(manifestRaw),baseline:resolve(baselineFile),baseline_sha256:hash(baseRaw),engine_directory:engineDir,receipts,limits:'Descriptive season workload. Even strength is not exclusively 5v5; shares are of own TOI, not team opportunities. No duty, unit, station, skill or rating inferred.'},null,2));
console.log(`Projected ${receipts.length} source-validated workload cards; prior profile fields unchanged.`);
