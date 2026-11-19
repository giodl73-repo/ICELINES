import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const [historicalFile,currentFile,output]=process.argv.slice(2);if(!output)throw Error('Usage: HISTORICAL_FIXTURES CURRENT_FIXTURES OUTPUT_BUNDLE');
const historical=JSON.parse(await readFile(historicalFile,'utf8')),current=JSON.parse(await readFile(currentFile,'utf8'));
assert.deepEqual(Object.keys(historical).sort(),Object.keys(current).sort());
for(const [key,p]of Object.entries(historical)){
 const c=current[key];assert.equal(p.request.player_id,c.request.player_id);assert.equal(p.profile_window.kind,'season');assert.equal(c.profile_window.kind,'current');assert.equal(c.profile_window.season,p.league_current_season);assert.ok(c.profile_window.as_of&&Number.isFinite(Date.parse(c.profile_window.as_of)));
 assert.equal(c.numerical_evidence.season,c.profile_window.season);assert.equal(c.deployment_evidence.season,c.profile_window.season);assert.equal(c.achievements.season,c.profile_window.season);assert.equal(c.experimental_patterns.season,c.profile_window.season);assert.equal(Date.parse(c.profile_window.as_of),Date.parse(c.capture_period.through),'Current as-of must match the captured frame.');
 assert.deepEqual(c.character_sheet,p.character_sheet,'Historical dossier must retain its original dates and assertions.');
 const option=p.profile_window.choices.find(o=>o.kind==='current');assert.ok(option);option.available=true;option.label=c.profile_window.label;
 p.scope_variants={current:c};
}
await writeFile(output,JSON.stringify(historical));console.log('Bundled five complete seasonal/current alternatives; career remains gated.');
