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
  assert.doesNotMatch(front.textContent,/Detailed recovery account/);
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
