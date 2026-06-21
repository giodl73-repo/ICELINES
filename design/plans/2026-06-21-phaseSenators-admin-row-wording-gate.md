# Phase Senators - Admin row wording gate

> Phase Senators records the individual admin operation rows as scoped partials
> by design, using the safety boundary already closed by Phase Flyers.

**Created:** 2026-06-21
**Status:** Active - evidence gate passed

---

## Frame

Phase Flyers closed the admin operation safety gate. The rollup row already
states the intended boundary: runtime active-season config, data verify,
snapshot activate/delete, and game-cache warmers are safe POST-backed web admin
operations; web data install/remove remain deferred and unmounted; persistent
report-toggle writes remain a CLI/TUI durable config handoff.

The remaining issue is precision. The individual admin rows still begin with
plain `partial -` wording. Phase Senators tightens those rows so future readers
can tell intentional durable deferrals from unresolved admin drift.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Senators Goal 1 - Admin row inventory** | The row-level wording should match the Flyers closeout. | A wave inventory names implemented safe operations and durable deferrals. |
| 2 | **Senators Goal 2 - Evidence gate** | Wording changes need current route proof, not memory. | Focused `l1_admin_` route tests pass. |
| 3 | **Senators Goal 3 - Partial-by-design wording** | Plain partial wording hides intentional safety decisions. | The three individual admin rows say partial by design and preserve exact blockers. |
| 4 | **Senators Goal 4 - Closeout** | The surface matrix should carry the final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not add web data install/remove.
- Do not add persistent web report-toggle writes.
- Do not promote runtime web config to durable user config.
- Do not add GET-backed admin mutations.
- Do not broaden game-cache warmers into release bundle install/remove.
- Do not claim full admin parity.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused admin route tests passed and
   support scoped safe-operation wording.
3. **Pulse 03 - Matrix wording.** Convert admin rows to explicit partial by
   design wording only if evidence passes.
4. **Pulse 04 - Closeout.** Close Phase Senators with exact claims and
   non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused admin route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
