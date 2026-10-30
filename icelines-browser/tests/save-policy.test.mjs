import test from 'node:test';
import assert from 'node:assert/strict';
import { reconcileSavePolicy } from '../dist/src/save-policy.js';

test('removing a saved context or revoking its policy disables refresh saving', () => {
  for (const entries of [[], [{ id: 'season', keepUpdated: false }], [{ id: 'other', keepUpdated: true }]]) {
    const dataset = { id: 'season', keepUpdated: true, bytes: new Uint8Array([1]) };
    const bytes = dataset.bytes;
    reconcileSavePolicy(dataset, entries);
    assert.equal(dataset.keepUpdated, false);
    assert.equal(dataset.bytes, bytes);
  }
});

test('library notification cannot grant saving consent to a session-only tab', () => {
  const dataset = { id: 'season', keepUpdated: false };
  reconcileSavePolicy(dataset, [{ id: 'season', keepUpdated: true }]);
  assert.equal(dataset.keepUpdated, false);
});
