import test from 'node:test';
import assert from 'node:assert/strict';
import { historySeasonLabel, renderPlayerHistory } from '../dist/src/player-history.js';

class Node {
  children = []; attributes = {}; value = '';
  constructor(tag) { this.tag = tag; }
  append(...nodes) { this.children.push(...nodes); }
  setAttribute(key, value) { this.attributes[key] = value; }
  set textContent(value) { this.value = value; this.children = []; }
  get textContent() { return this.value + this.children.map(node => node.textContent).join(' '); }
}
globalThis.document = { createElement: tag => new Node(tag) };
const find = (node, tag) => [node, ...node.children.flatMap(child => find(child, tag))].filter(node => node.tag === tag);
const player = { player_id: 1, name: 'Player One', team: 'ALP', position: 'C', gp: 0, goals: 0, assists: 0,
  points: 0, pace_82: null, wins: null, save_pct: null, gaa: null };
const window = { season: 20252026, season_type: 'regular', revision: 'one', source: 'fixture',
  observed_at: null, fetched_at: null, goalie: false, player };

test('history identifies regular/playoff windows and distinguishes missing records from zero games', () => {
  assert.equal(historySeasonLabel(20252026, 'regular'), '2025–26 · Regular season');
  assert.equal(historySeasonLabel(20252026, 'playoff'), '2025–26 · Playoffs');
  const node = renderPlayerHistory({ schema_version: 1, player_id: 1, rows: [window,
    { ...window, season_type: 'playoff', player: null, goalie: null }] });
  const items = find(node, 'li'); assert.equal(items.length, 2);
  assert.deepEqual(find(items[0], 'dd').map(node => node.textContent), ['0', '0', '0', '0', '—']);
  assert.ok(!items[0].textContent.includes('No player record'));
  assert.match(items[1].textContent, /No player record.*does not mean zero games/);
  assert.equal(find(items[1], 'dd').length, 0);
  assert.match(node.textContent, /not a complete career/);
});

test('goalie history uses goalie values, preserves unknown dates and treats source text literally', () => {
  const source = '<img src=x onerror=alert(1)>';
  const node = renderPlayerHistory({ schema_version: 1, player_id: 1, rows: [
    { ...window, source, goalie: true, player: { ...player, wins: 0, save_pct: 0.912345, gaa: 2.345 } },
  ] });
  assert.deepEqual(find(node, 'dt').map(node => node.textContent), ['Games played', 'Wins', 'Save percentage', 'Goals against average']);
  assert.deepEqual(find(node, 'dd').map(node => node.textContent), ['0', '0', '0.912', '2.35']);
  assert.ok(node.textContent.includes(source)); assert.equal(find(node, 'img').length, 0);
  assert.match(node.textContent, /Observed unknown.*Fetched unknown/);
});

test('older detail responses keep the dialog usable with an explicit unavailable history', () => {
  const node = renderPlayerHistory(); assert.match(node.textContent, /history is unavailable/);
  assert.equal(find(node, 'li').length, 0);
});
