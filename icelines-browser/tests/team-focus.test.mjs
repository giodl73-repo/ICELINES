import test from 'node:test';
import assert from 'node:assert/strict';
import { teamSeasonFilter, playerTeamCodes } from '../dist/src/team-focus.js';

test('team navigation only generates a single literal Rust season-stint predicate', () => {
  assert.equal(teamSeasonFilter(' mtl '), 'team.any=MTL');
  assert.equal(teamSeasonFilter('NY'), 'team.any=NY');
  for (const code of ['', 'A', '123', 'ABCDEFGHI', 'MTL OR gp>0', 'MTL/SEA', '<script>', 'MTL\nAND']) {
    assert.throws(() => teamSeasonFilter(code), /team code/);
  }
});

test('player shortcuts preserve distinct season teams without trusting imported syntax', () => {
  assert.deepEqual(playerTeamCodes('SEA/NYR/SEA'), ['SEA', 'NYR']);
  assert.deepEqual(playerTeamCodes('—/UNK/<img>/MTL OR gp>0'), []);
  assert.deepEqual(playerTeamCodes(''), []);
});
