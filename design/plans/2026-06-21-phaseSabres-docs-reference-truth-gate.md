# Phase Sabres - Docs/reference truth gate

> Phase Sabres closes the docs/reference rollup placeholder with evidence-backed
> wording that keeps `/docs`, TUI docs, and CLI docs canonical while keeping the
> removed mkdocs/static-site surface out of active claims.

**Created:** 2026-06-21
**Status:** Closed - Phase Sabres complete

---

## Frame

Phase Islanders already tightened admin/docs truth, and later phases handled the
remaining admin rows. One docs/reference placeholder remains in the active
partial rollup: it says a future pulse should verify that wording does not
revive stale mkdocs/static-site or unimplemented operation claims.

Phase Sabres performs that pulse explicitly. It should not add a new docs
feature. It should prove the current docs/reference surfaces and replace the
placeholder with final wording.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Sabres Goal 1 - Docs inventory** | Docs/reference evidence spans CLI, TUI, Web, and supporting static artifacts. | A wave inventory names active surfaces and removed claims. |
| 2 | **Sabres Goal 2 - Evidence gate** | The matrix should be backed by current tests, not old cleanup notes. | Focused docs/reference tests pass. |
| 3 | **Sabres Goal 3 - Matrix wording** | The rollup should not contain a stale placeholder or imply mkdocs is active. | `design/specs/surface-parity.md` carries exact final wording. |
| 4 | **Sabres Goal 4 - Closeout** | The phase should leave no ambiguous docs/static-site status. | Closeout records final claims and non-claims. |

---

## Non-goals

- Do not reintroduce `icelines build`, `icelines deploy`, or `icelines site`.
- Do not reintroduce `/site/*`.
- Do not claim live mkdocs build/serve/deploy coverage.
- Do not rewrite historical design specs that describe older removed surfaces.
- Do not broaden docs/reference into a static-site publishing claim.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused docs/reference tests passed.
3. **Pulse 03 - Matrix wording.** Result: rollup and rows now preserve active
   docs/reference surfaces without reviving static-site claims.
4. **Pulse 04 - Closeout.** Result: Phase Sabres closed with exact
   docs/reference claims and static-site non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused docs/reference tests.
- No live mkdocs or network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
