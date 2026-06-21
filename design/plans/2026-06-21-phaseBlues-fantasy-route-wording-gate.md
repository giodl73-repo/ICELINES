# Phase Blues - Fantasy route wording gate

> Phase Blues records Fantasy Web/API read-product route rows with precise
> scoped wording, using the boundaries already in the Fantasy family row.

**Created:** 2026-06-21
**Status:** Active - route wording passed

---

## Frame

Fantasy read/product views are already implemented through shared ViewModels:
`FantasyRosterGapView`, `FantasySimulationView`, `FantasyDailyDeltaView`,
`FantasyMatchupWeekView`, and `RosterShapeValidationView`. CLI remains the
canonical mutation surface for league/team setup, Yahoo roster import, matchup
schedule setup, and roster-shape presets.

The remaining issue is route-row precision. The route inventory still uses
terse `done` wording for `/fantasy` and `/api/v1/fantasy/*` rows. Phase Blues
tightens those rows without implying browser mutation support or GET-backed
state changes.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Blues Goal 1 - Route inventory** | Route-level wording should match the Fantasy family row. | A wave inventory names route pairs, evidence, and blockers. |
| 2 | **Blues Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Fantasy Web route tests pass. |
| 3 | **Blues Goal 3 - Scoped read/product wording** | Terse `done` wording hides important mutation deferrals. | Route rows name ViewModels and preserve exact blockers. |
| 4 | **Blues Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add browser league/team setup or Yahoo roster import.
- Do not add GET-backed roster-shape or matchup schedule mutation.
- Do not change Fantasy runtime behavior.
- Do not weaken missing-state or read-only SQLite sidecar guards.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Fantasy Web route tests passed
   and support scoped read/product wording.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   read/product wording while preserving mutation and local-state deferrals.
4. **Pulse 04 - Closeout.** Close Phase Blues with exact route claims and
   non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Fantasy Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
