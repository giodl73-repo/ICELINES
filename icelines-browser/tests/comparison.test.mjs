import test from 'node:test';
import assert from 'node:assert/strict';
import { PlayerComparison, comparisonStats } from '../dist/src/comparison.js';

const context = { revision: 'revision-a', season: 20252026, season_type: 'regular', goalies: false };
const row = { player_id: 1, name: 'Player One', team: 'ALP', position: 'C', gp: 9,
  goals: 0, assists: 1, points: 1, pace_82: null, wins: null, save_pct: null, gaa: null };

test('comparison owns at most two distinct player projections, with explicit removal', () => {
  const selection = new PlayerComparison(); selection.setContext(context);
  selection.add(row, context); selection.add(row, context);
  assert.equal(selection.players.length, 1);
  row.name = 'Changed elsewhere'; assert.equal(selection.players[0].name, 'Player One');
  selection.add({ ...row, player_id: 2 }, context);
  assert.throws(() => selection.add({ ...row, player_id: 3 }, context), /Remove a player/);
  assert.deepEqual(selection.players.map(p => p.player_id), [1, 2]);
  selection.remove(1); selection.add({ ...row, player_id: 3 }, context);
  assert.deepEqual(selection.players.map(p => p.player_id), [2, 3]);
  selection.clear(); assert.equal(selection.players.length, 0);
});

test('same-context filter changes preserve picks, while revision/window/kind changes clear them', () => {
  for (const change of [{ revision: 'revision-b' }, { season: 20242025 }, { season_type: 'playoff' }, { goalies: true }]) {
    const selection = new PlayerComparison(); selection.setContext(context); selection.add(row, context);
    selection.setContext({ ...context }); assert.equal(selection.players.length, 1);
    selection.setContext({ ...context, ...change }); assert.equal(selection.players.length, 0);
    assert.throws(() => selection.add(row, context), /dataset changed/);
  }
  const selection = new PlayerComparison(); selection.setContext(context); selection.add(row, context);
  selection.setContext(); assert.equal(selection.players.length, 0);
  assert.throws(() => selection.add(row, context), /dataset changed/);
});

test('comparison preserves zero versus missing and uses Rust pace and goalie values without recomputing', () => {
  assert.deepEqual(comparisonStats(row, false), [
    ['Games played', '9'], ['Goals', '0'], ['Assists', '1'], ['Points', '1'], ['Pace per 82', '—'],
  ]);
  assert.equal(comparisonStats({ ...row, pace_82: 0 }, false).at(-1)[1], '0.0');
  assert.deepEqual(comparisonStats({ ...row, wins: 0, save_pct: 0.912345, gaa: null }, true), [
    ['Games played', '9'], ['Wins', '0'], ['Save percentage', '0.912'], ['Goals against average', '—'],
  ]);
});
