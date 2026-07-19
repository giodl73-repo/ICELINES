# Team Season Forecast — Implementation Plan

**Date**: 2026-07-19  
**Status**: Draft  
**Spec**: [`../specs/team-season-forecast.md`](../specs/team-season-forecast.md)

---

## Outcome

Ship a reproducible IceLines command that forecasts all 1,344 games in the
2026–27 NHL season, simulates consistent outcomes for all 32 teams, explains
each matchup, and reports standings, playoff odds, streak leaders, pivotal
games, and uncertainty. Rangers and Kraken reports are the first acceptance
showcase.

The implementation is season-generic. The 2026–27 schedule is its first
acceptance fixture, and completed seasons can be replayed chronologically
without future-information leakage.

## Architecture Boundary

- `icelines-fetch`: official schedule and source-stamped input adapters.
- `icelines-core`: schedule context, game probability, chronological Monte
  Carlo state, standings, streaks, attribution, and typed ViewModels.
- `icelines-cli`: parameter/scenario parsing and text/JSON rendering.
- future TUI/Web work consumes the same ViewModel after CLI/core acceptance.

Do not couple this engine to fantasy `season-sim`; shared low-level schedule or
Monte Carlo helpers may be extracted only when their semantics truly match.

## Milestone 1 — Schedule Authority and Invariants

- Define a season-calendar contract for team membership, expected game counts,
  date bounds, named breaks, deadline authority, venues, and rules/tiebreaks.
- Load the complete official season schedule through the existing cache path.
- Normalize game ID, timestamp, home/away team, venue, and status.
- Add arena coordinates and itinerary distance calculation.
- Derive rest, back-to-back, congestion, road-trip, home-stand, timezone, and
  All-Star break context.
- Fence 2026–27 with 1,344 unique games, 84 per team, and 42/42 home/road.

Exit: `team_schedule_context.v1` is deterministic and fixture-tested.

## Milestone 1B — Historical Calendar and Replay Authority

- Load historical schedules, results, rosters, transactions, and stats through
  their original as-of boundaries.
- Support season-start, rolling, and explicitly counterfactual replay modes.
- Freeze each rolling pregame forecast before actual-result comparison.
- Add non-32-team, non-84-game, shortened-season, and expansion fixtures.

Exit: `team_season_replay.v1` can reproduce a completed season without future
leakage and score the model against actual outcomes.

## Milestone 2 — Pregame Team Strength

- Define source-stamped offense, defense, special-teams, roster/depth, and
  goalie components.
- Add early-season regression and time-decayed current-season evidence.
- Produce a neutral matchup prior with disclosed uncertainty.
- Establish simple home-only, standings, and Elo-style comparison baselines.

Exit: every scheduled game has a no-leak pregame strength snapshot.

## Milestone 2B — Point-in-Time Personnel Ledger

- Create dated player/team membership and role intervals from rosters,
  transactions, recalls, injuries/returns, goalie roles, and supported coaching
  changes.
- Resolve every game against the latest evidence strictly before puck drop.
- Preserve stable player identity through trades and assignments.
- Add leakage tests for deadline acquisitions, injury returns, goalie changes,
  and current-roster contamination of historical replays.

Exit: any forecast date can explain exactly which people formed each team input.

## Milestone 3 — Game Probability and Explanation

- Model home-regulation, away-regulation, and overtime outcome probabilities.
- Apply schedule context with bounded configurable weights.
- Attribute signed probability deltas to grouped factors.
- Validate normalization, monotonicity, symmetry, and attribution
  reconciliation.

Exit: `team_game_forecast.v1` answers “who is favored and why?” for any game.

## Milestone 4 — Chronological League Simulator

- Advance all games in timestamp/game-ID order.
- Sample one shared result per game per trial.
- Maintain legal W-L-OTL, standings points, tiebreak state, streaks, goalie
  workload, and bounded form.
- Apply scenario injuries, returns, transactions, and strength changes only
  after their effective time.
- Derive hunt, qualification, elimination, and spoiler context from each
  trial's state.

Exit: seeded runs are reproducible and internally consistent across 32 teams.

## Milestone 4B — Simulated Trade Market

- Add `off`, `actual`, `plausible`, and `scenario` trade modes.
- Derive trial-specific buyers, sellers, needs, and competitive windows.
- Generate named-player/draft-asset packages from roster value, scarcity,
  contract/control, cap/roster constraints where available, and mutual fairness.
