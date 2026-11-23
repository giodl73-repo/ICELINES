import test from 'node:test';
import assert from 'node:assert/strict';
import {renderCharacterCard} from '../dist/src/rinkcraft-character.js';

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

test('workload requires matching actor, season, season type and position group',()=>{
  const usage={player_id:1,season:20242025,season_type:'regular',games:2,goalie:false,starts:null,
    rows:[{key:'total',label:'All strengths',per_game_display:'20:00',minutes_display:'40.0',seconds:2400,share_display:'100.0%'},{key:'pp',label:'Power play',per_game_display:'3:00',minutes_display:'6.0',seconds:360,share_display:'15.0%'},{key:'sh',label:'Shorthanded',per_game_display:'0:00',minutes_display:'0.0',seconds:0,share_display:'0.0%'}],source_label:'NHL source',source_url:'https://api.nhle.com/stats/rest/en/skater/timeonice',source_sha256:'a'.repeat(64),notes:['Even strength is not exclusively 5v5.']};
  const render=(u,request={player_id:1,season_type:'regular'})=>renderCharacterCard('Example',{...sheet,player_id:1},undefined,{season:20242025,position:'Center',games:2,metrics:[]},undefined,undefined,undefined,undefined,true,undefined,u,request);
  const p=render(usage),front=classed(p,'rk-front')[0];
  assert.match(classed(front,'rk-front-role-coverage')[0].textContent,/PP 3:00 \/ SH 0:00 per GP/);
  assert.match(classed(front,'rk-front-deployment')[0].textContent,/Unit \/ station undocumented/);
  assert.match(classed(front,'rk-front-deployment')[0].textContent,/0 seconds \/ 2 appearances/);
  assert.equal(classed(front,'rk-usage')[0].dataset.usageSeason,'20242025');
  for(const u of [{...usage,season:20252026},{...usage,player_id:2},{...usage,season_type:'playoff'},{...usage,goalie:true}])assert.equal(classed(render(u),'rk-usage').length,0);
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
