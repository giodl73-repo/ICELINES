---
wave: sim-the-spark
date_open: 2026-05-15
status: active
source: Measure the Finish closeout and the Phase Rocket Richard roadmap
---

# Sim the Spark

## Mission

Turn Rocket Richard scoring intelligence into descriptive pace and outlook
surfaces that explain what a player or team is tracking toward from already
loaded IceLines data. Keep the work explicitly non-betting: pace, range, source
state, and assumptions are allowed; odds, win probability, and proprietary model
claims are not.

## Award Fit

The Rocket Richard Trophy rewards goal scoring. The previous Rocket waves
proved official scoring-event inputs, player/team scoring profiles, finishing
trend rows, inside-shot proxy buckets, and shot-streak leaderboards. This wave
asks the next user question: "if this scoring rate keeps going, what does the
finish look like, and how much should I trust that read?"

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Pace contracts | Define reusable ViewModel rows for goal/shot/point pace from current season totals and recent scoring trends. | Build a betting forecast or win-probability model. |
| Player outlook | Surface descriptive rest-of-season / 82-game scoring pace with sample-size and source-state flags. | Hide low-GP uncertainty behind a single confident number. |
| Team outlook | Summarize goal-for/goal-against pace and recent scoring trend direction from loaded schedule/score data. | Replace standings, playoff odds, or quality-ledger work. |
| Surface expansion | Start in core and web/API, then add CLI/TUI only where the ViewModel is already stable. | Put projection math in templates, route handlers, or command formatting. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Projection inventory and assumptions contract | done | `SPARK-INVENTORY.md`; `plans/pulse-01.md` |
| 02 - Player scoring pace ViewModel | done | `PlayerScoringPaceView`; `plans/pulse-02.md` |
| 03 - Team scoring outlook ViewModel | done | `TeamScoringOutlookView`; `plans/pulse-03.md` |
| 04 - Surface parity and docs | planned | `plans/pulse-04.md`; depends on Pulses 02-03 |
| 05 - Wave closeout | planned | depends on Pulse 04 |

## Role Notes

- **pace**: every formula, threshold, tiebreaker, and confidence label must be
  stated as an assumption and tested with known values.
- **scout**: copy must read as hockey context, not certainty. A heater, soft
  deployment, or low-GP sample needs plain-language caveats.
- **wire**: use loaded stats, schedule, boxscore, and play-by-play caches only.
  GET surfaces must not fetch live network data.
- **bench**: test exact threshold behavior, low-GP suppression, zero-shot
  conversion, missing-source disclosure, and cross-surface JSON contracts.

## Current Result

Wave opened after Measure the Finish closed and CI passed for Pulse 05. Pulse 01
inventoried existing pace/projection code, scoring trend inputs, team season
inputs, cache/source-state constraints, and required tests before implementation.
Pulse 02 added core-owned `PlayerScoringPaceView` rows for goals, points, and
shots from already-loaded `PlayerView` totals. Pace and projected-finish values
stay nullable below `MIN_GP` or when remaining games are unavailable.
Pulse 03 added core-owned `TeamScoringOutlookView` rows for goals for and goals
against from caller-supplied regular-season schedule/score inputs. Projected
finish stays nullable without a remaining-game count, and missing/partial source
states are explicit in the ViewModel context.

## Next

Execute Pulse 04: surface parity and docs.
