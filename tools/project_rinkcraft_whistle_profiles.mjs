// Apply only approved wording and Rust-derived signed records, preserving old awards.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const [baseline,patchFile,output]=process.argv.slice(2);if(!output)throw Error('Usage: THREE_SCOPE_FIXTURES WHISTLE_PATCHES OUTPUT');
const bytes=await readFile(baseline),profiles=JSON.parse(bytes),original=structuredClone(profiles),patches=JSON.parse(await readFile(patchFile,'utf8'));
assert.equal(patches.schema,'rinkcraft.whistle-profile-patches.v1');assert.equal(patches.input_sha256,createHash('sha256').update(bytes).digest('hex'));assert.deepEqual(Object.keys(profiles).sort(),Object.keys(patches.patches).sort());
for(const [key,p]of Object.entries(profiles)){
 const seen=new Set();for(const patch of patches.patches[key]){
  assert.ok(['season','current'].includes(patch.scope));assert.ok(!seen.has(patch.scope));seen.add(patch.scope);
  const frame=patch.scope==='season'?p:p.scope_variants[patch.scope],w=frame.profile_window,patterns=frame.experimental_patterns;assert.equal(patch.player_id,p.request.player_id);assert.equal(w.kind,patch.scope);
  for(const k of ['player_id','season','season_type','as_of'])assert.equal(patch[k],w[k]);assert.equal(patch.group,patterns.group);assert.equal(patch.source_sha256,patterns.source_sha256);assert.equal(patch.id,'penalty-balance');assert.equal(patch.name,'Relative Whistle Record');
  if(patch.scope==='season')assert.equal(patch.verified_season_reports_sha256,patterns.source_sha256);
  const targets=patterns.patterns.filter(r=>r.id===patch.id);assert.equal(targets.length,1);const target=targets[0];for(const k of ['penalties_drawn','penalties_taken']){if(target.opportunities[k]!=null)assert.equal(patch.opportunities[k],target.opportunities[k]);target.opportunities[k]=patch.opportunities[k];}if(patch.whistle_record!==null){assert.equal(patch.whistle_record.schema,'rinkcraft.whistle-record.v1');assert.equal(patch.whistle_record.drawn,target.opportunities.penalties_drawn);assert.equal(patch.whistle_record.taken,target.opportunities.penalties_taken);}target.name=patch.name;target.whistle_record=patch.whistle_record;
 }
 const expected=[p,p.scope_variants.current].filter(f=>f.experimental_patterns.patterns.some(r=>r.id==='penalty-balance')).length;assert.equal(seen.size,expected);
}
const stripped=structuredClone(profiles);for(const [key,p]of Object.entries(stripped))for(const [kind,frame]of [['season',p],['current',p.scope_variants.current]]){const old=kind==='season'?original[key]:original[key].scope_variants.current;for(const t of frame.experimental_patterns.patterns){if(t.id!=='penalty-balance')continue;const before=old.experimental_patterns.patterns.find(r=>r.id===t.id);t.name=before.name;t.opportunities=before.opportunities;if('whistle_record' in before)t.whistle_record=before.whistle_record;else delete t.whistle_record;}}assert.deepEqual(stripped,original);
await writeFile(output,JSON.stringify(profiles));console.log('Updated relative whistle wording and signed records only; old awards and numeric payloads preserved.');
