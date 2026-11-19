import test from 'node:test';
import assert from 'node:assert/strict';
import {matchesObservations,renderObservations} from '../dist/src/rinkcraft-observations.js';
class Element {
 children=[];dataset={};constructor(tag){this.tag=tag;}
 append(...nodes){this.children.push(...nodes);}
 set textContent(v){this.text=v;this.children=[];}
 get textContent(){return (this.text??'')+this.children.map(n=>n.textContent).join(' ');}
}
globalThis.document={createElement:tag=>new Element(tag)};
const all=n=>[n,...n.children.flatMap(all)];
const a={player_id:1,season:20242025,season_type:'regular',group:'Forwards',games:60,source_sha256:'a'.repeat(64)};
const o={schema:'rinkcraft.profile-observations.v1',policy:'lower-tail-observations.v0.1-exploratory',...a,analysis_sha256:'b'.repeat(64),positive_count:4,caution_count:2,coverage_gaps:['Entry denial'],note:'Overlapping entries, not independent abilities.',low_callouts:[{id:'low-blocks',kind:'low_activity',label:'Low recorded Blocks / GP',percentile:9,reference_group:'Forwards',condition:{key:'blocks',value:0.2,unit:'PerGame',percentile:9,peers:456,numerator:12,denominator:60},interpretation:'Low activity is not proof of poor defending.'}]};
test('activity callout preserves the measured context, overlap warning and unknowns',()=>{
 const root=renderObservations({...a,profile_observations:o});assert.ok(root);
 assert.match(root.textContent,/4 positive · 2 caution · 1 low/);
 assert.match(root.textContent,/Low Blocks \/ GP.*P9.0.*Low activity.*0.200 \/ GP.*456 Forwards/);
 assert.match(root.textContent,/not proof of poor defending/);
 assert.match(root.textContent,/Assessment gaps.*Entry denial.*Unmeasured here; not player weaknesses/);
 assert.ok(!root.textContent.includes('level'));
 assert.equal(all(root).find(e=>e.dataset.observationId)?.dataset.observationKind,'low_activity');
});
test('actor, season, games, group and input lineage must match',()=>{
 for(const [key,value]of [['player_id',2],['season',20252026],['season_type','playoff'],['games',59],['group','Defense'],['source_sha256','c'.repeat(64)],['analysis_sha256','unknown']])assert.equal(matchesObservations({...o,[key]:value},a),false,key);
 assert.equal(renderObservations(a),null);
 assert.equal(matchesObservations({...o,group:'Goalies'},{...a,group:'Goalies'}),false);
});
test('invalid ranks, missing measurements, repeated observations and small cohorts are withheld',()=>{
 for(const change of [{percentile:null},{percentile:21},{condition:{...o.low_callouts[0].condition,value:null}},{condition:{...o.low_callouts[0].condition,peers:19}},null]){
  const low=change===null?null:{...o.low_callouts[0],...change};
  assert.equal(matchesObservations({...o,low_callouts:[low]},a),false);
 }
 assert.equal(matchesObservations({...o,low_callouts:[o.low_callouts[0],o.low_callouts[0]]},a),false);
});
test('zero measured output remains visible while empty annotations do not invent a weakness',()=>{
 const zero={...o.low_callouts[0],id:'low-goals',kind:'low_output',label:'Low recorded Goals / GP',condition:{...o.low_callouts[0].condition,key:'goals',value:0}};
 assert.match(renderObservations({...a,profile_observations:{...o,low_callouts:[zero]}}).textContent,/Low output · 0.000/);
 const empty=renderObservations({...a,profile_observations:{...o,low_callouts:[]}});
 assert.equal(all(empty).filter(e=>e.dataset.observationId).length,0);
 assert.match(empty.textContent,/0 low/);
});
