import test from 'node:test';
import assert from 'node:assert/strict';
import { parseViewHash, serializeViewHash, isPublicDataset, localDatasetForView } from '../dist/src/view-state.js';
const view = {season:20242025,type:'playoff',kind:'goalies',sort:'gaa',gp:'0',data:'public',filter:'sv%>=0.9'};
const catalog = [{id:'public-season',sha256:'digest',season:20242025,season_type:'playoff'}];
const dataset = (id, revision = 'local-digest', season = 20242025, type = 'playoff') => ({id,revision,metadata:{season,season_type:type}});
test('all view controls round-trip and explicit zero remains distinct from default', () => {
  assert.deepEqual(parseViewHash(serializeViewHash(view, true)), view);
  const defaults = parseViewHash('#/leaders?season=20242025');
  assert.equal(defaults.gp, ''); assert.equal(defaults.kind, 'skaters'); assert.equal(defaults.sort, 'points'); assert.equal(defaults.type, 'regular');
});
test('normal bookmarks omit private filter text and identifiers', () => {
  const hash = serializeViewHash({...view,filter:'private note: my roster'});
  assert.equal(parseViewHash(hash).filter, ''); assert.ok(!hash.includes('private'));
  assert.deepEqual([...new URLSearchParams(hash.split('?')[1]).keys()], ['season','type','kind','sort','gp']);
});
test('public filter inclusion is explicit and URL-encoded without changing meaning', () => {
  const filter = 'name="Slafkovský" AND p>=50 & gp>=10';
  const hash = serializeViewHash({...view,kind:'skaters',sort:'points',filter},true);
  assert.equal(parseViewHash(hash).filter, filter); assert.ok(hash.includes('%C3%BD')); assert.ok(hash.includes('%26'));
});
test('local links never include filters even with public-filter option requested', () => {
  const hash = serializeViewHash({...view,data:'local',filter:'my private note'}, true);
  assert.equal(parseViewHash(hash).data, 'local'); assert.equal(parseViewHash(hash).filter, '');
  assert.throws(() => parseViewHash(hash + '&filter=private'), /unavailable/);
});
test('only exact public catalog identity, revision and context count as public', () => {
  assert.equal(isPublicDataset(dataset('public-season','digest'),catalog),true);
  for (const value of [dataset('import-secret','digest'),dataset('public-season','live-revision'),dataset('public-season','digest',20232024),dataset('public-season','digest',20242025,'regular')]) assert.equal(isPublicDataset(value,catalog),false);
});
test('local restoration respects context and active ID, and refuses ambiguous substitutes', () => {
  const local = {...view,data:'local'}; const first = dataset('first'), second = dataset('second');
  const records = [dataset('public-season','digest'), first, second, dataset('wrong-season','x',20232024)];
  assert.equal(localDatasetForView(records,'first',local,catalog),first);
  assert.equal(localDatasetForView(records,'public-season',local,catalog),undefined);
  assert.equal(localDatasetForView([dataset('public-season','digest'),first],undefined,local,catalog),first);
  assert.equal(localDatasetForView([dataset('public-season','digest')],undefined,local,catalog),undefined);
});
test('unsupported paths, arbitrary URLs, duplicate and unknown parameters are refused', () => {
  for (const hash of ['#/other?season=20242025','#/leaders?season=20242025&url=https://evil.test', '#/leaders?season=20242025&season=20232024','#/leaders?season=20242025&kind=teams','#/leaders?season=20242025&type=all','#/leaders?season=20242025&data=https://evil.test','#/leaders?season=20242025&kind=goalies&sort=points']) assert.throws(() => parseViewHash(hash));
  assert.equal(parseViewHash(''),undefined); assert.equal(parseViewHash('#analysis'),undefined);
});
test('season, GP and text bounds reject malformed or unavailable state', () => {
  for (const query of ['season=20242026','season=20042005','season=x','season=20242025&gp=-1','season=20242025&gp=1.5','season=20242025&gp=1001','season=20242025&filter='+ 'x'.repeat(4097)]) assert.throws(() => parseViewHash('#/leaders?'+query));
  assert.throws(() => parseViewHash('#/leaders?season=20242025&filter='+'x'.repeat(8192)));
});
