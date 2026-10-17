import test from 'node:test';
import assert from 'node:assert/strict';
import { rowWindow } from '../dist/src/viewport-table.js';

test('viewport windows are bounded and cover every visible rank through the last player', () => {
  const count = 933;
  for (let offset = -800; offset < count * 72; offset += 41) {
    const [start, end] = rowWindow(count, offset, 800);
    assert.ok(start >= 0 && end <= count && end >= start);
    assert.ok(end - start <= 20);
    const firstVisible = Math.max(0, Math.floor(offset / 72));
    const lastVisible = Math.min(count - 1, Math.floor((offset + 799) / 72));
    if (firstVisible < count) assert.ok(start <= firstVisible && end > firstVisible);
    if (lastVisible >= 0) assert.ok(end > lastVisible);
  }
  assert.deepEqual(rowWindow(0, 100, 800), [0, 0]);
  assert.deepEqual(rowWindow(8, 0, 800), [0, 8]);
  const [terminalStart, terminalEnd] = rowWindow(933, 100000, 800);
  assert.equal(terminalEnd, 933);
  assert.ok(terminalStart <= 932 && terminalEnd - terminalStart <= 20);
});
