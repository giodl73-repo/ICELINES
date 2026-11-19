// Assemble explicit whole-profile variants. Never merge statistics across scopes.
import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const [historicalFile,currentFramesFile,output]=process.argv.slice(2);
if(!output)throw Error('Usage: HISTORICAL_FIXTURES CURRENT_FRAME_PATCHES OUTPUT_FIXTURES');
const historical=JSON.parse(await readFile(historicalFile,'utf8')),frames=JSON.parse(await readFile(currentFramesFile,'utf8'));
assert.deepEqual(Object.keys(historical).sort(),Object.keys(frames).sort());
for(const [key,base]of Object.entries(historical)){
 const patch=frames[key];assert.equal(patch.request.player_id,base.request.player_id);assert.equal(patch.request.first_season,base.league_current_season);assert.equal(patch.request.first_season,patch.request.last_season);
 const current={...structuredClone(base),...patch};delete current.profile_window;delete current.experimental_patterns;delete current.scope_variants;
 assert.equal(current.numerical_evidence.season,current.request.first_season);assert.equal(current.deployment_evidence.season,current.request.first_season);assert.equal(current.achievements.season,current.request.first_season);
 assert.equal(current.numerical_evidence.games,current.latest_available.games);assert.equal(current.deployment_evidence.games,current.latest_available.games);assert.equal(current.achievements.games,current.latest_available.games);
 assert.equal(current.numerical_history,null);assert.deepEqual(current.attribute_seasons,[]);
 historical[key]=current;
}
await writeFile(output,JSON.stringify(historical));console.log('Assembled explicit current frames with historical dossier provenance retained.');
