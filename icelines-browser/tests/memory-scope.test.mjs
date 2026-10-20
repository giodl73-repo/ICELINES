import test from 'node:test';
import assert from 'node:assert/strict';
import { normalWorkloadPhases } from '../e2e/memory-scope.mjs';

test('a normal engine restart remains part of an unmodified workload', () => {
  const phases = [{ phase: 'blank_browser' }, { phase: 'resident_settled' }, { phase: 'engine_restarted' }];
  assert.deepEqual(normalWorkloadPhases(phases), phases);
});

test('diagnostic peaks and post-intervention restart cannot enter the control budget', () => {
  for (const diagnostic of ['diagnostic_main_gc', 'diagnostic_rendering_0_visible']) {
    const phases = [{ phase: 'resident_settled', peak_private_bytes: 600 }, { phase: diagnostic, peak_private_bytes: 900 }, { phase: 'engine_restarted', peak_private_bytes: 1000 }];
    assert.deepEqual(normalWorkloadPhases(phases), [phases[0]]);
  }
});

test('a run starting with an intervention supplies no normal control phases', () => {
  assert.deepEqual(normalWorkloadPhases([{ phase: 'diagnostic_before_gc' }, { phase: 'engine_restarted' }]), []);
});
