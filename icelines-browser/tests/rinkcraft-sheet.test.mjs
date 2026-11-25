import test from 'node:test';
import assert from 'node:assert/strict';
import {renderCharacterCard} from '../dist/src/rinkcraft-character.js';
import {renderStatScore} from '../dist/src/rinkcraft-stat-scores.js';
import {renderStatFeats} from '../dist/src/rinkcraft-achievements.js';

test('overall draw breakdown uses manpower labels while late draws retain score context',async()=>{
 const {pressureEntries}=await import('../dist/src/rinkcraft-under-pressure.js');
 const e={id:'draws',name:'Contested draws',result:'4/10 won',challenge:'A draw',response:'Win',definition:'Credited wins',source:'NHL',coverage:'A frozen sample',captured_at:'2026-11-24T00:00:00Z',note:'Not retained possession',breakdown:[{key:'advantage',name:'Skater advantage',result:'3/4 won',opportunities:4,successes:3}]};
 const view=pressureEntries({entries:[e]});assert.match(view.textContent,/skater balance.*both goalies present.*Skater advantage.*3\/4 won/);assert.doesNotMatch(view.textContent,/score before/);
 assert.match(pressureEntries({entries:[{...e,id:'late-draws'}]}).textContent,/score before/);
});

test('point mix shows supplied composition without treating assist detail as extra points',async()=>{
 const {productionMix,renderProductionMix}=await import('../dist/src/rinkcraft-production-mix.js');
 const r={schema:'rinkcraft.production-mix.v1',player_id:1,season:20262027,group:'Forwards',games:4,minutes:80,headline:'25.0% goal share',rows:[{label:'Goals',rate:'1.00 /60',share:'25.0% of points'},{label:'Assists',rate:'3.00 /60',share:'75.0% of points'}],assist_detail:[{label:'Primary assists',rate:'2.00 /60',share:'50.0% of points'},{label:'Secondary assists',rate:'1.00 /60',share:'25.0% of points'}],note:'Recorded production, not a class.'};
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',games:4,minutes:80,scope:'all strengths',small_sample:true,production_mix:r,scores:['Skating','Handling','Passing','Shooting','Reading','Defending'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'}))};
 assert.equal(productionMix(stats),r);
 const args=Array(18).fill(undefined);args[0]='Test';args[1]=sheet;args[3]={season:20262027,games:4,position:'Center',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];
 assert.match(classed(renderCharacterCard(...args),'rk-front-traits')[0].textContent,/Point mix.*25.0% goal share.*Assist detail.*included above.*Primary assists/);
 for(const stale of [{...r,player_id:2},{...r,season:20252026},{...r,games:3},{...r,minutes:81},{...r,group:'Goalies'}])assert.equal(productionMix({...stats,production_mix:stale}),undefined);
 assert.match(renderProductionMix({...r,assist_detail:[]}).textContent,/Matched primary\/secondary assist detail unavailable/);
 assert.equal(productionMix({...stats,group:'Goalies',production_mix:{...r,group:'Goalies'}}),undefined);assert.equal(productionMix(undefined),undefined);
});

test('penalty balance renders supplied rates within tendencies and withholds stale records',async()=>{
 const {penaltyBalance}=await import('../dist/src/rinkcraft-penalty-balance.js');
 const r={schema:'rinkcraft.penalty-balance.v1',player_id:1,season:20262027,group:'Forwards',games:20,minutes:400,drawn:'1.23 /60',taken:'0.79 /60',balance:'+0.45 /60',formula:'Drawn minus taken before rounding.',note:'Recorded calls, not power plays created.'};
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',games:20,minutes:400,scope:'all strengths',small_sample:false,penalty_balance:r,scores:['Skating','Handling','Passing','Shooting','Reading','Defending'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'}))};
 assert.equal(penaltyBalance(stats),r);
 const args=Array(18).fill(undefined);args[0]='Test';args[1]=sheet;args[3]={season:20262027,games:20,position:'Center',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];
 assert.match(classed(renderCharacterCard(...args),'rk-front-traits')[0].textContent,/Penalty balance \+0.45.*Drawn.*1.23.*Taken.*0.79.*not power plays/);
 for(const stale of [{...r,player_id:2},{...r,season:20252026},{...r,games:19},{...r,minutes:401},{...r,group:'Goalies'}])assert.equal(penaltyBalance({...stats,penalty_balance:stale}),undefined);
 assert.equal(penaltyBalance({...stats,group:'Goalies',penalty_balance:{...r,group:'Goalies'}}),undefined);
 delete stats.penalty_balance;assert.equal(classed(renderCharacterCard(...args),'rk-penalty-balance').length,0);
});

test('Under Pressure shows Rust response samples and gaps without inventing ratings',()=>{
 const pressure={schema:'rinkcraft.under-pressure.v1',player_id:1,season:20262027,group:'Forwards',games:20,entries:[{id:'late-draws',name:'Late-close draws',challenge:'A late draw',response:'Win draw',opportunities:4,successes:1,result:'1/4 won / 25.0%',definition:'Third period, last five minutes, one-goal gap.',source:'NHL',captured_at:'2026-11-24T00:00:00Z',coverage:'200 finalized games',note:'Not retained possession.'}],gaps:['Press resistance: no measured opportunities.'],note:'Observed outcomes, not abilities.'};
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',games:20,minutes:400,scope:'all strengths',small_sample:false,under_pressure:pressure,scores:['Skating','Handling','Passing','Shooting','Reading','Defending'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'}))};
 const args=Array(18).fill(undefined);args[0]='Skater';args[3]={season:20262027,games:20,position:'Center',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];
 pressure.entries[0].breakdown=[{key:'tied',name:'Tied',opportunities:4,successes:1,result:'1/4 won / 25.0%'},{key:'leading',name:'Leading by one',opportunities:0,successes:0,result:'No observed opportunities'},{key:'trailing',name:'Trailing by one',opportunities:0,successes:0,result:'No observed opportunities'}];
 const front=renderCharacterCard(...args);assert.match(classed(front,'rk-front-pressure')[0].textContent,/Late-close draws.*1\/4 won.*200 finalized games.*Press resistance/);assert.equal(classed(front,'rk-pressure-entry').length,1);assert.match(classed(front,'rk-pressure-breakdown')[0].textContent,/Tied.*1\/4 won.*Leading by one.*No observed opportunities/);
 for(const stale of [{...pressure,player_id:2},{...pressure,games:19},{...pressure,season:20252026},{...pressure,group:'Goalies'}]){stats.under_pressure=stale;assert.equal(classed(renderCharacterCard(...args),'rk-pressure-entry').length,0);}
 stats.under_pressure={...pressure,entries:[]};const empty=classed(renderCharacterCard(...args),'rk-front-pressure')[0];assert.equal(classed(empty,'rk-pressure-entry').length,0);assert.match(empty.textContent,/Press resistance/);assert.doesNotMatch(empty.textContent,/0\/0|0%/);
});

