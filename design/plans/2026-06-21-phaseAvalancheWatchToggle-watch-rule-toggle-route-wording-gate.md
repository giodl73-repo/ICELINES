# Phase Avalanche Watch Toggle - Watch-rule toggle route wording gate

> Phase Avalanche Watch Toggle records persisted player-rule enabled toggles
> with precise intent, storage, redirect, and non-editing boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Avalanche Watch Toggle complete

---

## Frame

The watch-rule toggle routes already mutate persisted player-rule enabled state
through a shared mutation intent. Phase Avalanche Watch Toggle tightens the route
matrix so the rows name `WatchRuleMutationIntent::set_enabled`, persisted
player-rule scope, stored `enabled` updates, `MutationResultView`, `/watchlist`
redirects, and default-rule/team-deployment/event non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Avalanche Watch Toggle Goal 1 - Route inventory** | Toggle rows should name persisted-rule scope and non-claims. | A wave inventory names route rows, evidence, and boundaries. |
| 2 | **Avalanche Watch Toggle Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused watch-rule toggle tests pass. |
| 3 | **Avalanche Watch Toggle Goal 3 - Scoped route wording** | Existing rows are accurate but terse for rule safety. | Rows name intent, id validation, stored enabled updates, results, redirects, and non-claims. |
| 4 | **Avalanche Watch Toggle Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add default-rule mutation.
- Do not add arbitrary team/deployment rule editing.
- Do not fire watch-rule events from toggle routes.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused watch-rule toggle tests passed.
3. **Pulse 03 - Matrix wording.** Result: toggle rows now carry scoped
   persisted-rule wording.
4. **Pulse 04 - Closeout.** Result: Phase Avalanche Watch Toggle is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Avalanche Watch Toggle closed the watch-rule toggle route wording gate.
The rows now record persisted player-rule `enabled` updates through
`WatchRuleMutationIntent`, JSON `MutationResultView`, HTML `/watchlist`
redirects, and default-rule/team-deployment/event non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused watch-rule toggle route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
