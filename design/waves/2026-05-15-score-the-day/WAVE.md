---
wave: score-the-day
date_open: 2026-05-15
status: active
source: Tier 3 backlog - Fantasy daily delta scoring
---

# Score the Day

## Mission

Add fantasy daily delta scoring: a dated view of fantasy points earned by each
fantasy roster from cached, finalized game data. The wave must reuse shared
fantasy scoring contracts instead of adding surface-local math, and it must stay
offline-testable.

## Award Fit

This is a Lady Byng / Selke product-utility wave: it turns existing fantasy
league and boxscore rails into a clean daily review loop without overreaching
into proprietary projections or live external integrations.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Scoring contract | Define a shared daily-delta ViewModel and row schema in core. | Invent a second fantasy scoring formula. |
| Data path | Build daily rows from cached finalized boxscore/game-line data and local FantasyDb rosters. | Fetch live NHL data during scoring or tests. |
| Surfaces | Add discoverable CLI/web/TUI handoff surfaces after the shared contract exists. | Add main-dashboard write mutations. |
| Closeout | Document commands, data requirements, and gates. | Convert all fantasy league management into web mutations. |

## Operating Rules

- Daily fantasy points are descriptive: points earned on a date from loaded
  game lines, not a prediction.
- Use `Scheme`, `compute_fantasy_score`, `compute_goalie_fantasy_score`, or a
  shared daily-stat adapter; do not fork category weights in CLI/TUI/web.
- Read from cached/finalized game data only. Missing game data must surface as
  source state or warnings, not zero-shaped success.
- Preserve local SQLite league/team/roster ownership in FantasyDb.
- Do not add live-network tests.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Daily delta inventory and pulse map | complete | `FANTASY-DAILY-DELTA-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Core daily-delta ViewModel and scoring adapter | planned | depends on Pulse 01 |
| 03 - Cached boxscore/FantasyDb data path | planned | depends on Pulse 02 |
| 04 - CLI, web, and TUI read surfaces | planned | depends on Pulse 03 |
| 05 - Docs, regression gates, and closeout | planned | depends on Pulses 02-04 |

## Role Notes

- **pace**: every daily scoring formula and rounding/tiebreak rule must be
  explicit and descriptive.
- **bench**: fixture-driven tests are required for final/live/missing games and
  skater/goalie rows; no live NHL calls.
- **wire**: missing cache and unfinalized boxscores must be explicit source
  states, not silently zeroed.
- **forge**: keep scoring in `icelines-core`, cache/SQLite reads in
  `icelines-fetch`/CLI adapters, and surfaces thin.

## Current Result

Pulse 01 opened the wave, mapped the existing fantasy scoring/projection
contracts, and split the feature into core contract, cached data path, surface,
and closeout pulses.

## Next

Execute Pulse 02: core daily-delta ViewModel and scoring adapter.
