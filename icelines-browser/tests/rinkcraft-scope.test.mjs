import test from 'node:test';
import assert from 'node:assert/strict';
import {matchesProfileWindow,renderProfileWindow} from '../dist/src/rinkcraft-scope.js';
const window={player_id:1,kind:'season',season:20242025,season_type:'regular',coverage:'selected_season',label:'Season · 2024–25',note:'Career requires complete coverage.',choices:[{kind:'season',label:'Season',available:true},{kind:'career',label:'Career incomplete',available:false},{kind:'current',label:'Current not loaded',available:false}]};
const patterns={schema:'rinkcraft.scoped-patterns.v1',player_id:1,season:20242025,season_type:'regular',group:'Forwards'};
test('whole profile and candidate scope must agree on actor, year, type and group',()=>{
 assert.equal(matchesProfileWindow(window,patterns,1,20242025,'regular','Forwards'),true);
 for(const [field,value]of [['player_id',2],['season',20252026],['season_type','playoff'],['group','Defense'],['schema','unknown']])assert.equal(matchesProfileWindow(window,{...patterns,[field]:value},1,20242025,'regular','Forwards'),false);
 assert.equal(matchesProfileWindow({...window,kind:'career'},patterns,1,20242025,'regular','Forwards'),false);
 assert.equal(matchesProfileWindow({...window,coverage:'partial'},patterns,1,20242025,'regular','Forwards'),false);
});
test('incomplete career and current options cannot relabel seasonal numbers',()=>{
 class Node{children=[];dataset={};constructor(tag){this.tag=tag;}append(...n){this.children.push(...n);}setAttribute(){}set textContent(v){this.text=v;}}
 globalThis.document={createElement:tag=>new Node(tag)};
 const root=renderProfileWindow(window),select=root.children[0].children[0];
 assert.equal(select.children[0].disabled,false);assert.equal(select.children[1].disabled,true);assert.equal(select.children[2].disabled,true);
 select.value='career';select.onchange();assert.equal(select.value,'season');
});
