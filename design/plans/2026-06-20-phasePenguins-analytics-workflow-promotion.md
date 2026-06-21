# Phase Penguins - Analytics workflow promotion gate

> Phase Penguins decides whether WP-009 analytics-cache Web/API first-route
> evidence can become broader workflow claims, or whether each family remains a
> bounded prepared-cache consumer.

**Created:** 2026-06-20
**Status:** Active - pulse 01 planning opened

---

## Frame

WP-009 shipped a strong analytics-cache foundation: typed records, strict store
and invalidation behavior, `AnalyticsCacheConsumerView`, and representative
Web/API routes for named reports, coach dashboard, opponent scout, player
evidence card, line explorer, goalie readiness, practice focus, postgame review,
postgame adjustments, and agent evidence.

The active surface matrix still marks those rows partial because first-route
evidence is not the same as a finished coaching, scouting, player research,
line deployment, goalie, practice, postgame, or agent workflow. Phase Penguins
should audit those claims and promote only where workflow evidence and product
copy are strong enough.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Penguins Goal 1 - WP-009 workflow inventory** | Existing evidence is spread across the WP-009 wave, route tests, and the surface matrix. | A wave inventory lists each cache consumer family, first-route evidence, non-claims, and promotion blockers. |
| 2 | **Penguins Goal 2 - Promotion lane selection** | Not every cache consumer family should move at once. | The phase picks one workflow family for promotion evidence or records why all remain bounded first-route consumers. |
| 3 | **Penguins Goal 3 - Product-copy gate** | Cache routes must not imply live recomputation, prediction accuracy, deployment advice, injury certainty, causal blame, mandatory practice, or autonomous agent action. | Accepted or deferred families have explicit copy/non-claim wording and focused tests/docs. |
| 4 | **Penguins Goal 4 - Workflow evidence gate** | A workflow claim needs more than rendering one prepared cache record. | Accepted families have repeatable evidence proving the user journey, unavailable states, and no recomputation/source mutation. |
| 5 | **Penguins Goal 5 - Surface matrix closeout** | The matrix should distinguish promoted workflows from first-route evidence. | `design/specs/surface-parity.md` carries exact final wording for each affected family. |

---

## Non-goals

- Do not add new analytics formulas or live recomputation.
- Do not weaken the prepared-cache contract or allow GET routes to create cache
  storage.
- Do not claim autonomous coaching, scouting, deployment, goalie start/sit,
  mandatory practice, postgame causal blame, automatic correction, or agent
  action authority.
- Do not promote every WP-009 route family at once without family-specific
  evidence.
- Do not reopen Phase Capitals Signals cache decisions.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Record route families, tests, first-route
   evidence, non-claims, and blockers.
2. **Pulse 02 - Promotion lane selection.** Choose a single family to test for
   workflow promotion or decide all families remain bounded.
3. **Pulse 03 - Product-copy gate.** Verify or update copy for the selected
   family, including explicit non-claims.
4. **Pulse 04 - Workflow evidence gate.** Add or run focused evidence for the
   selected family, or record durable deferral.
5. **Pulse 05 - Closeout.** Update the wave, plan, and surface matrix.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Route or behavior changes run focused `icelines-web` L2 analytics-cache tests.
- Tests stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
