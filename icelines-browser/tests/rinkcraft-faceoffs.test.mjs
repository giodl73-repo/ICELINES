import test from 'node:test';import assert from 'node:assert/strict';
import {matchesTaskEvidence,matchesFaceoffPattern} from '../dist/src/rinkcraft-faceoffs.js';
const scope={player_id:1,kind:'season',season:20242025,season_type:'regular'};
const e={schema:'rinkcraft.task-evidence.v1',scope_kind:'season',player_id:1,season:20242025,season_type:'regular',group:'Forwards',source_urls:['https://api.nhle.com/stats/rest/en/skater/faceoffwins?cayenneExp=seasonId%3D20242025%20and%20gameTypeId%3D2'],source_sha256:'a'.repeat(64),screen_sha256:'b'.repeat(64),rules_sha256:'c'.repeat(64),captured_through:'2026-11-19T15:00:00Z'};
const task={id:'draw-specialist',opportunities:{wins:0,losses:0,draws:0,games:20},evidence:e,task_record:{schema:'rinkcraft.faceoff-record.v1',wins:0,losses:0,attempts:0,games:20,fraction:null}};
test('faceoff records require exact child source and whole-card scope',()=>{
 assert.equal(matchesFaceoffPattern(task,scope,'Forwards'),true);
 for(const [key,value]of [['player_id',2],['season',20252026],['season_type','playoff'],['group','Defense'],['scope_kind','career'],['schema','unknown'],['source_sha256','bad'],['captured_through','invalid']])assert.equal(matchesFaceoffPattern({...task,evidence:{...e,[key]:value}},scope,'Forwards'),false);
 for(const kind of ['current','career'])assert.equal(matchesFaceoffPattern(task,{...scope,kind},'Forwards'),false);
 assert.equal(matchesFaceoffPattern({...task,task_record:{...task.task_record,wins:1}},scope,'Forwards'),false);
});
test('old patterns remain compatible; new source URLs cannot cross windows',()=>{
 assert.equal(matchesFaceoffPattern({id:'pp-efficiency'},scope,'Forwards'),true);
 for(const urls of [[],['bad'],['https://other.test/'],[e.source_urls[0].replace('20242025','20252026')],[e.source_urls[0].replace('gameTypeId%3D2','gameTypeId%3D3')]])assert.equal(matchesTaskEvidence({...e,source_urls:urls},scope,'Forwards'),false);
});
test('older history binds its own source year; zero attempts stay an observed count',()=>{
 const old={...e,season:20232024,source_urls:[e.source_urls[0].replace('20242025','20232024')]};assert.equal(matchesTaskEvidence(old,scope,'Forwards',20232024),true);assert.equal(matchesTaskEvidence(e,scope,'Forwards',20232024),false);
 assert.equal(matchesFaceoffPattern(task,scope,'Forwards'),true);assert.equal(task.task_record.fraction,null);assert.equal(task.task_record.attempts,0);
});
