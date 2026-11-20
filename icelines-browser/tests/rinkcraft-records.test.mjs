import test from 'node:test';
import assert from 'node:assert/strict';
import {matchesRecordWindow,renderRecordMilestones} from '../dist/src/rinkcraft-records.js';
const w={kind:'career',coverage:'career_reconciled',player_id:1,first_season:20152016,season:20262027,season_type:'regular',as_of:'2026-11-19T12:00:00Z'};
const r={schema:'rinkcraft.career-records.v1',scope_kind:'career',player_id:1,first_season:20152016,last_season:20262027,season_type:'regular',as_of:w.as_of,group:'Forwards',source_input_sha256:'abc',tracks:[],notice:'Literal count markers'};
test('career records bind actor, complete window, group and source',()=>{
 assert.equal(matchesRecordWindow(r,w,'Forwards','abc'),true);
 for(const [key,value]of [['player_id',2],['first_season',20142015],['last_season',20252026],['season_type','playoff'],['as_of','2026-11-18T12:00:00Z'],['scope_kind','season'],['schema','other'],['source_input_sha256','other']])assert.equal(matchesRecordWindow({...r,[key]:value},w,'Forwards','abc'),false);
 for(const kind of ['season','current'])assert.equal(matchesRecordWindow(r,{...w,kind},'Forwards','abc'),false);
 assert.equal(matchesRecordWindow(r,w,'Goalies','abc'),false);
});
test('only two highest markers appear upfront; unknown and real zero remain distinct',()=>{
 class N {children=[];dataset={};constructor(tag){this.tag=tag;}append(...n){this.children.push(...n);}set textContent(v){this.text=v;}}
 globalThis.document={createElement:t=>new N(t)};
 const track=(id,value,highest)=>({id,name:id,unit:'points',raw_value:value,value,highest,next:100,remaining_raw:value===null?null:100-value,divisor:1,state:value===null?'unavailable':highest===null?'not_reached':'recorded',steps:[{threshold:100,earned:value===null?null:value>=100}]});
 const root=renderRecordMilestones({...r,tracks:[track('a',200,100),track('b',200,100),track('c',200,100),track('unknown',null,null),track('zero',0,null)]});
 assert.equal(root.children.filter(n=>n.dataset.recordPrimary==='true').length,2);
 const rows=root.children.at(-1).children.filter(n=>n.dataset.recordTrack);
 assert.equal(rows.length,5);assert.equal(rows[3].children[0].children[1].text,'Unavailable');assert.match(rows[4].children[0].children[1].text,/0 recorded/);
});
