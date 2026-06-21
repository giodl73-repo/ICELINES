# Phase Blackhawks Inventory

## Purpose

Inventory the Playoff bracket/detail row before deciding whether it can be
promoted from partial to a bounded detail/export claim.

## Current Surface

| Item | Evidence | Blackhawks posture |
|---|---|---|
| CLI bracket | `icelines playoffs` | Existing text/JSON/CSV bracket command using bundled playoff data. |
| CLI series detail | `icelines playoffs --series A --season ...` | Bounded series momentum/detail command; not a live prediction surface. |
| TUI bracket/detail | `tui playoffs`, Enter to series detail | Series detail renders summary, game log, and compact non-tied margin sparkline. |
| Web bracket | `/playoffs`, `/api/v1/playoffs` | Existing bracket route and JSON envelope through `PlayoffsView`. |
| Markdown series export | `export md series` | Renders bundled playoff game-log rows from `PlayoffsView` plus inline game-margin SVG when supported. |

## Promotion Blockers

- Do not claim live playoff data or live recomputation.
- Do not claim exhaustive browser drilldown for every series beyond existing
  `/playoffs` and API bracket routes.
- Do not claim predictive playoff momentum, betting insight, or causal series
  analysis.
- Do not infer missing game logs; bundled data and explicit unavailable/error
  states remain the source boundary.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Run focused playoff CLI/TUI/Web/export tests. Result: passed;
   focused evidence supports a bounded `PlayoffsView` detail/export claim.
3. Matrix wording. Promote only to bounded `PlayoffsView` detail/export if
   evidence supports it. Result: passed; matrix promotes bounded detail/export
   with explicit non-claims.
4. Closeout. Result: passed; Phase Blackhawks closed.
