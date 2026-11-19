// Attach verified career records; retain both existing alternatives exactly.
import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const [baseline,patchFile,output]=process.argv.slice(2);if(!output)throw Error('Usage: EXISTING_SCOPE_BUNDLE CAREER_PATCHES OUTPUT_BUNDLE');
const profiles=JSON.parse(await readFile(baseline,'utf8')),patches=JSON.parse(await readFile(patchFile,'utf8'));
assert.deepEqual(Object.keys(profiles).sort(),Object.keys(patches).sort());
for(const [key,p]of Object.entries(profiles)){
 const patch=patches[key],id=p.request.player_id;assert.equal(patch.request.player_id,id);assert.equal(patch.profile_window.kind,'career');assert.equal(patch.profile_window.coverage,'career_reconciled');assert.equal(patch.career_reconciliation.state,'core_counts_reconciled');
 assert.equal(patch.profile_window.first_season,patch.request.first_season);assert.equal(patch.profile_window.season,patch.request.last_season);assert.equal(patch.numerical_evidence.scope_kind,'career');assert.ok(patch.numerical_evidence.scope_label);assert.equal(patch.deployment_evidence.scope_kind,'career');
 assert.equal(patch.numerical_evidence.games,patch.latest_available.games);assert.equal(patch.deployment_evidence.games,patch.latest_available.games);assert.equal(patch.latest_available.games,patch.career_reconciliation.counts.games);assert.ok(patch.profile_window.as_of&&Number.isFinite(Date.parse(patch.profile_window.as_of)));assert.equal(Date.parse(patch.profile_window.as_of),Date.parse(patch.capture_period.through));
 assert.ok(patch.numerical_evidence.metrics.every(m=>m.percentile===null&&m.measured_peers===0));assert.equal(patch.achievements,null);assert.deepEqual(patch.experimental_patterns.patterns,[]);
 const c={...structuredClone(p),...patch};delete c.scope_variants;
 const label=patch.profile_window.label;
 for(const frame of [p,p.scope_variants.current]){const choice=frame.profile_window.choices.find(o=>o.kind==='career');assert.ok(choice);choice.available=true;choice.label=label;}
 p.scope_variants.career=c;assert.deepEqual(c.character_sheet,p.character_sheet);
}
await writeFile(output,JSON.stringify(profiles));console.log('Bundled reconciled career alternatives without career ranks or seasonal feat reuse.');