test('context actions render without faceoffs or invented outcome rates',async()=>{
 const {pressureEntries}=await import('../dist/src/rinkcraft-under-pressure.js');
 const view=pressureEntries({entries:[],actions:[{name:'Protecting lead',result:'3 blocks · 0 takeaways',baseline:'All situations: 25 blocks · 5 takeaways',definition:'Late regulation, leading by one.',coverage:'20 roster games',exposure:'Matched shift subset: 1:30 on ice; same-game counts.',captured_at:'2026-11-24T00:00:00Z',note:'Action counts only; no success rate.'}]});
 assert.equal(classed(view,'rk-pressure-action').length,1);
 assert.match(view.textContent,/Context actions.*counts only.*Protecting lead.*3 blocks.*0 takeaways.*All situations.*20 roster games/);
 assert.match(view.textContent,/Matched shift subset.*1:30 on ice/);assert.doesNotMatch(view.textContent,/%|opportunities/);
 assert.equal(classed(pressureEntries({entries:[]}),'rk-pressure-action').length,0);
});

test('observed unit links preserve Rust values, ties and exact selected metadata',async()=>{
 const {unitLinks,renderUnitLinks,renderUnitDeployment}=await import('../dist/src/rinkcraft-unit-links.js');
 const g={team:'NYR',partners_display:'Player Two',time_display:'1:40',share_display:'66.7%'};
 const r={schema:'rinkcraft.unit-links.v1',player_id:1,season:20262027,group:'Defense',games:20,game_ids:[2026020001],coverage:'1 matched appearance / 2026-10-01',contexts:[{key:'5v5',label:'5v5 pair',seconds:150,time_display:'2:30',total_groups:2,tied_leaders:1,groups:[g]},{key:'5v4',label:'PP / 5v4',seconds:0,time_display:'0:00',total_groups:0,tied_leaders:0,groups:[]},{key:'4v5',label:'PK / 4v5',seconds:0,time_display:'0:00',total_groups:0,tied_leaders:0,groups:[]}],partners:[],recent:{game_ids:[2026020001],first_date:'2026-10-01',last_date:'2026-10-01',leading:g,tied_leaders:1,different_from_full:false},remainder_display:'Other 0:00 / unresolved 0:00',note:'Observed shared time.'};
 const p={player_id:1,season:20262027,group:'Defense',games:20,unit_links:r};assert.equal(unitLinks(p),r);for(const stale of [{...r,player_id:2},{...r,season:20252026},{...r,games:19},{...r,group:'Forwards'}])assert.equal(unitLinks({...p,unit_links:stale}),undefined);
 const view=renderUnitLinks(r);assert.match(classed(view,'rk-unit-summary')[0].textContent,/Player Two.*1:40.*66.7%/);assert.match(view.textContent,/Last 1 matched appearances.*Same leading combination/);assert.match(renderUnitDeployment(r).textContent,/Matched shifts.*5v5.*2:30.*5v4.*0:00/);
 const tied=renderUnitLinks({...r,contexts:[{...r.contexts[0],tied_leaders:2},...r.contexts.slice(1)]});assert.match(classed(tied,'rk-unit-summary')[0].textContent,/2 co-leading groups/);assert.doesNotMatch(classed(tied,'rk-unit-summary')[0].textContent,/Player Two/);assert.equal(unitLinks(undefined),undefined);
});

test('skating detail renders supplied measurements and withholds stale or goalie data',async()=>{
 const {skatingDetail,renderSkatingDetail}=await import('../dist/src/rinkcraft-skating-detail.js');
 const r={schema:'rinkcraft.skating-detail.v1',player_id:1,season:20262027,group:'Defense',games:4,minutes:80,headline:'22.12 mph',secondary:'20+ mph bursts 6.50 /60',rows:[{key:'peak-speed',label:'Reported peak speed',value:'22.12 mph',comparison:null},{key:'bursts',label:'20+ mph bursts',value:'6.50 /60',comparison:null}],note:'Recorded inputs, not acceleration.'};
 const p={player_id:1,season:20262027,group:'Defense',games:4,minutes:80,skating_detail:r};assert.equal(skatingDetail(p),r);const body=renderSkatingDetail(r);assert.match(body.textContent,/22.12 mph.*6.50 \/60.*Measures & scope.*Recorded inputs/);
 for(const stale of [{...r,player_id:2},{...r,season:20252026},{...r,games:3},{...r,minutes:81},{...r,group:'Goalies'}])assert.equal(skatingDetail({...p,skating_detail:stale}),undefined);
 assert.equal(skatingDetail({...p,group:'Goalies',skating_detail:{...r,group:'Goalies'}}),undefined);assert.equal(skatingDetail(undefined),undefined);
});

test('recorded shot methods retain scouted techniques and reject stale metadata',async()=>{
 const {shotMethods,renderShotMethods}=await import('../dist/src/rinkcraft-shot-methods.js');
 const m={id:'wrist',name:'Wrist shot',result:'Attempts 5 · Goals 1',share:'71.4% of typed attempts',outcomes:'On goal 4 · Misses 1'};
 const r={schema:'rinkcraft.shot-methods.v1',player_id:1,season:20262027,group:'Forwards',games:20,typed_attempts:7,total_attempts:10,coverage:'7/10 attempts typed · 7 cached roster games',primary:[m],additional:[],unknown_display:'3 attempts without a supported type',captured_at:'2026-11-24T00:00:00Z',note:'Observed types, not mastery.'};
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',games:20,minutes:400,scope:'all strengths',small_sample:false,shot_methods:r,scores:['Skating','Handling','Passing','Shooting','Reading','Defending'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'}))};
 assert.equal(shotMethods(stats),r);for(const stale of [{...r,player_id:2},{...r,season:20252026},{...r,games:19},{...r,group:'Goalies'}])assert.equal(shotMethods({...stats,shot_methods:stale}),undefined);
 const args=Array(18).fill(undefined);args[0]='Test';args[1]=sheet;args[3]={season:20262027,games:20,position:'Center',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];const front=renderCharacterCard(...args);assert.match(classed(front,'rk-front-methods')[0].textContent,/Recorded shot repertoire.*Wrist shot.*Attempts 5/);assert.match(classed(front,'rk-scouted-methods')[0].textContent,/Scouted methods.*Recover/);
 assert.match(renderShotMethods({...r,primary:[]}).textContent,/No supported shot types recorded/);assert.equal(shotMethods(undefined),undefined);
});

