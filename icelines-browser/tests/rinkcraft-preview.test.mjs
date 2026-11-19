import test from 'node:test';
import assert from 'node:assert/strict';
import {filterPreviewPlayers,validatePreviewCatalog,captureLabel} from '../dist/src/rinkcraft-preview.js';
const players=[{id:1,name:'Éric Example',position:'D',group:'Defense',teams:['NYR','SEA'],games:50,url:'data/20242025/1.json'},{id:2,name:'Goalie Example',position:'Goalie',group:'Goalies',teams:['SEA'],games:19,url:'data/20242025/2.json'}];
const c={schema:'rinkcraft.public-preview.v1',current_season:20242025,windows:[{season:20242025,captured_through:'2026-11-18',players}]};
test('name search handles accents, team changes and combined filters',()=>{
 assert.deepEqual(filterPreviewPlayers(players,'eric sea','NYR','Defense').map(p=>p.id),[1]);
 assert.deepEqual(filterPreviewPlayers(players,'','SEA','Goalies').map(p=>p.id),[2]);
 assert.deepEqual(filterPreviewPlayers(players,'missing','',''),[]);
});
test('season inventory refuses duplicate actors, unexpected paths and missing current coverage',()=>{
 assert.equal(validatePreviewCatalog(c),c);
 for(const mutate of [x=>x.windows[0].players.push(x.windows[0].players[0]),x=>x.windows[0].players[0].url='https://example.com/private',x=>x.current_season=20252026,x=>x.windows.push(x.windows[0]),x=>x.windows[0].players[0].games=-1]){
  const x=structuredClone(c);mutate(x);assert.throws(()=>validatePreviewCatalog(x));
 }
});
test('date-only captures do not invent a time',()=>{assert.equal(captureLabel('2026-11-18'),'2026-11-18 (capture date)');assert.match(captureLabel('2026-11-19T22:52:41Z'),/capture time/);});
