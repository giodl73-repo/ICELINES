# Phase Red Wings - Favorites/watch boundary gate

> Phase Red Wings decides whether Favorites/watch/watch-rules should remain a
> deliberate narrow partial or expand into richer group/rule editing.

**Created:** 2026-06-21
**Status:** Active - pulse 03 matrix wording passed

---

## Frame

Favorites, named groups, watchlists, and watch rules have useful read and
POST-backed mutation paths today. The remaining partial is narrower: arbitrary
group create/rename/delete/member edits and arbitrary team/deployment watch-rule
editing are intentionally deferred because the shared mutation contracts do not
yet carry validated fields for those dimensions.

Phase Red Wings audits that boundary and only promotes the row if the existing
contracts support the richer behavior without GET mutation or ambiguous command
reinterpretation.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Red Wings Goal 1 - Surface inventory** | Evidence is split across favorites, watchlist, watch-rule, dashboard command, and TUI command surfaces. | A wave inventory names supported paths and blockers. |
| 2 | **Red Wings Goal 2 - Evidence gate** | The partial needs test evidence for both supported paths and explicit refusals. | Focused favorites/watch tests pass across CLI/Web/dashboard where applicable. |
| 3 | **Red Wings Goal 3 - Contract decision** | Richer group/rule editing should not ship without shared validated mutation intents. | Decision recorded as keep partial, promote, or defer. |
| 4 | **Red Wings Goal 4 - Matrix closeout** | The matrix must distinguish deliberate narrow scope from missing verification. | `design/specs/surface-parity.md` carries exact final wording. |

---

## Non-goals

- Do not add GET-backed mutations.
- Do not add a group schema migration.
- Do not add arbitrary team/deployment watch-rule editing without a shared
  validated mutation contract.
- Do not reinterpret unsupported commands as narrower successful mutations.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused CLI/Web favorites/watch
   evidence supports keeping the partial narrow by design.
3. **Pulse 03 - Contract decision and matrix wording.** Keep partial by design
   or promote only if the evidence justifies it. Result: matrix keeps
   Favorites/watch/watch-rules partial by design, with richer editing blocked on
   shared mutation contracts.
4. **Pulse 04 - Closeout.** Update wave, plan, and indexes.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused CLI/Web favorites/watch tests.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