test('selected snapshot pace fills contact and blocks while rejecting another season',()=>{
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',games:20,minutes:400,scope:'all strengths',small_sample:false,
  scores:['Skating','Handling','Passing','Shooting','Reading','Defending'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'})),
  estimated_record:{observed_games:20,season:20262027,target_games:84,basis:'84-game pace / all strengths',metrics:[{recorded_count:10,calculation:'10 / 20 GP x 84 games = 42.00',key:'hits',label:'Hits',value:42,unit:'Count',per_game:0.5,projected_total:true},{recorded_count:0,calculation:'0 / 20 GP x 84 games = 0.00',key:'blocks',label:'Blocks',value:0,unit:'Count',per_game:0,projected_total:true}]}};
 const args=Array(18).fill(undefined);args[0]='Skater';args[3]={season:20262027,games:20,position:'Center',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];
 const block=classed(renderCharacterCard(...args),'rk-front-record')[0];assert.match(block.textContent,/84-game pace.*Hits 42 0.50 \/game.*Blocks 0 0.00 \/game/);assert.doesNotMatch(block.textContent,/Unknown|\bP\d|\bn=/);
 assert.match(classed(block,'rk-pace-detail')[0].textContent,/20 observed GP.*10 recorded.*42.00.*0 recorded/);
 stats.estimated_record.season=20252026;assert.doesNotMatch(classed(renderCharacterCard(...args),'rk-front-record')[0].textContent,/84-game pace/);
});

test('task repertoire uses Rust order and names while preserving the complete feat library',()=>{
 const feat=(id,state='earned')=>({id,name:id,state,tier:'Silver',placements:['task_skills'],matched:1,required:1,conditions:[{key:id,label:'Shots / GP',value:2,unit:'PerGame',numerator:40,denominator:20,percentile:80,peers:100,reference_sha256:'ref',qualifies:true}]});
 const p={player_id:1,season:20262027,season_type:'regular',group:'Forwards',games:20,minimum_games:10,rules_sha256:'rules',source_sha256:'input',source_urls:[],earned:2,notes:[],feats:[feat('a'),feat('b'),feat('locked','not_earned')]};
 p.task_repertoire={schema:'rinkcraft.task-repertoire.v1',player_id:1,season:20262027,season_type:'regular',group:'Forwards',games:20,skills:[{feat_id:'b',name:'Draw penalties',family:'drawing'}],note:'Selection only'};
 const r=renderStatFeats(p,'task_skills',5);assert.equal(classed(r,'rk-task-feat').length,1);assert.match(classed(r,'rk-task-feat')[0].textContent,/Draw penalties.*Silver.*Shots 2.00\/game/);assert.equal(classed(r,'rk-stat-feat').length,3);assert.match(r.textContent,/1 selected \/ 2 earned.*2026-27.*20 GP/);
 for(const stale of [{...p.task_repertoire,season:20252026},{...p.task_repertoire,games:19},{...p.task_repertoire,skills:[{feat_id:'locked',name:'Invalid',family:'drawing'}]}])assert.equal(classed(renderStatFeats({...p,task_repertoire:stale},'task_skills',5),'rk-task-repertoire').length,0);
 assert.equal(classed(renderStatFeats(p,'playing_tendencies',2),'rk-task-repertoire').length,0);
});

class Element {
  children = []; attributes = {}; dataset = {}; style = {};
  constructor(tag) { this.tag = tag; }
  append(...nodes) { this.children.push(...nodes); }
  setAttribute(key, value) { this.attributes[key] = value; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? '') + this.children.map(n => n.textContent).join(' '); }
}
globalThis.document = {
  createElement: tag => new Element(tag),
  createTextNode: text => Object.assign(new Element('#text'), {text}),
};
const all = n => [n, ...n.children.flatMap(all)];
const classed = (n, name) => all(n).filter(e => e.className?.split(' ').includes(name));

test('skater attribute records retain measured zeroes and filter per-attribute detail',()=>{
 const context={source_credit:'NHL / MoneyPuck.com',notes:['Shot quality and credited assists are outcome context.'],values:{},rows:[
 {attribute:'Passing',label:'Primary assists',value:'13'},{attribute:'Passing',label:'Secondary assists',value:'7'},
 {attribute:'Shooting',label:'Goals minus expected (goalie in net)',value:'+1.20'},{attribute:'Defending',label:'Penalty-kill deployment',value:'0:00 /GP'}]};
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',games:20,minutes:400,scope:'all strengths',small_sample:false,
 scores:['Skating','Handling','Passing','Shooting','Reading','Defending'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'})),skater_context:context};
 const args=Array(18).fill(undefined);args[0]='Skater';args[3]={season:20262027,games:20,position:'Center',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];
 const block=classed(renderCharacterCard(...args),'rk-skater-context')[0];assert.match(block.textContent,/Primary assists 13.*Secondary assists 7.*0:00 \/GP/);
 const passing=renderStatScore(stats,stats.scores[2]);assert.match(passing.textContent,/Passing record.*Primary assists 13/);
 args[17]=[{...stats,season:20252026}];assert.equal(classed(renderCharacterCard(...args),'rk-skater-context').length,0);
});

test('goalie shot contexts show Rust counts and keep attribute details separate',()=>{
 const context={source_url:'https://peter-tanner.com/moneypuck/downloads/shots_2026.zip',captured_at:'2026-11-24T00:17:34Z',sha256:'a'.repeat(64),
  headline:[{attribute:'Movement',label:'Cross-slot rebound shots on goal',value:'1/2 saved · 50.0%'}],
  rows:[{attribute:'Movement',label:'Cross-slot rebound shots on goal',value:'1/2 saved · 50.0%'},{attribute:'Recovery',label:'Follow-ups after missed shots',value:'No on-goal opportunities'}]};
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Goalies',games:20,minutes:1000,scope:'all strengths',small_sample:false,
  scores:['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play'].map(label=>({label,score:10,stat_points:50,parts:[],note:'Proxy'})),shot_context:context};
 const args=Array(18).fill(undefined);args[0]='Goalie';args[3]={season:20262027,games:20,position:'Goalie',metrics:[]};args[11]={player_id:1,season_type:'regular'};args[17]=[stats];
 const block=classed(renderCharacterCard(...args),'rk-shot-context')[0];assert.match(block.textContent,/Shot Record.*MoneyPuck.com.*1\/2 saved/);
 args[4]={player_id:1,season:20262027,group:'Goalies',situation:'all strengths',games:20,minutes:1000,eligible_peers:20,facets:[],metrics:[],leading:[],unknowns:[],source_credit:'NHL',stat_attributes:{...stats,shot_context:undefined}};
 assert.equal(classed(renderCharacterCard(...args),'rk-shot-context').length,1,'New attached catalog supplies detail ahead of a legacy embedded estimate.');
 args[4]=undefined;
 const popup=renderStatScore(stats,stats.scores[2]);assert.match(popup.textContent,/Observed shot contexts.*Cross-slot/);
 assert.match(popup.textContent,/not measured goalie travel or reset time/);
 assert.equal(classed(renderStatScore(stats,stats.scores[0]),'rk-shot-context').length,0);
 args[17]=[{...stats,season:20252026}];assert.equal(classed(renderCharacterCard(...args),'rk-shot-context').length,0);
});