- Apply accepted packages atomically before the configured deadline and update
  all later personnel/strength snapshots.
- Aggregate buy/sell, partner, player, asset, and standings-impact
  distributions while retaining the trial transaction journal.

Exit: plausible trades are deterministic under seed, legal under modeled
constraints, clearly hypothetical, and measurable against a no-trade baseline.

## Milestone 5 — League Summary and Streak Products

- Aggregate expected records and percentile intervals.
- Compute division/conference/playoff and trophy odds.
- Compute longest win/loss streak distributions and league-leader odds.
- Identify highest-confidence picks, upset candidates, hardest trips, best and
  worst stretches, hunt games, and spoiler games.
- Confirm focused team output is only a filter over the league run.

Exit: `team_season_forecast.v1` contains all league and team products.

## Milestone 6 — CLI and Scenario Files

- Add provisional branded `icelines icecast season` command.
- Add provisional branded `icelines icereplay season` command with `--through` and
  rolling/season-start/counterfactual modes.
- Support `--season`, repeatable `--team`, `--as-of`, `--trials`, `--seed`,
  `--parameters`, `--scenario`, `--all-games`, `--json`, and `--out`.
- Validate unknown teams, malformed dates, impossible probabilities, negative
  weights, events outside the season, and unofficial/missing deadline inputs.
- Default the working 2026–27 deadline boundary to 2027-03-05 at 3 p.m. ET and
  label it user-provided until official NHL authority is attached.
- Render a concise league overview and detailed Rangers/Kraken showcase.

Exit: users can reproduce and alter the forecast without code changes.

## Milestone 7 — Calibration and Stress Testing

- Add rolling-origin completed-season backtests.
- Report Brier score, multiclass log loss, calibration bins/slope, and baseline
  deltas.
- Run parameter ablations for rest, travel, form, goalie, hunt, spoiler, break,
  and trade effects.
- Stress missing goalies, stale rosters, major injuries, deadline deltas,
  compressed schedules, and extreme user parameters.
- Prove no future-result, future-stat, or future-roster leakage.

Exit: default parameters are evidence-backed or explicitly labeled heuristic.

## Milestone 8 — Rangers and Kraken Release Showcase

- Publish preseason Rangers and Kraken game-by-game forecasts from the same
  league run.
- Explain projected record range, best/worst stretches, travel risks,
  back-to-backs, post-break congestion, deadline sensitivity, playoff odds,
  and longest streak distribution.
- Include at least three scenarios: baseline, injury downside, and deadline
  upgrade.
- Retain seed, parameters, scenario, as-of time, and data fingerprints so every
  published result is reproducible.

Exit: the showcase can be regenerated by any user command with identical data.

## Test Matrix

- Schedule: uniqueness, counts, chronology, home/road, trip segmentation.
- Probability: bounds, sum-to-one, symmetry, monotonic strength, neutral knobs.
- Fatigue: back-to-back, 3-in-4, travel, timezone, return-home, no double count.
- Break/deadline: boundary instants and post-effective-only changes.
- Simulation: seed determinism, one shared result, points conservation, legal
  records, streak calculations, standings/tiebreak ordering.
- Dynamic state: capped form, hunt activation, elimination, spoiler activation.
- Personnel: point-in-time roster/role identity, injuries/returns, goalie roles,
  coaching changes, and transaction-boundary leakage.
- Trades: atomicity, no duplicate assets, deadline boundary, mutual-value gate,
  roster/cap legality, seeded determinism, and actual-vs-counterfactual policy.
- Authority: stale/missing evidence warnings and as-of leakage fences.
- Replay: prediction freeze before result join, historical calendar variants,
  relocation/team membership, and deterministic rolling checkpoints.
- Output: 32 teams, 1,344 games, focused-filter parity, JSON round trip.
- Calibration: historical rolling-origin fixtures and baseline comparison.

## First Implementation Slice

Build only deterministic schedule context and a baseline game forecast:

1. validate the 2026–27 1,344-game schedule;
2. derive rest, back-to-back, congestion, trip index, distance, timezone, and
   break features;
3. combine existing team strength with home advantage into normalized outcome
   probabilities;
4. render Rangers and Kraken game tables with factor explanations; and
5. add fixture tests before Monte Carlo state or narrative hunt/spoiler effects.

This creates useful game picks immediately while establishing the trustworthy
inputs required by the full season simulator.
