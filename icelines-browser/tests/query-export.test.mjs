import test from 'node:test';
import assert from 'node:assert/strict';
import { csvCell, queryExport, queryCsv } from '../dist/src/query-export.js';

// Independent RFC 4180 reader checks generated records/field boundaries.
function records(csv) {
  const rows = []; let row = [], field = '', quoted = false;
  for (let i = 0; i < csv.length; i++) {
    const char = csv[i];
    if (char === '"') {
      if (quoted && csv[i + 1] === '"') { field += '"'; i++; }
      else quoted = !quoted;
    } else if (!quoted && char === ',') { row.push(field); field = ''; }
    else if (!quoted && char === '\r' && csv[i + 1] === '\n') { row.push(field); rows.push(row); row = []; field = ''; i++; }
    else field += char;
  }
  assert.equal(quoted, false); row.push(field); rows.push(row); return rows;
}
const request = {filter:'p>=100',sort:'points',goalies:false,minimum_games:null,today:'2026-10-03'};
const row = {player_id:7,name:'Slafkovský, "Juraj"',team:'MTL',position:'L',gp:10,goals:0,assists:0,points:0,pace_82:0,wins:null,save_pct:null,gaa:null};
const result = {schema_version:1,season:20242025,season_type:'regular',revision:'a'.repeat(64),source:'import\r\n=HYPERLINK("bad")',observed_at:null,fetched_at:'2026-10-03T00:00:00Z',missing_sources:['realtime'],minimum_games:0,pace_minimum_games:10,rows:[row]};

test('JSON carries exact query, provenance, floors, missing sources and full precision', () => {
  const exported = queryExport(result, request);
  assert.deepEqual(exported.query,{filter:'p>=100',sort:'points',kind:'skaters',requested_minimum_games:null,today:'2026-10-03'});
  assert.equal(exported.export_schema_version,1); assert.equal(exported.revision,result.revision);
  assert.equal(exported.observed_at,null); assert.equal(exported.fetched_at,result.fetched_at);
  assert.equal(exported.minimum_games,0); assert.equal(exported.pace_minimum_games,10);
  assert.deepEqual(exported.missing_sources,['realtime']); assert.deepEqual(exported.rows,[row]);
  assert.match(exported.methodology.history,/not era adjusted/);
  request.filter = 'edited after export'; assert.equal(exported.query.filter,'p>=100'); request.filter = 'p>=100';
});

test('CSV neutralizes spreadsheet formulas including leading whitespace and controls', () => {
  for (const value of ['=1+1','+SUM(A1)','-1+2','@SUM(A1)','  =1','\ufeff=1','\ttext','\rtext','\ntext','\u0000=1']) {
    assert.equal(records(csvCell(value))[0][0],"'" + value);
  }
  assert.equal(records(csvCell(-1.25))[0][0],'-1.25');
  assert.equal(records(csvCell(0))[0][0],'0'); assert.equal(records(csvCell(null))[0][0],'');
});

test('CSV quotes text, preserves Unicode/null/zero/precision and never creates metadata records', () => {
  const rows = records(queryCsv({...result,rows:[{...row,save_pct:0.923456789,gaa:null}, {...row,player_id:8,name:'=1+1',pace_82:null}]}, {...request,filter:'name="a,b"\nOR gp>=0'}));
  const header = rows.findIndex(value => value[0] === 'player_id');
  assert.ok(header > 0); assert.ok(rows.slice(0,header).every(value => value.length === 1 && value[0].startsWith('# ')));
  const metadata = Object.fromEntries(rows.slice(1,header).map(([value]) => { const equal=value.indexOf('='); return [value.slice(2,equal),JSON.parse(value.slice(equal+1))]; }));
  assert.equal(metadata.source,result.source); assert.equal(metadata.query.filter,'name="a,b"\nOR gp>=0');
  assert.equal(metadata.minimum_games,0); assert.equal(metadata.pace_minimum_games,10); assert.equal(metadata.observed_at,null);
  assert.equal(metadata.revision,result.revision); assert.deepEqual(metadata.missing_sources,['realtime']);
  assert.equal(rows.length,header+3); assert.equal(rows[header+1].length,12);
  assert.equal(rows[header+1][1],row.name); assert.equal(rows[header+1][8],'0');
  assert.equal(rows[header+1][10],'0.923456789'); assert.equal(rows[header+1][11],'');
  assert.equal(rows[header+2][1],"'=1+1"); assert.equal(rows[header+2][8],'');
});

test('goalie exports retain explicit zero floor and engine order, including empty results', () => {
  const goalie = {...request,goalies:true,sort:'gaa',minimum_games:0};
  assert.deepEqual(queryExport({...result,rows:[]},goalie).query,{filter:request.filter,sort:'gaa',kind:'goalies',requested_minimum_games:0,today:request.today});
  const rows=records(queryCsv({...result,rows:[]},goalie)); assert.equal(rows.at(-1)[0],'player_id');
  assert.deepEqual(queryExport({...result,rows:[{...row,player_id:9},row]},goalie).rows.map(row=>row.player_id),[9,7]);
});
