import test from 'node:test';
import assert from 'node:assert/strict';
import { profileValue, renderRinkcraftProfile } from '../dist/src/rinkcraft-profile.js';
class Node {
  children = []; attributes = {}; value = ''; checked = false;
  constructor(tag) { this.tag = tag; }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.children = nodes; }
  setAttribute(key, value) { this.attributes[key] = value; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? '') + this.children.map(n => n.textContent).join(' '); }
}
globalThis.document = { createElement: tag => new Node(tag) };
const find = (n, tag) => [n, ...n.children.flatMap(c => find(c, tag))].filter(n => n.tag === tag);
const metric = { key:'goals',label:'Goals',unit:'Count',value:0,status:'available',aggregation:'catalog_read',observed_seasons:1,missing_seasons:[],report_source:null,ratio:null };
test('profile formatting keeps zero, null, seconds and percentage scale distinct', () => {
  assert.equal(profileValue(metric),'0');
  assert.equal(profileValue({...metric,value:null}),'—');
  assert.equal(profileValue({unit:'Pct',value:0.915}),'91.50%');
  assert.equal(profileValue({unit:'Seconds',value:91.5}),'91.5 s');
});
test('profile selectors preserve unavailable metrics and literal evidence text', () => {
  const section=renderRinkcraftProfile({player_name:'Test',request:{season_type:'regular'},included_seasons:[20252026],latest_available:{season:20252026},
    summary:[metric],timeline:[{season:20252026,games:1,position:'Defense',teams:['AAA'],traded:false,metrics:[metric,{...metric,key:'hits',label:'<img src=x>',value:null,status:'unavailable'}]}],
    job_families:[{name:'Prevent threats',catalog_keys:['goals'],interpretation:'Evidence, not a score'}],changes:[],limitations:['Unknown coverage'],unmeasured_dimensions:['style']});
  assert.match(section.textContent,/Latest: 2025–26/);
  assert.equal(find(section,'tbody')[0].children.length,1);
  const checkbox=find(section,'input').find(n=>n.type==='checkbox'); checkbox.checked=true;checkbox.onchange();
  assert.equal(find(section,'tbody')[0].children.length,2);
  assert.match(section.textContent,/—.*unavailable/);
  assert.equal(find(section,'img').length,0);
  const search=find(section,'input').find(n=>n.type==='search');search.value='goals';search.oninput();
  assert.equal(find(section,'tbody')[0].children.length,1);
});