test('goalie puck record preserves counts and links without leaking across card scopes',()=>{
 const puck={player_id:1,season:20262027,season_type:'regular',coverage:'377 captured games · 32 selected clubs',captured_through:'2026-11-24T00:36:35Z',
 metrics:[{label:'Goals',count:0},{label:'Giveaways',count:5}],penalty_types:[{label:'tripping',count:1}],
 plays:[{label:'Giveaway',game_id:2026020001,event_id:3,period:1,clock:'01:00',game_url:'https://www.nhl.com/gamecenter/2026020001',source_url:'https://api-web.nhle.com/v1/gamecenter/2026020001/play-by-play',following_attempt:'Opponent shot on goal 5s later; timing, not causation.'}],note:'Recorded counts, not projected totals or handling success rates.'};
 const render=(p=puck,position='Goalie',type='regular')=>renderCharacterCard('Example',undefined,undefined,{season:20262027,games:5,position,metrics:[]},undefined,undefined,undefined,undefined,true,undefined,undefined,{player_id:1,season_type:type},undefined,undefined,undefined,undefined,undefined,undefined,p);
 const block=classed(render(),'rk-puck-record')[0];assert.ok(block);assert.match(block.textContent,/Puck Record.*Goals 0.*Giveaways 5.*tripping: 1/);
 assert.match(block.textContent,/timing, not causation/);assert.equal(all(block).find(n=>n.tag==='a').href,puck.plays[0].game_url);
 for(const p of [undefined,{...puck,player_id:2},{...puck,season:20252026},{...puck,season_type:'playoff'}]){
  const args=p===undefined?null:p;
  assert.equal(classed(render(args),'rk-puck-record').length,0);
 }
 assert.equal(classed(render(puck,'Center'),'rk-puck-record').length,0);
 assert.equal(classed(render(puck,'Goalie','playoff'),'rk-puck-record').length,0);
});
test('estimated record shows season totals with per-game rates rather than percentile codes',()=>{
 const numerical={player_id:1,season:20262027,group:'Forwards',situation:'all strengths',games:20,minutes:300,eligible_peers:200,facets:[],metrics:[],leading:[],source_credit:'NHL',unknowns:[],estimated_record:{season:20262027,target_games:84,basis:'84-game pace',metrics:[{key:'points',label:'Points',value:168,unit:'Count',per_game:2,projected_total:true},{key:'missing',label:'Missing',value:null,unit:'Count',per_game:null,projected_total:true}]}};
 const record=classed(renderCharacterCard('Example',undefined,undefined,undefined,numerical),'rk-front-record')[0];
 assert.match(record.textContent,/Estimated Record.*84-game pace.*Points 168 2.00 \/game/);
 assert.match(record.textContent,/Missing — Unavailable/);
 assert.doesNotMatch(record.textContent,/\bP\d|\bn=/);
});
test('simple stat estimates occupy the six primary attributes and require matching identity/window',()=>{
 const labels=['Skating','Handling','Passing','Shooting','Reading','Defending'];
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20252026,group:'Forwards',scope:'all strengths',small_sample:false,scores:labels.map((label,i)=>({label,score:i+10,stat_points:(i+10)*5,parts:[],note:'Stat approximation'}))};
 const numerical={player_id:1,season:20252026,group:'Forwards',situation:'all strengths',games:30,minutes:300,eligible_peers:200,facets:[],metrics:[],leading:[],source_credit:'NHL',unknowns:[],stat_attributes:stats};
 const render=n=>renderCharacterCard('Example',undefined,undefined,{season:20252026,position:'Center',games:30,metrics:[]},n,undefined,undefined,undefined,true,undefined,undefined,{player_id:1,season_type:'regular'});
 const ratings=all(render(numerical)).filter(n=>n.dataset.capabilityIndex!==undefined);
 assert.equal(ratings.length,6);assert.ok(ratings.every(r=>r.dataset.ratingState==='stat_estimate'));
 assert.deepEqual(ratings.map(r=>r.children[1].textContent),['10','11','12','13','14','15']);
 for(const invalid of [{...stats,player_id:2},{...stats,season:20242025},{...stats,group:'Defense'}])assert.ok(all(render({...numerical,stat_attributes:invalid})).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
 const small=render({...numerical,stat_attributes:{...stats,small_sample:true}});assert.match(classed(small,'rk-front-attributes')[0].textContent,/Small sample/);
});
const sheet = {
  primary:{id:'warden',name:'Warden',analogy:'Paladin',purpose:'Protect the final space.'},
  secondary:[], specialty:'Edge keeper', summary:'Full documentary summary.',
  features:[{name:'Recover',detail:'Detailed recovery account.',evidence_type:'attributed method',source_ids:['SRC-1']}],
  traits:[], sources:[{id:'SRC-1',title:'Evidence account',url:'https://example.com',published:'2025-01-01',locator:'Page 1'}],
  unknowns:['Technique is not measured.'], assessed:'2026-11-09',evidence_through:'2025-01-01',
  protocol:'experimental',catalog_sha256:'a'.repeat(64),as_of_season:20252026,
};

