# Browser WASM pulse 07 — bounded worker recovery

Date: 2026-10-03. Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Implementation

Engine client now owns a worker epoch and request timers. A failure terminates
the worker, rejects every outstanding promise, advances context and invalidates
results. Constructor failure, send/decode failure, malformed response envelope,
initialization failure, WASM runtime trap and a two-minute watchdog share that
path. Old workers cannot deliver responses into a new engine. Ordinary query
errors remain recoverable without restarting. Context changes reject obsolete
requests immediately instead of waiting for a late response.

The UI attempts one restart, loads the active dataset's saved revision from
IndexedDB and reruns the current query. A repeated failure during recovery stops
automatic retries and allows an explicit Restart engine. Session-only data is
not restored into the engine. Its original bytes remain in a clearly labeled
export-only recovery copy in the tab; closing/reloading loses those unsaved bytes.
No automatic persistent save or network acquisition is performed by recovery.
Existing query/detail UI is cleared/closed on failure, with exports disabled.

## Evidence

`npm test --prefix icelines-browser` passed all 47 Node tests, including ten new
client lifecycle tests for cancellation, old responses, crashes, constructor/
serialization/initialization failures, protocol validation and timeout. Build
and TypeScript compilation passed. `node scripts/verify-browser-dist.mjs`
verified all 75 packages through actual WASM and all 15 shell assets.

`scripts/prepare-browser-recovery-preview.mjs` creates a local-only acceptance
harness outside dist. At `http://127.0.0.1:8051/`, browser actions proved:

- Memory-only 20242025 regular data: real worker termination plus a synthetic
  error event cleared active/results, left the recovery export visible, disabled
  query export, and restarted without automatically saving/restoring that data.
- After explicit saving, the same fault rebuilt a real WASM worker and restored
  the saved season with 905 players, removing the redundant recovery copy.
- Blocking worker construction stopped recovery with a visible error and an
  enabled manual restart control; unblocking and restarting restored 905 players.
- The restored shared `p>=100` query returned six players led by Kucherov.

[Restored browser evidence](browser-pulse-07.png). Server session 68601, browser
tab 8 marked for continuation. The harness modifies index.html deliberately;
offline installation rejects its mismatched digest, as shown in the screenshot.
This is expected harness behavior and does not prove or disprove offline reopening
of the unmodified distribution. Native browser process/OOM crashes were not forced.

## Remaining scope

The goal remains active. Recovery during simultaneous storage/acquisition work,
storage quota/schema/version-change recovery, complete request payload/error
contracts, URL restoration, deployed live access, offline/update acceptance,
Pages publication/rollback and performance budgets remain open. Native web/CLI
skater ordering differences still need reconciliation. No shipped status changes.
