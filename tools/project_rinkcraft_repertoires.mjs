// Explicit offline catalog projection through the shared Rust selector.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [engineDirectory,catalogFile,fixtureFile,outputDirectory]=process.argv.slice(2);
if(!outputDirectory)throw Error('Usage: node tools/project_rinkcraft_repertoires.mjs WASM_PACKAGE_DIRECTORY CATALOG_JSON BASE_FIXTURES_JSON OUTPUT_DIRECTORY');
const directory=resolve(engineDirectory),catalogRaw=await readFile(resolve(catalogFile),'utf8'),fixtureRaw=await readFile(resolve(fixtureFile),'utf8');
const {BrowserEngine,initSync}=await import(pathToFileURL(join(directory,'icelines_wasm.js')).href);
initSync({module:await readFile(join(directory,'icelines_wasm_bg.wasm'))});
const engine=new BrowserEngine(),fixtures=JSON.parse(fixtureRaw),rows=[];
if(!fixtures||Array.isArray(fixtures)||!Object.keys(fixtures).length)throw Error('A nonempty explicit profile map is required.');
try{
 for(const [key,p] of Object.entries(fixtures)){
  const before=structuredClone(p),old=p.character_sheet;
  if(!old?.player_id||!old?.as_of_season)throw Error('Explicit documentary identity/date required; no fallback.');
  if(p.request?.player_id!==old.player_id||p.player_name!==old.player_name)throw Error('Fixture identity mismatch; no cross-player joins.');
  const sheet=JSON.parse(engine.character_sheet(catalogRaw,old.player_id,old.as_of_season));
  if(!sheet||sheet.player_id!==old.player_id||sheet.player_name!==p.player_name)throw Error('Requested dossier is unsupported or mismatched; no fallback.');
  p.character_sheet=sheet;
  const rest=structuredClone(p),prior=structuredClone(before);delete rest.character_sheet;delete prior.character_sheet;
  assert.deepEqual(rest,prior);
  // A repertoire revision must not silently change the accepted identity/methods.
  for(const field of ['primary','secondary','specialty','summary','features','traits'])assert.deepEqual(sheet[field],old[field],field);
  rows.push({key,player_id:sheet.player_id,documentary_as_of:sheet.as_of_season,statistics_season:p.numerical_evidence?.season??null,pressure:sheet.under_pressure?.length??0,skills:sheet.task_skills?.length??0,catalog_sha256:sheet.catalog_sha256,numerical_fields_unchanged:true,identity_methods_unchanged:true});
 }
}finally{engine.free();}
const out=resolve(outputDirectory);await mkdir(out,{recursive:true});
await writeFile(join(out,'fixtures.json'),JSON.stringify(fixtures));
const sha=s=>createHash('sha256').update(s).digest('hex');
await writeFile(join(out,'projection.json'),JSON.stringify({schema:'icelines.repertoire-projection.v1',catalog_input:resolve(catalogFile),catalog_raw_sha256:sha(catalogRaw),base_fixtures:resolve(fixtureFile),base_sha256:sha(fixtureRaw),engine_directory:directory,rows,limits:'Dated curated documentary selections, not observed task rates, current-season continuity or capability grades. Numeric fields and existing identity/methods are preserved.'},null,2));
console.log(JSON.stringify({status:'passed',players:rows.length,pressure:rows.reduce((n,r)=>n+r.pressure,0),skills:rows.reduce((n,r)=>n+r.skills,0)}));
