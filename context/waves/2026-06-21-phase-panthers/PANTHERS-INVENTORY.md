# Phase Panthers Inventory

## Purpose

Inventory the Player Signals surface row before converting its plain partial
wording into partial-by-design wording.

## Current Surface

| Area | Evidence | Panthers posture |
|---|---|---|
| Player Signals Web HTML | `/player/:id/signals` | Keep direct `PlayerSignalsView` inspection with unavailable evidence and non-claim copy. |
| Player Signals Web JSON | `/api/v1/player/:id/signals` | Keep the Web JSON projection of `PlayerSignalsView`; missing evidence remains unavailable/null, never zero-filled. |
| CLI/TUI/Markdown Signals | `signals`, TUI player-card handoff, `export md signals` | Preserve Hurricane direct inspection surfaces and disclosure copy. |
| Signals roster | `signals-roster --team <ABBR>` | Keep Rangers team-scoped inspection matrix, not public ranking or cache publication. |
| Future promotions | analytics cache, `StatId`, filters, catalog sorting, leaderboards | Keep deferred by Capitals until source-state, cache-key, invalidation, methodology, and bounded ranking-copy contracts exist. |

## Risks to Avoid

- Rewording Signals as cache-backed, catalog-backed, filterable, or leaderboard-ready.
- Claiming Signals are predictions, betting edges, injury signals, deployment
  recommendations, player grades, or coaching recommendations.
- Zero-filling missing Signal inputs.
- Weakening the Capitals future-promotion prerequisites.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Signals Web route tests support
   partial-by-design surface-row wording.
3. Matrix wording. Convert the Player Signals row to partial-by-design wording
   if evidence passes.
4. Closeout. Record final claims and non-claims.
