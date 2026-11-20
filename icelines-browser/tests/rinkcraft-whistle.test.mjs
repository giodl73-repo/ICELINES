import test from 'node:test';
import assert from 'node:assert/strict';
import {whistlePresentation} from '../dist/src/rinkcraft-scope.js';
const pattern={id:'penalty-balance',name:'Whistle Balance',state:'earned',opportunities:{penalties_drawn:3,penalties_taken:5},whistle_record:{schema:'rinkcraft.whistle-record.v1',drawn:3,taken:5,net:-2,sign:'negative',display:'Drawn 3 · taken 5 · net -2',note:'Relative peer ranks do not require positive net.'}};
test('relative qualifier can display negative signed record without changing its award',()=>{const p=whistlePresentation(pattern);assert.equal(p.name,'Relative Whistle Record');assert.equal(p.record.net,-2);assert.equal(p.record.sign,'negative');assert.equal(pattern.state,'earned');});
test('zero is real data, missing or mismatched counts remain unavailable',()=>{
 const zero={...pattern,opportunities:{penalties_drawn:0,penalties_taken:0},whistle_record:{...pattern.whistle_record,drawn:0,taken:0,net:0,sign:'zero',display:'Drawn 0 · taken 0 · net +0'}};assert.equal(whistlePresentation(zero).record.net,0);
 for(const p of [{...pattern,whistle_record:null},{...pattern,opportunities:{}},{...pattern,opportunities:{penalties_drawn:9,penalties_taken:5}},{...pattern,whistle_record:{...pattern.whistle_record,schema:'unknown'}}]){assert.equal(whistlePresentation(p).record,undefined);assert.match(whistlePresentation(p).note,/unavailable/);}
});
test('other patterns retain their name and receive no whistle diagnostic',()=>{assert.deepEqual(whistlePresentation({...pattern,id:'pp-efficiency',name:'PP Production Rate'}),{name:'PP Production Rate'});});
