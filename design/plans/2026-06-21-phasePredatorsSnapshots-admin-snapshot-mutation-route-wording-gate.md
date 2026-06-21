# Phase Predators Snapshots - Admin snapshot mutation route wording gate

> Phase Predators Snapshots records scoped snapshot activation and deletion rows
> with precise sealed/inactive guards and browser-maintenance boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Predators Snapshots complete

---

## Frame

The admin snapshot mutation routes already route through shared snapshot intents.
Phase Predators Snapshots tightens the route matrix so the rows name
`SnapshotMutationIntent`, sealed-only activation, inactive-only deletion,
active-snapshot delete rejection, `MutationResultView`, HTML redirects, and
browser-maintenance non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Predators Snapshots Goal 1 - Route inventory** | Snapshot mutation rows should name guards and non-claims. | A wave inventory names route rows, evidence, and boundaries. |
| 2 | **Predators Snapshots Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused admin snapshot mutation tests pass. |
| 3 | **Predators Snapshots Goal 3 - Scoped route wording** | Existing rows are accurate but terse for snapshot safety. | Rows name intent, guard, result/redirect behavior, and browser-maintenance non-claims. |
| 4 | **Predators Snapshots Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add web snapshot creation, sealing, or arbitrary maintenance.
- Do not allow active snapshot deletion.
- Do not bypass `SnapshotMutationIntent`.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused admin snapshot mutation tests passed.
3. **Pulse 03 - Matrix wording.** Result: snapshot mutation rows now carry scoped
   sealed/inactive guard wording.
4. **Pulse 04 - Closeout.** Result: Phase Predators Snapshots is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Predators Snapshots closed the admin snapshot mutation route wording gate.
The rows now record sealed-only activation, inactive-only deletion, shared
`SnapshotMutationIntent`, active-snapshot delete rejection, `MutationResultView`,
HTML redirects, and browser-maintenance non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused admin snapshot mutation route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
