import test from 'node:test';
import assert from 'node:assert/strict';
import { PlayerTable } from '../dist/src/player-table.js';

// Small DOM double checks ownership and identity; browser acceptance exercises
// the actual DOM, worker queries and dialog interaction.
class Node {
  children = []; parent = null; onclick = null; nodeValue = null;
  constructor(tag) { this.tag = tag; }
  append(...nodes) {
    for (const node of nodes) {
      if (node.tag === 'fragment') { this.append(...[...node.children]); continue; }
      if (node.parent) node.parent.children.splice(node.parent.children.indexOf(node), 1);
      this.children.push(node); node.parent = this;
    }
  }
  replaceChildren(...nodes) { for (const node of this.children) node.parent = null; this.children = []; this.append(...nodes); }
  set textContent(value) { this.replaceChildren(Object.assign(new Node('text'), { nodeValue: value })); }
  get textContent() { return this.nodeValue ?? this.children.map(node => node.textContent).join(''); }
}
globalThis.document = {
  createElement: tag => new Node(tag), createDocumentFragment: () => new Node('fragment'),
  createTextNode: value => Object.assign(new Node('text'), { nodeValue: value }),
};
const row = { player_id: 1, name: '<img src=x onerror=alert(1)>', team: 'MTL', gp: 10, goals: 0, assists: 2, points: 2, pace_82: null, wins: 0, save_pct: 0.912345, gaa: null };
const fixture = () => { const body = new Node('tbody'), head = new Node('thead'), opened = []; return { body, head, opened, table: new PlayerTable(body, head, value => opened.push(value)) }; };

test('reuses row, text and header nodes but opens the current player after query changes', () => {
  const { table, body, head, opened } = fixture(); table.render([row], false);
  const tr = body.children[0], header = head.children[0], button = tr.children[1].children[0], text = button.children[0];
  assert.equal(text.nodeValue, row.name); assert.equal(button.children.length, 1); assert.equal(text.tag, 'text');
  assert.equal(tr.children[4].textContent, '0'); assert.equal(tr.children[7].textContent, '—');
  table.clear(false); assert.equal(body.children.length, 0); assert.equal(button.onclick, null); assert.equal(text.nodeValue, '');
  const next = { ...row, player_id: 2, name: 'Next season player' }; table.render([next], false);
  assert.equal(body.children[0], tr); assert.equal(head.children[0], header); assert.equal(button.children[0], text);
  button.onclick(); assert.deepEqual(opened, [next]);
});

test('shorter results erase spare names and callbacks; unload releases the pool', () => {
  const { table, body } = fixture(); table.render([row, { ...row, player_id: 2 }], false);
  const spare = body.children[1], spareButton = spare.children[1].children[0];
  table.render([row], false); assert.equal(spare.textContent, ''); assert.equal(spareButton.onclick, null);
  table.render([row, row], false); assert.notEqual(body.children[1], spare);
  const previous = body.children[0]; table.clear(); table.render([row], false);
  assert.notEqual(body.children[0], previous); assert.equal(previous.textContent, '');
});

test('goalie transitions rebuild column shape and retain null, zero and display precision', () => {
  const { table, body, head } = fixture(); table.render([row], false); const skater = body.children[0];
  table.render([row], true); const goalie = body.children[0];
  assert.notEqual(goalie, skater); assert.equal(skater.textContent, '');
  assert.equal(goalie.children.length, 7); assert.equal(head.children[0].children.length, 7);
  assert.equal(goalie.children[4].textContent, '0'); assert.equal(goalie.children[5].textContent, '0.912'); assert.equal(goalie.children[6].textContent, '—');
});

test('pool is bounded without limiting results', () => {
  const { table, body } = fixture(); const rows = Array.from({ length: 1030 }, (_, i) => ({ ...row, player_id: i }));
  table.render(rows, false); const retained = body.children[1023], beyondPool = body.children[1024];
  assert.equal(body.children.length, 1030); table.render(rows, false);
  assert.equal(body.children.length, 1030); assert.equal(body.children[1023], retained); assert.notEqual(body.children[1024], beyondPool);
  assert.equal(beyondPool.textContent, ''); assert.equal(beyondPool.children[1].children[0].onclick, null);
});
