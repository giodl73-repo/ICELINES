import test from 'node:test';
import assert from 'node:assert/strict';
import { digest } from '../dist/src/acquisition.js';

const expected = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';
test('SHA-256 hashes only the input view and snapshots bytes before caller mutation', async () => {
  const padded = new Uint8Array([255,97,98,99,255]);
  const pending = digest(padded.subarray(1,4));
  padded.fill(0);
  assert.equal(await pending,expected);
});
test('shared-backed input retains the same SHA-256 meaning', async () => {
  const shared = new Uint8Array(new SharedArrayBuffer(5));
  shared.set([255,97,98,99,255]);
  assert.equal(await digest(shared.subarray(1,4)),expected);
});
