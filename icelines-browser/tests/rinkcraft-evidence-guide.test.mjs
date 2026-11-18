import test from 'node:test';
import assert from 'node:assert/strict';
import {renderEvidenceGuide} from '../dist/src/rinkcraft-evidence-guide.js';
class Element {
  children=[]; dataset={};
  constructor(tag){this.tag=tag;}
  append(...nodes){this.children.push(...nodes);}
  set textContent(value){this.text=value;this.children=[];}
  get textContent(){return (this.text??'')+this.children.map(c=>c.textContent).join(' ');}
}
globalThis.document={createElement:tag=>new Element(tag)};
const topic=scope=>({scope,key:scope,title:scope,question:'<img src=x>',evidence:'Task definition',boundary:'Not a rating'});
const topics=['all','skater','goalie','unrecognized'].map(topic);
const keys=n=>n.children.find(n=>n.className==='rk-guide-topics').children.map(c=>c.dataset.guideTopic);
test('guide keeps goalie, skater and unknown branches separate',()=>{
  assert.deepEqual(keys(renderEvidenceGuide(topics,'D')),['all','skater']);
  assert.deepEqual(keys(renderEvidenceGuide(topics,'Goalies')),['all','goalie']);
  assert.deepEqual(keys(renderEvidenceGuide(topics,'Unknown')),['all']);
  assert.deepEqual(keys(renderEvidenceGuide(topics,'invented')),['all']);
  assert.equal(renderEvidenceGuide(undefined,'D'),undefined);
  assert.equal(renderEvidenceGuide([],'D'),undefined);
});
test('guide is closed by default and renders explanation as literal text',()=>{
  const guide=renderEvidenceGuide(topics,'C');
  assert.equal(guide.tag,'details');assert.equal(guide.open,undefined);
  const all=n=>[n,...n.children.flatMap(all)];
  assert.equal(all(guide).some(n=>n.tag==='img'),false);
  assert.match(guide.textContent,/<img src=x>/);
  assert.match(guide.textContent,/not player observations/);
});
