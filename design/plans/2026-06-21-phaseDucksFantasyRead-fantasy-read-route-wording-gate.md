# Phase Ducks Fantasy Read - Fantasy read route wording gate

> Phase Ducks Fantasy Read records Fantasy HTML/gaps/simulate read routes with
> precise FantasyDb, scenario, sidecar, and mutation boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Ducks Fantasy Read complete

---

## Frame

The Fantasy read routes already project roster gaps and scenario simulations
from existing local FantasyDb state. Phase Ducks Fantasy Read tightens the route
matrix so the rows name `FantasyRosterGapView`, `FantasySimulationView`,
read-only SQLite behavior, missing-db no-create behavior, scenario warnings,
unknown-drop errors, and browser mutation non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Ducks Fantasy Read Goal 1 - Route inventory** | Fantasy rows should name read-only state and mutation boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Ducks Fantasy Read Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Fantasy HTML/gaps/simulate tests pass. |
| 3 | **Ducks Fantasy Read Goal 3 - Scoped route wording** | Existing rows are accurate but terse for local-state safety. | Rows name ViewModels, existing-db reads, no-create/sidecar guards, scenario behavior, and mutation non-claims. |
| 4 | **Ducks Fantasy Read Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add browser league/team setup.
- Do not add roster import.
- Do not persist add/drop/drop-only scenarios.
- Do not add matchup schedule or roster-shape mutation.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused Fantasy read tests passed.
3. **Pulse 03 - Matrix wording.** Result: Fantasy rows now carry scoped
   read-only wording.
4. **Pulse 04 - Closeout.** Result: Phase Ducks Fantasy Read is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Ducks Fantasy Read closed the Fantasy read/product route wording gate. The
rows now record `FantasyRosterGapView` and `FantasySimulationView` projection
from existing FantasyDb state, read-only gaps and scenario JSON behavior,
missing-db and SQLite sidecar guards, unknown-drop warnings/errors, and browser
mutation non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Fantasy HTML/gaps/simulate route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
