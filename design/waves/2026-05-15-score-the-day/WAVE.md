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
| 02 - Core daily-delta ViewModel and scoring adapter | complete | `icelines-core/src/view_model/fantasy_daily.rs`; `icelines-core/src/view_model/mod.rs`; `icelines-core/src/lib.rs`; `plans/pulse-02.md` |
| 03 - Cached boxscore/FantasyDb data path | complete | `icelines-fetch/src/fantasy_daily.rs`; `icelines-fetch/src/lib.rs`; `plans/pulse-03.md` |
| 04 - CLI, web, and TUI read surfaces | complete | `icelines-cli/src/commands/fantasy.rs`; `icelines-web/src/handlers/fantasy.rs`; `icelines-cli/src/tui/command.rs`; `plans/pulse-04.md` |
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

Pulse 04 added thin read surfaces over that shared data path: CLI
`fantasy daily --date`, JSON `/api/v1/fantasy/daily?date=...`, and TUI/web
command handoffs. Missing cache remains an explicit warning/source-state, not a
zero-shaped success.

## Next

Execute Pulse 05: docs, regression gates, and closeout.
