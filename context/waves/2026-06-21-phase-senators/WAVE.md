# Phase Senators

## Scope

Plan and execute the admin operation row-wording gate after Phase Flyers. The
wave does not reopen the Flyers safety decisions; it records the remaining
admin rows as scoped partials by design where the current route/test evidence
already proves safe operational behavior.

## Entry Posture

- Phase Flyers closed the admin operation safety gate.
- The Admin operations rollup already says runtime active-season config, data
  verify, snapshot activate/delete, and game-cache warmers are POST-backed and
  covered by focused `l1_admin_` route tests.
- The individual admin rows still start with plain `partial -` wording, which
  makes intentional deferrals harder to distinguish from unresolved drift.
- Web data install/remove remain deferred and unmounted.
- Persistent report-toggle writes remain a CLI/TUI durable config handoff.

## Goals

1. Inventory the individual admin operation rows and the Flyers evidence they
   depend on.
2. Validate focused admin route evidence for the scoped safe-operation claims.
3. Tighten the Data install/list/remove, Snapshot operations, and Config/report
   toggles rows so their partial status is explicit and by design.
4. Preserve exact non-claims around install/remove, persistent report-toggle
   writes, GET mutations, and live-network/destructive browser operations.
5. Close the phase with the final matrix wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Senators goals | passed; see `SENATORS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Admin route evidence gate | pending |
| 03 | Admin row wording gate | pending |
| 04 | Close Phase Senators | pending |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused admin route tests.
- No live network dependency in tests.
