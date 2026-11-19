// Merge Rust-authored additions without reserializing existing floats through Rust.
import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const [baseline,patchesFile,output]=process.argv.slice(2);
if(!output)throw Error('Usage: BASE_PROFILE_FIXTURES RUST_SCOPE_PATCHES OUTPUT_FIXTURES');
const profiles=JSON.parse(await readFile(baseline,'utf8')),patches=JSON.parse(await readFile(patchesFile,'utf8'));
assert.deepEqual(Object.keys(profiles).sort(),Object.keys(patches).sort());
for(const [key,p]of Object.entries(profiles)){
 const before=structuredClone(p),patch=patches[key];assert.deepEqual(Object.keys(patch).sort(),['experimental_patterns','profile_window']);
 const w=patch.profile_window,a=patch.experimental_patterns;
 assert.equal(w.kind,'season');assert.equal(w.player_id,p.request.player_id);assert.equal(a.player_id,p.request.player_id);
 assert.equal(w.season,p.request.first_season);assert.equal(w.season,p.request.last_season);assert.equal(w.season,p.numerical_evidence.season);
 assert.equal(a.season,w.season);assert.equal(w.season_type,p.request.season_type);assert.equal(a.season_type,p.request.season_type);assert.equal(a.group,p.numerical_evidence.group);
 for(const pattern of a.patterns)assert.ok(pattern.history.every(h=>h.season<=w.season));
 Object.assign(p,patch);const preserved=structuredClone(p);delete preserved.profile_window;delete preserved.experimental_patterns;assert.deepEqual(preserved,before);
}
await writeFile(output,JSON.stringify(profiles));console.log('Merged scope patches with all prior profile values preserved.');