test('stat feats populate five slots, retain the long list and gate actor/window/group',()=>{
 const f=(id,placement,state='earned')=>({id,name:id,state,tier:state==='earned'?'Gold':null,placements:[placement],matched:state==='earned'?2:1,required:2,conditions:[{key:'hits',label:'Hits / GP',value:2,unit:'PerGame',numerator:100,denominator:50,percentile:95,peers:100,reference_sha256:'ref',qualifies:true}]});
 const a={player_id:1,season:20242025,season_type:'regular',group:'Forwards',games:50,minimum_games:20,rules_sha256:'rules',source_sha256:'input',source_urls:[],earned:4,notes:['Experimental stat patterns, not abilities.'],feats:[f('Hitter','task_skills'),f('Locked','task_skills','not_earned'),f('Volume','playing_tendencies'),f('Triple','distinctive_features'),f('Flag','risk_flags')]};
 const render=feat=>renderCharacterCard('Example',{...sheet,player_id:1},undefined,{season:20242025,position:'Center',games:50,metrics:[]},undefined,undefined,undefined,undefined,true,undefined,undefined,{player_id:1,season_type:'regular'},feat);
 const card=render(a),front=classed(card,'rk-front')[0];assert.equal(classed(front,'rk-stat-feats').length,5);assert.match(classed(front,'rk-front-skills')[0].textContent,/Hitter.*Hits: 2.00\/game.*Locked/);assert.match(classed(front,'rk-front-limitations')[0].textContent,/Flag/);assert.equal(classed(front,'rk-front-pressure')[0].textContent.includes('Hitter'),false);
 assert.ok(all(front).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
 for(const wrong of [{...a,player_id:2},{...a,season:20252026},{...a,season_type:'playoff'},{...a,group:'Goalies'}])assert.equal(classed(render(wrong),'rk-stat-feats').length,0);
 const early={...a,games:9,minimum_games:10,earned:0,feats:[f('Early hitter','task_skills','ineligible')]};
 assert.match(classed(render(early),'rk-front-skills')[0].textContent,/Needs 10 GP \(9\/10\)/);
 const qualified={...early,games:18,earned:1,feats:[f('Hitter','task_skills')]};
 assert.doesNotMatch(classed(render(qualified),'rk-front-skills')[0].textContent,/Needs .*GP|No unlocked/);
});

test('workload requires matching actor, season, season type, GP and position group',()=>{
  const usage={player_id:1,season:20242025,season_type:'regular',games:2,goalie:false,starts:null,
    rows:[{key:'total',label:'All strengths',per_game_display:'20:00',minutes_display:'40.0',seconds:2400,share_display:'100.0%'},{key:'pp',label:'Power play',per_game_display:'3:00',minutes_display:'6.0',seconds:360,share_display:'15.0%'},{key:'sh',label:'Shorthanded',per_game_display:'0:00',minutes_display:'0.0',seconds:0,share_display:'0.0%'}],source_label:'NHL source',source_url:'https://api.nhle.com/stats/rest/en/skater/timeonice',source_sha256:'a'.repeat(64),notes:['Even strength is not exclusively 5v5.']};
  const render=(u,request={player_id:1,season_type:'regular'})=>renderCharacterCard('Example',{...sheet,player_id:1},undefined,{season:20242025,position:'Center',games:2,metrics:[]},undefined,undefined,undefined,undefined,true,undefined,u,request);
  const p=render(usage),front=classed(p,'rk-front')[0];
  assert.match(classed(front,'rk-front-role-coverage')[0].textContent,/PP 3:00 \/ SH 0:00 per GP/);
  assert.match(classed(front,'rk-front-deployment')[0].textContent,/Assigned unit \/ station undocumented/);
  assert.match(classed(front,'rk-front-deployment')[0].textContent,/0 seconds \/ 2 appearances/);
  assert.equal(classed(front,'rk-usage')[0].dataset.usageSeason,'20242025');
  for(const u of [{...usage,games:1},{...usage,season:20252026},{...usage,player_id:2},{...usage,season_type:'playoff'},{...usage,goalie:true}])assert.equal(classed(render(u),'rk-usage').length,0);
  assert.equal(classed(render(usage,{season_type:'playoff'}),'rk-usage').length,0);
  assert.ok(all(front).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
});

test('factual background is explicit, source-linked and separate from capability ratings',()=>{
  const dossier={...sheet,card_facts:[{...sheet.features[0],field:'development',name:'Example college (NCAA)',context:'Selected pre-NHL pathway'},{...sheet.features[0],field:'experience',name:'NHL debut 2019',context:'2019-04-15 · postseason'}]};
  const front=classed(renderCharacterCard('Example',dossier),'rk-front')[0];
  const facts=all(front).filter(n=>n.dataset.factField);
  assert.equal(facts.length,2);
  assert.match(facts[0].textContent,/Example college.*Selected pre-NHL pathway.*Evidence account/);
  assert.match(facts[1].textContent,/NHL debut 2019.*postseason/);
  assert.equal(all(facts[0]).find(n=>n.tag==='a').href,'https://example.com');
  assert.ok(all(front).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
  const missing=all(renderCharacterCard('Legacy',sheet)).filter(n=>n.dataset.factField);
  assert.ok(missing.every(n=>n.textContent.includes('Undocumented')));
});

test('compact identity keeps documentary detail and provenance in a closed disclosure', () => {
  const profile=renderCharacterCard('<img src=x>',sheet);
  const front=classed(profile,'rk-front')[0], evidence=classed(profile,'rk-evidence')[0];
  assert.match(front.textContent,/Edge keeper/);
  assert.match(front.textContent,/Recover/);
  assert.match(front.textContent,/Detailed recovery account/);
  assert.match(front.textContent,/attributed method/);
  assert.match(evidence.textContent,/Detailed recovery account/);
  assert.match(evidence.textContent,/Technique is not measured/);
  assert.match(evidence.textContent,/a{64}/);
  assert.equal(evidence.attributes.open,undefined);
  assert.equal(all(profile).some(n=>n.tag==='img'),false);
});

test('front-sheet signals retain zero, missing values and unranked measurements', () => {
  const metric=(key,value,percentile)=>({key,label:key,unit:'Count',value,percentile,measured_peers:12,interpretation:'Recorded only',state:percentile===null?'unranked':'ranked'});
  const metrics=[metric('Zero',0,0),metric('Missing',null,null)];
  const numerical={season:20252026,group:'Goalies',situation:'all strengths',games:20,minutes:700,eligible_peers:12,facets:metrics.map(measurement=>({label:measurement.label,measurement})),metrics,leading:[],unknowns:[],source_credit:'Source'};
  const profile=renderCharacterCard('Test goalie',sheet,undefined,undefined,numerical);
  const front=classed(profile,'rk-front')[0];
 assert.match(front.textContent,/Zero 0 Recorded value/);
  assert.match(front.textContent,/Missing.*—.*Unavailable/);
  assert.equal(all(front).filter(n=>n.dataset.numericalKey).length,2);
  assert.match(classed(front,'rk-front-attributes')[0].textContent,/Tracking.*Positioning.*Movement.*Rebounds.*Recovery.*Puck Play/);
});

test('missing dossiers leave source and question coverage unknown', () => {
  const profile=renderCharacterCard('Unscouted player',undefined);
  const front=classed(profile,'rk-front')[0];
  assert.match(front.textContent,/Unscouted/);
  assert.match(front.textContent,/Sources Unknown/);
  assert.match(front.textContent,/Open Questions Unknown/);
  assert.match(front.textContent,/statistics do not assign a class/);
});


test('capabilities without estimates stay unrated while record statistics remain separate', () => {
  const profile=renderCharacterCard('Test skater',sheet);
  const rail=classed(profile,'rk-front-attributes')[0];
  const scores=all(rail).filter(n=>n.dataset.capabilityIndex!==undefined);
  assert.equal(scores.length,6);
  assert.deepEqual(scores.map(s=>s.children[0].textContent),['Skating','Handling','Passing','Shooting','Reading','Defending']);
  assert.ok(scores.every(s=>s.children[1].textContent==='—'&&s.dataset.ratingState==='unassessed'));
  assert.equal(classed(profile,'rk-front-role')[0].textContent.includes(sheet.primary.purpose),false);
  assert.equal(classed(profile,'rk-front-limitations')[0].textContent.includes(sheet.unknowns[0]),false);
  assert.match(classed(profile,'rk-front-questions')[0].textContent,/Technique is not measured/);
});

test('goalie capabilities remain separate and uncurated repertoires stay unknown', () => {
  const profile=renderCharacterCard('Test goalie',sheet,undefined,{position:'Goalie',games:20,metrics:[]});
  const rail=classed(profile,'rk-front-attributes')[0];
  assert.deepEqual(all(rail).filter(n=>n.dataset.capabilityIndex!==undefined).map(s=>s.children[0].textContent),['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play']);
  assert.match(classed(profile,'rk-front-skills')[0].textContent,/Repertoire not yet catalogued/);
  assert.doesNotMatch(classed(profile,'rk-front-skills')[0].textContent,/Retrieval|Rebound Direction|Unassessed/);
  assert.doesNotMatch(rail.textContent,/Shooting|Defending/);
});

test('pressure and task repertoires use only their explicit curated entries and never scores or methods',()=>{
  const f=name=>({...sheet.features[0],name,detail:'When the lane closes, use the supported option.'});
  const dossier={...sheet,under_pressure:['Draw & slip','Reverse escape','Hold & scan','Early rim'].map(f),task_skills:['Seam feeds','PP distribution','Entry denial','Rim retrieval','Outlet reception','Point setup'].map(f)};
  const front=classed(renderCharacterCard('Test',dossier),'rk-front')[0];
  const pressure=classed(front,'rk-front-pressure')[0],skills=classed(front,'rk-front-skills')[0];
  assert.match(pressure.textContent,/Draw & slip/);assert.doesNotMatch(pressure.textContent,/Seam feeds|Recover|\/ 20|Unassessed/);
  assert.match(skills.textContent,/Seam feeds/);assert.doesNotMatch(skills.textContent,/Draw & slip|Recover|\/ 20|Unassessed/);
  assert.equal(classed(pressure,'rk-repertoire-list')[0].children.length,3);
  assert.equal(classed(skills,'rk-repertoire-list')[0].children.length,5);
  assert.match(classed(pressure,'rk-repertoire-more')[0].textContent,/\+1 more/);
  assert.match(classed(skills,'rk-repertoire-more')[0].textContent,/\+1 more/);
  assert.equal(classed(pressure,'rk-repertoire-more')[0].attributes.open,undefined);
  assert.match(pressure.textContent,/Evidence account/);
  assert.match(classed(front,'rk-front-methods')[0].textContent,/Recover/);
});


test('unavailable catalog is distinct from an unscouted player on the front', () => {
  const profile=renderCharacterCard('Test',undefined,undefined,undefined,undefined,undefined,undefined,undefined,false);
  const front=classed(profile,'rk-front')[0];
  assert.match(front.textContent,/Class dossier unavailable/);
  assert.doesNotMatch(front.textContent,/Unscouted|No supported class dossier/);
});

test('method evidence joins only declared context and keeps missing technique evidence visible', () => {
  const measurement={key:'primary-assists-per-60',label:'Primary assists /60',unit:'Per60',value:0,percentile:0,measured_peers:20,formula:'assists / seconds * 3600',numerator:0,denominator:3600,scale:3600};
  const numerical={season:20242025,group:'Forwards',situation:'5on5',games:20,minutes:60,eligible_peers:20,leading:[],unknowns:[],source_credit:'Synthetic',facets:[],metrics:[measurement],feature_context:[{attribute_key:'assist-output',relation:'Outcome context',note:'Assist output does not measure containment.',measurements:[measurement]}]};
  const dossier={...sheet,features:[
    {...sheet.features[0],name:'Contain',stat_evidence:[{attribute_key:'assist-output',relation:'outcome_context'}],unmeasured_stats:['Coverage opportunities']},
    {...sheet.features[0],name:'Carry',stat_evidence:[{attribute_key:'unknown-transport',relation:'measured_aspect'}],unmeasured_stats:['Controlled entry opportunities']},
  ]};
  const front=classed(renderCharacterCard('Test',dossier,undefined,undefined,numerical),'rk-front')[0];
  const evidence=classed(front,'rk-method-evidence');
  assert.equal(evidence.length,2);
  assert.match(evidence[0].textContent,/Outcome context.*0\.00 \/60.*P0\.0/);
  assert.match(evidence[0].textContent,/does not measure containment/);
  assert.match(evidence[0].textContent,/Need: Coverage opportunities/);
  assert.match(evidence[1].textContent,/No matched statistic/);
  assert.doesNotMatch(evidence[1].textContent,/Primary assists/);
  assert.ok(evidence.every(e=>e.attributes.open===undefined));
  assert.equal(all(front).filter(e=>e.dataset.measurementKey).length,1);
  assert.ok(all(front).filter(e=>e.dataset.capabilityIndex!==undefined).every(e=>e.children[1].textContent==='—'));
});

test('observed season maximum is movement detail, never a capability or mixed-season fallback', () => {
  const speed={key:'max-skating-speed',label:'Maximum speed',value:23.969,unit:'Mph',numerator:23.969,denominator:1,scope:'All strengths · 67 GP',meaning:'Observed maximum',sources:[]};
  const numerical={season:20242025,group:'Forwards',situation:'5on5',games:67,minutes:1000,eligible_peers:20,leading:[],unknowns:[],source_credit:'Synthetic',facets:[],metrics:[],style_evidence:{season:20242025,measured:[speed],gaps:[],limitations:[]}};
  const render=n=>classed(renderCharacterCard('Test',sheet,undefined,undefined,n),'rk-front')[0];
  const front=render(numerical),detail=classed(front,'rk-front-movement')[0];
  assert.match(detail.textContent,/23\.97 mph.*Season max · all strengths/);
  assert.equal(detail.dataset.movementState,'observed_maximum');
  assert.match(detail.title,/not puck-carrying speed/);
  assert.ok(all(front).filter(e=>e.dataset.capabilityIndex!==undefined).every(e=>e.children[1].textContent==='—'));
  for(const n of [
    {...numerical,group:'Goalies'},
    {...numerical,style_evidence:{...numerical.style_evidence,season:20232024}},
    {...numerical,style_evidence:{...numerical.style_evidence,measured:[{...speed,unit:'Count'}]}},
    {...numerical,style_evidence:{...numerical.style_evidence,measured:[{...speed,scope:'Unknown'}]}},
  ])assert.equal(classed(render(n),'rk-front-movement')[0].dataset.movementState,'unassessed');
  const zero=render({...numerical,style_evidence:{...numerical.style_evidence,measured:[{...speed,value:0,numerator:0}]}});
  assert.match(classed(zero,'rk-front-movement')[0].textContent,/0\.00 mph/);
});

test('experimental finishing stays in the dossier, with history scope and no capability promotion', () => {
  const finishing={status:'experimental',note:'Earlier history only.',input_sha256:'b'.repeat(64),model_id:'Neutral model',model_config_sha256:'c'.repeat(64),target_season:20242025,calibration_season:20232024,history_seasons:[20232024],history_goals:0,history_xg:1,mean:0,lower:0,upper:1,probability:.9,sources:[],limits:['Not a complete Shooting grade.']};
  const numerical={season:20242025,group:'Forwards',situation:'5on5',games:20,minutes:60,eligible_peers:20,leading:[],unknowns:[],source_credit:'Synthetic',facets:[],metrics:[],finishing_research:finishing};
  const p=renderCharacterCard('Test',sheet,undefined,undefined,numerical),front=classed(p,'rk-front')[0],dossier=classed(p,'rk-evidence')[0];
  assert.equal(classed(front,'rk-finishing').length,0);
  const section=classed(dossier,'rk-finishing')[0];
  assert.match(section.textContent,/0\.000.*90% conditional interval.*0\.000 – 1\.000/);
  assert.match(section.textContent,/Player history: 2023–24 · 0 goals/);
  assert.match(section.textContent,/Not a complete Shooting grade/);
  assert.ok(all(front).filter(e=>e.dataset.capabilityIndex!==undefined).every(e=>e.children[1].textContent==='—'));
  const unavailable=renderCharacterCard('Test',undefined,undefined,undefined,{...numerical,finishing_research:{...finishing,status:'unavailable',note:'No exact source match.',mean:null,lower:null,upper:null}});
  const missing=classed(unavailable,'rk-finishing')[0];assert.match(missing.textContent,/No exact source match/);assert.equal(classed(missing,'rk-finishing-values').length,0);
});

test('finishing ledger displays shared support including zero and withholds unavailable ledgers', async () => {
  const {renderFinishing}=await import('../dist/src/rinkcraft-finishing.js');
  const f={status:'experimental',note:'Earlier only',input_sha256:'a',model_id:'',model_config_sha256:'',target_season:20252026,calibration_season:20232024,history_seasons:[20242025],history_goals:0,history_xg:1,mean:.8,lower:.2,upper:2,probability:.9,sources:[],limits:[],support:{rows:[{season:20242025,position:'D',goals:0,provider_xg:1,position_factor:1,adjusted_xg:1}],prior_shape:4,prior_rate:4,posterior_shape:4,posterior_rate:5,adjusted_history_xg:1,history_weight:.2}};
  const ledger=classed(renderFinishing(f),'rk-finishing-support')[0];
  assert.match(ledger.textContent,/20\.0%.*not confidence, reliability or a capability score/);
  assert.match(ledger.textContent,/2024–25 D 0 1\.00 1\.000 1\.00/);
  assert.match(ledger.textContent,/shape 4\.000, rate 5\.000/);
  assert.equal(ledger.attributes.open,undefined);
  for(const variant of [{...f,support:undefined},{...f,status:'unavailable'},{...f,status:'not_applicable'}])assert.equal(classed(renderFinishing(variant),'rk-finishing-support').length,0);
  const empty=classed(renderFinishing({...f,status:'baseline_only',support:{...f.support,rows:[],history_weight:0,adjusted_history_xg:0}}),'rk-finishing-support')[0];
  assert.match(empty.textContent,/0\.0%/);assert.match(empty.textContent,/No earlier player rows/);
});

test('snapshot differences preserve zero, signed percentage points and unavailable states', async () => {
 const {renderSeasonChanges}=await import('../dist/src/rinkcraft-history.js');
 const row={key:'test',label:'Test',unit:'Pct',state:'descriptive',from_value:.1,to_value:0,difference:-.1,percentile_point_difference:20,note:'Descriptive only'};
 const render=r=>renderSeasonChanges([{from_season:20242025,to_season:20252026,from_games:20,to_games:30,from_minutes:100,to_minutes:200,rows:[r]}]);
 const view=render(row);assert.match(view.textContent,/20 → 30 GP.*100\.0 → 200\.0 minutes/);
 assert.match(view.textContent,/10\.00% 0\.00% -10\.00 pp \+20\.0 percentile points/);
 assert.match(view.textContent,/do not establish ability changes/);assert.equal(view.attributes.open,undefined);
 const missing=render({...row,state:'unavailable',from_value:null,to_value:null,difference:null,percentile_point_difference:null});
 assert.match(missing.textContent,/— — — Unavailable/);assert.doesNotMatch(missing.textContent,/0\.00%/);
});

test('evidence brief preserves Rust-selected order, zero, catalogue gaps and exact history matching', async () => {
 const {renderEvidenceBrief}=await import('../dist/src/rinkcraft-brief.js');
 const n={player_id:1,season:20242025,group:'Defense',situation:'5on5',games:20,minutes:400,leading:['b','a'],metrics:[{key:'a',label:'A',unit:'Per60',value:1,percentile:99,measured_peers:20},{key:'b',label:'B',unit:'Per60',value:0,percentile:0,measured_peers:20}]};
 const h={player_id:1,seasons:[{season:20242025}],assessment:{first_season:20232024,last_season:20242025,reported_seasons:2,supplied_seasons:2,repeated:[],repeated_reason:'No qualifying repeated signal.'}};
 const dossier={...sheet,features:[{...sheet.features[0],unmeasured_stats:['Passing attempts','Passing attempts','Release timing']}]};
 const b=renderEvidenceBrief(n,h,dossier);assert.deepEqual(all(b).filter(e=>e.dataset.briefKey).map(e=>e.dataset.briefKey),['b','a']);
 assert.match(b.textContent,/0\.00 \/60 · P0\.0/);assert.equal(b.textContent.match(/Passing attempts/g).length,1);
 assert.equal(all(b).find(e=>e.dataset.briefHistory).dataset.briefHistory,'matched');
 for(const history of [undefined,null,{...h,player_id:2},{...h,seasons:[{season:20252026}]},{...h,assessment:{...h.assessment,last_season:20252026}}]){
  const view=renderEvidenceBrief(n,history,dossier);assert.match(view.textContent,/No history summary matched/);assert.doesNotMatch(view.textContent,/No qualifying repeated signal/);
 }
 const absent=renderEvidenceBrief({...n,leading:[]});assert.match(absent.textContent,/No selected measured signals/);assert.match(absent.textContent,/No task-specific gap list/);
});
test('current NHL panels use their own source and peer rules rather than historical provider defaults',async()=>{
 const {renderNumericalPanel}=await import('../dist/src/rinkcraft-numbers.js');
 const panel=renderNumericalPanel({season:20262027,situation:'all strengths',games:2,minutes:30,eligible_peers:100,source_label:'NHL public reports',source_credit:'Captured public NHL rows',peer_rules:'20 skater GP and 20 measured peers',leading:[],facets:[],metrics:[],sources:[]});
 assert.match(panel.textContent,/NHL public reports/);assert.match(panel.textContent,/20 skater GP/);assert.doesNotMatch(panel.textContent,/MoneyPuck|Skaters: 5-on-5|Source CSV/);
});
test('career audit preserves source conflict without manufacturing a saving value',async()=>{
 const {renderCareerAudit}=await import('../dist/src/rinkcraft-career.js');
 const view=renderCareerAudit({same_provider:true,seasons:[{season:20232024,games:55,seconds:1,teams:'AAA',state:'reconciled'}],fields:[{key:'saves',state:'conflicting_sources',rest_seasons:8840,derived_from_published:8839,conflict_seasons:[{season:20232024,reported_saves:1466,shots_against:1606,goals_against:141,published_save_fraction:.9122}]}]});
 assert.match(view.textContent,/8840.*8839/);assert.match(view.textContent,/1466.*1606.*141/);assert.match(view.textContent,/same official provider/);assert.match(view.textContent,/No resolution inferred/);
 const {numericalValue}=await import('../dist/src/rinkcraft-numbers.js');assert.equal(numericalValue({value:0,unit:'Count'}),'0');assert.equal(numericalValue({value:0,unit:'PerGame'}),'0.00');
});

test('ordinary profile estimates render without a numerical dossier and remain GP/window/position scoped',()=>{
 const labels=['Skating','Handling','Passing','Shooting','Reading','Defending'];
 const stats={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Forwards',scope:'all strengths',games:30,minutes:300,small_sample:false,scores:labels.map(label=>({label,score:10,stat_points:50,parts:[],note:'Stat estimate'}))};
 const render=(scores,type='regular',position='Center')=>renderCharacterCard('Example',undefined,undefined,{season:20262027,position,games:30,metrics:[]},undefined,undefined,undefined,undefined,true,undefined,undefined,{player_id:1,season_type:type},undefined,undefined,undefined,undefined,undefined,scores);
 const fields=all(render([stats])).filter(n=>n.dataset.capabilityIndex!==undefined);
 assert.equal(fields.length,6);assert.ok(fields.every(n=>n.dataset.ratingState==='stat_estimate'));
 assert.deepEqual(fields.map(n=>n.children[1].textContent),['10','10','10','10','10','10']);
 for(const wrong of [{...stats,games:29},{...stats,season:20252026},{...stats,player_id:2},{...stats,group:'Defense'},{...stats,games:undefined}])assert.ok(all(render([wrong])).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
 for(const [type,position] of [['playoff','Center'],['regular','Goalie']])assert.ok(all(render([stats],type,position)).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
});

test('six-attribute history preserves season order, unknown scores, sample labels and source identity',async()=>{
 const {renderStatHistory}=await import('../dist/src/rinkcraft-stat-scores.js');
 const labels=['Skating','Handling','Passing','Shooting','Reading','Defending'];
 const make=(season,games,score)=>({schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season,group:'Forwards',scope:'all strengths',games,minutes:games*10,small_sample:games<10,scores:labels.map(label=>({label,score,stat_points:score===null?null:score*5,parts:[],note:'Stat estimate'}))});
 const old=make(20242025,40,10),recent=make(20252026,4,null);
 const input=[old,recent,{...old,player_id:2},{...old,group:'Goalies'},{...old,games:undefined}];
 const view=renderStatHistory(input,1,'regular','Example');
 const rows=all(view).filter(n=>n.dataset.attributeSeason!==undefined);
 assert.deepEqual(rows.map(n=>n.dataset.attributeSeason),['20252026','20242025']);
 assert.match(rows[0].textContent,/Small sample/);
 assert.deepEqual(all(rows[0]).filter(n=>n.tag==='button').map(n=>n.textContent),Array(6).fill('—'));
 assert.deepEqual(all(rows[1]).filter(n=>n.tag==='button').map(n=>n.textContent),Array(6).fill('10'));
 assert.equal(input[0],old);
 assert.match(renderStatHistory(input,1,'playoff','Example').textContent,/No season-matched/);
});

test('history changes display supplied Rust deltas and never fabricate comparisons',async()=>{
 const {renderStatHistory}=await import('../dist/src/rinkcraft-stat-scores.js');
 const labels=['Skating','Handling','Passing','Shooting','Reading','Defending'];
 const make=season=>({schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season,group:'Forwards',scope:'all strengths',games:40,small_sample:false,scores:labels.map(label=>({label,score:10,stat_points:50,parts:[],note:'Estimate'}))});
 const profiles=[make(20242025),make(20252026)];
 const change={player_id:1,season:20252026,previous_season:20242025,deltas:[0,4,null,-1,-1,-2]};
 const view=renderStatHistory(profiles,1,'regular','Example',[change]);
 assert.deepEqual(all(view).filter(n=>n.dataset.attributeDelta!==undefined).map(n=>n.textContent),['Δ 0','Δ +4','Δ —','Δ -1','Δ -1','Δ -2']);
 for(const invalid of [{...change,player_id:2},{...change,previous_season:20232024}])assert.equal(all(renderStatHistory(profiles,1,'regular','Example',[invalid])).filter(n=>n.dataset.attributeDelta!==undefined).length,0);
 assert.equal(all(renderStatHistory(profiles,1,'regular','Example')).filter(n=>n.dataset.attributeDelta!==undefined).length,0);
});

test('goalies receive their own six estimates and reject skater labels',async()=>{
 const labels=['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play'];
 const scores={schema:'rinkcraft.simple-stat-attributes.v1',player_id:1,season:20262027,group:'Goalies',scope:'all strengths',games:20,minutes:1000,small_sample:false,scores:labels.map(label=>({label,score:10,stat_points:50,parts:[],note:'Stat estimate'}))};
 const render=s=>renderCharacterCard('Goalie',undefined,undefined,{season:20262027,position:'Goalie',games:20,metrics:[]},undefined,undefined,undefined,undefined,true,undefined,undefined,{player_id:1,season_type:'regular'},undefined,undefined,undefined,undefined,undefined,[s]);
 assert.match(classed(render(scores),'rk-front-counters')[0].textContent,/MIN.*1000/);
 assert.match(classed(render(scores),'rk-front-footer')[0].textContent,/MoneyPuck.com/);
 const fields=all(render(scores)).filter(n=>n.dataset.capabilityIndex!==undefined);assert.equal(fields.length,6);assert.ok(fields.every(n=>n.dataset.ratingState==='stat_estimate'));assert.deepEqual(fields.map(n=>n.children[0].textContent),labels);
 assert.ok(all(render({...scores,group:'Forwards'})).filter(n=>n.dataset.capabilityIndex!==undefined).every(n=>n.dataset.ratingState==='unassessed'));
 const {renderStatHistory}=await import('../dist/src/rinkcraft-stat-scores.js');assert.match(renderStatHistory([scores],1,'regular','Goalie').textContent,/Tracking.*Positioning.*Movement.*Rebounds.*Recovery.*Puck Play/);
});
