# Phase Coyotes - Docs route wording gate

> Phase Coyotes records the Web `/docs` route row with precise scoped wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Coyotes complete

---

## Frame

Phase Sabres already closed the docs/reference truth gate across CLI docs, the
TUI docs overlay, Web `/docs`, and dashboard/menu docs handoffs. The remaining
cleanup is route-row precision: the `GET /docs` row still begins with terse
`done` wording even though it already names `COMMANDS.md`, `DocsView`, and the
removed `/site/*` boundary.

Phase Coyotes tightens that individual route row without changing docs runtime
behavior or reopening static-site publishing claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Coyotes Goal 1 - Route inventory** | The Web docs row should match the Sabres docs truth gate. | A wave inventory names the route row, evidence, and non-claims. |
| 2 | **Coyotes Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused `/docs` route test passes. |
| 3 | **Coyotes Goal 3 - Scoped route wording** | Terse `done` wording hides the `DocsView` and `/site/*` boundary. | The route row names embedded `COMMANDS.md`, `DocsView`, dashboard/menu handoffs, and removed static-site non-claims. |
| 4 | **Coyotes Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change docs rendering behavior.
- Do not reintroduce `/site/*`.
- Do not reintroduce removed mkdocs build/serve/deploy claims.
- Do not broaden `/docs` into a static-site publishing route.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused `/docs` route test passed.
3. **Pulse 03 - Matrix wording.** Result: route row now carries scoped docs
   route wording.
4. **Pulse 04 - Closeout.** Result: Phase Coyotes is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Coyotes closed the `/docs` route wording gate. `GET /docs` now records
the embedded `COMMANDS.md` and `DocsView` contract, the career fetch
instruction evidence, dashboard/menu handoffs, and the explicit removed
static-site boundary.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run the focused `/docs` Web route test.
- No live mkdocs or network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
