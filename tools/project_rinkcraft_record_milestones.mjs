// Additive-only projection: no changes to existing statistics, ranks or feats.
import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const [baseline,patchFile,auditFile,output]=process.argv.slice(2);if(!output)throw Error('Usage: THREE_SCOPE_BUNDLE RECORD_PATCHES SAVE_AUDIT OUTPUT');
const profiles=JSON.parse(await readFile(baseline,'utf8')),original=structuredClone(profiles),patches=JSON.parse(await readFile(patchFile,'utf8')),audit=JSON.parse(await readFile(auditFile,'utf8'));
assert.deepEqual(Object.keys(profiles).sort(),Object.keys(patches).sort());
for(const [key,p]of Object.entries(profiles)){
 const c=p.scope_variants.career,w=c.profile_window,r=patches[key];
 assert.equal(r.schema,'rinkcraft.career-records.v1');assert.equal(w.kind,'career');assert.equal(w.coverage,'career_reconciled');assert.equal(r.scope_kind,'career');assert.equal(r.player_id,p.request.player_id);assert.equal(r.player_id,w.player_id);assert.equal(r.first_season,w.first_season);assert.equal(r.last_season,w.season);assert.equal(r.season_type,w.season_type);assert.equal(r.as_of,w.as_of);assert.equal(r.group,c.numerical_evidence.group);assert.equal(r.source_input_sha256,c.deployment_evidence.source_sha256);
 assert.equal(c.record_milestones,undefined);c.record_milestones=r;
 if(r.player_id===audit.actor){
  assert.equal(audit.schema,'rinkcraft.save-source-audit.v1');assert.equal(audit.state,'localized_unresolved');
  const f=c.numerical_evidence.career_audit.fields.find(f=>f.key==='saves');assert.equal(f.state,'conflicting_sources');const year=f.conflict_seasons.find(y=>y.season===audit.season);assert.equal(year.reported_saves,audit.season_totals.reported_saves);assert.equal(year.shots_against,audit.season_totals.shots_against);assert.equal(year.goals_against,audit.season_totals.goals_against);c.numerical_evidence.career_audit.investigation=audit;
 }
}
const stripped=structuredClone(profiles);for(const p of Object.values(stripped)){delete p.scope_variants.career.record_milestones;delete p.scope_variants.career.numerical_evidence.career_audit.investigation;}assert.deepEqual(stripped,original);
await writeFile(output,JSON.stringify(profiles));console.log('Attached career-only record markers and unresolved source investigation; prior payloads preserved.');
