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
const sheet = {
  primary:{id:'warden',name:'Warden',analogy:'Paladin',purpose:'Protect the final space.'},
  secondary:[], specialty:'Edge keeper', summary:'Full documentary summary.',
  features:[{name:'Recover',detail:'Detailed recovery account.',evidence_type:'attributed method',source_ids:['SRC-1']}],
  traits:[], sources:[{id:'SRC-1',title:'Evidence account',url:'https://example.com',published:'2025-01-01',locator:'Page 1'}],
  unknowns:['Technique is not measured.'], assessed:'2026-11-09',evidence_through:'2025-01-01',
  protocol:'experimental',catalog_sha256:'a'.repeat(64),as_of_season:20252026,
};

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
  assert.match(front.textContent,/0\.00.*P0\.0/);
  assert.match(front.textContent,/Missing.*—.*Unranked/);
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


test('capabilities stay unrated while statistics remain in Measured Record', () => {
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

test('goalie capability and task fields use the goalie branch', () => {
  const profile=renderCharacterCard('Test goalie',sheet,undefined,{position:'Goalie',games:20,metrics:[]});
  const rail=classed(profile,'rk-front-attributes')[0];
  assert.deepEqual(all(rail).filter(n=>n.dataset.capabilityIndex!==undefined).map(s=>s.children[0].textContent),['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play']);
  assert.match(classed(profile,'rk-front-skills')[0].textContent,/Rebound Direction/);
  assert.doesNotMatch(rail.textContent,/Shooting|Defending/);
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
 for(const history of [undefined,{...h,player_id:2},{...h,seasons:[{season:20252026}]},{...h,assessment:{...h.assessment,last_season:20252026}}]){
  const view=renderEvidenceBrief(n,history,dossier);assert.match(view.textContent,/No history summary matched/);assert.doesNotMatch(view.textContent,/No qualifying repeated signal/);
 }
 const absent=renderEvidenceBrief({...n,leading:[]});assert.match(absent.textContent,/No selected measured signals/);assert.match(absent.textContent,/No task-specific gap list/);
});
