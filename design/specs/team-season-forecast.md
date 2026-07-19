# Team Season Forecast — Specification

**Version**: 0.1  
**Date**: 2026-07-19  
**Status**: Draft  
**Plan**: [`../plans/2026-07-19-team-season-forecast.md`](../plans/2026-07-19-team-season-forecast.md)

---

## Purpose

Build a reusable NHL regular-season forecast engine. It predicts every
scheduled game once, simulates the complete league many times, explains why one
team is favored, and derives internally consistent records, standings, streaks,
playoff races, spoiler games, and uncertainty.

The New York Rangers and Seattle Kraken are the initial showcase teams, not
special cases. Any IceLines user can select teams, change model parameters, add
scenario events, choose a random seed, and reproduce a season forecast.

Season length, team membership, schedule boundaries, breaks, and deadline
metadata are inputs rather than constants. The same engine supports future
seasons, historical 82/84-game seasons, shortened seasons, and expansion-era
team counts without branching on a favorite year.

## 2026–27 Authority

The official NHL schedule contains 1,344 games: 84 games for each of 32 teams,
42 home and 42 road, from 2026-09-29 through 2027-04-10. The NHL All-Star break
is 2027-02-04 through 2027-02-07.

Schedule facts come from the official NHL schedule cache. Roster, injury,
goalie, transaction, and team-strength inputs retain their own source and
as-of timestamps. The working 2026–27 trade-deadline boundary is March 5, 2027,
as supplied by the product owner. Until an official NHL source is attached, the
output labels it as a user-provided scenario date rather than league authority.

## User Workflow

```powershell
# Forecast the whole league with reproducible defaults.
icelines icecast season --season 20262027 --trials 10000 --seed 20262027

# Focus rendered output on the Rangers and Kraken while still simulating all games.
icelines icecast season --team NYR --team SEA --trials 25000

# Show every game pick and its explanation.
icelines icecast season --team NYR --all-games

# Apply a user-authored scenario and parameter set.
icelines icecast season --scenario scenario.json --parameters model.json

# Machine-readable league forecast.
icelines icecast season --json --out forecast-20262027.json

# Replay a completed season with rolling, no-future-information forecasts.
icelines icereplay season --season 20252026 --trials 10000 --seed 20252026

# Inspect what the model knew before one historical date.
icelines icereplay season --season 20252026 --through 2026-01-15 --all-games
```

The branded command names are provisional until CLI collision and compatibility
review. They remain distinct from fantasy-roster `fantasy season-sim`; any
legacy/internal forecast verb is preserved as an alias if already public.

## Season Calendar Contract

Each season resolves a typed calendar containing participating teams, expected
game count where known, schedule start/end, named breaks, trade deadline and
authority, arena/venue version, and rules/tiebreak version. Schedule validation
uses that calendar rather than assuming 32 teams or 84 games.

The 2026–27 calendar is the first fixture. Historical calendars may represent
82-game, 56-game, lockout-shortened, pandemic-realigned, or expansion seasons.
Missing calendar metadata is explicit and individually overridable.

## Historical Replay Modes

Replay is not a disguised hindsight forecast:

- **rolling replay** processes historical games chronologically and rebuilds
  every pregame input from evidence available before puck drop;
- **season-start replay** freezes roster/strength evidence at opening day and
  simulates the entire known schedule;
- **counterfactual replay** applies an explicit user scenario to a historical
  season and is labeled hypothetical; and
- **actual comparison** joins predictions to final results only after each
  prediction is frozen, then reports calibration and misses.

Historical results may never enter features for the same game or an earlier
forecast. Output stores the input cutoff and source fingerprints for every
replayed game.

## Forecast Unit

One scheduled NHL game is the atomic forecast unit. Each game produces:

- home regulation-win probability;
- away regulation-win probability;
- overtime/shootout probability and conditional winner probability;
- expected standings points for each team;
- most likely result;
- confidence band and model uncertainty;
- factor contributions explaining the probability delta from neutral; and
- one shared sampled result per Monte Carlo trial.

Both teams consume the same sampled result. Team records are never simulated
independently.

## Pregame Strength Model

The neutral-site prior combines only information available at the forecast
`as_of` instant:

1. regressed team scoring and prevention strength;
2. current roster value and depth by position;
3. expected goalie/tandem value and starter uncertainty;
4. special-teams performance;
5. recent form with a bounded weight;
6. home advantage; and
7. opponent-adjusted historical evidence.

Raw goal differential and short winning streaks must not dominate the model.
Early-season estimates regress more heavily toward multi-season and roster
priors. As games accumulate, current-season evidence gains weight under a
disclosed schedule.

## Schedule Context

Every game derives deterministic context from the complete schedule:

- rest days for both teams;
- back-to-back status;
- three games in four nights and four in six;
- home-stand and road-trip game index;
- trip length, travel distance, and timezone displacement;
- return-home spot after a long trip;
- opponent rest advantage;
- days before and after the All-Star break;
- compressed post-break schedule; and
- divisional/conference importance.

Travel is calculated from arena coordinates and itinerary order, not merely
home/away flags. Penalties cannot double-count back-to-back, congestion, and
travel without separately disclosed components.

## Dynamic Season State

Each Monte Carlo trial advances chronologically and maintains:

- standings and tiebreak state;
- current bounded form/streak state;
- roster and injury availability scenarios;
- goalie workload and uncertainty;
- transaction and trade effective dates;
- mathematical elimination/qualification state; and
- playoff-hunt or spoiler context.

Hunt, spoiler, and streak adjustments are small, configurable, and capped.
They are hypotheses to stress, not narrative certainty or gambler's-fallacy
bonuses. A team becomes a spoiler only after the trial's standings state makes
that label valid.

## Point-in-Time People and Team State

A dated personnel ledger answers who belonged to each team at every forecast
instant. It tracks stable player identity plus roster membership, position,
line/depth role, expected availability, injury/return windows, goalie role and
workload, scratches, recalls/assignments, trades, signings, and coaching changes
when sourced.

Historical replay resolves the ledger exactly as it was known on that date. A
player traded in March cannot strengthen the acquiring team's January replay,
and today's roster cannot replace the actual opening-night roster. Forecast
mode samples uncertain availability and future role changes with provenance and
confidence rather than presenting them as news.

## Simulated NHL Trade Market

Forecast trials may enable a seeded hypothetical trade market:

```powershell
icelines icecast season --trade-mode plausible --trade-deadline 2027-03-05
icelines icereplay season --season 20252026 --trade-mode actual
icelines icereplay season --season 20252026 --trade-mode plausible --counterfactual
```

Trade modes are:

- `off`: rosters change only through explicit scenario events;
- `actual`: apply sourced completed trades at their effective timestamps;
- `plausible`: generate hypothetical trades from the trial's standings and
  personnel state; and
- `scenario`: apply only user-authored trade packages.

The plausible market reuses IceLines roster value, positional scarcity,
package-balance, injury, and schedule-fit concepts where valid. NHL-specific
constraints remain separate from fantasy trades: roster limits, salary/cap
space when available, contract term/control, draft assets, team competitive
window, buyer/seller status, positional need, and known movement protection.

Every generated trade must:

- be legal under the modeled constraints;
- remove and add every asset atomically with no duplicated player or pick;
- pass a configurable mutual-value/fairness threshold;
- occur no later than the configured deadline;
- affect only later games;
- carry a deterministic event ID and explanation; and
- be labeled simulated, never reported as an actual transaction.

Buyer/seller behavior emerges from each trial's standings, injuries, roster
window, and needs. Output reports trade distributions—probability of buying or
selling, players/assets moved, partner frequencies, and resulting record/odds
deltas—rather than pretending one generated deal is certain.

## Scenario Events

Scenario JSON may contain sourced or hypothetical events:

```json
{
  "as_of": "2026-09-28T12:00:00-04:00",
  "trade_deadline": "2027-03-05T15:00:00-05:00",
  "events": [
    {
      "effective_at": "2027-02-25T18:00:00Z",
      "team": "NYR",
      "kind": "strength_delta",
      "offense": 0.08,
      "defense": 0.03,
      "source": "user scenario",
      "detail": "deadline scoring addition"
    }
  ]
}
```

Supported event families should include player unavailable/return, goalie
availability, roster transaction, team-strength delta, and explicit parameter
override. Events affect only games after their effective timestamp.

## Parameter Contract

All non-learned operational weights are serializable and validated:

- home advantage;
- back-to-back penalty;
- congestion penalties;
- travel-distance and timezone penalties;
- long-trip and return-home effects;
- post-break rest/rust effect;
- recent-form half-life and cap;
- goalie uncertainty weight;
- injury/roster uncertainty;
- trade-impact decay or persistence;
- playoff-hunt and spoiler caps; and
- overtime base rate.

IceLines ships a named default parameter set. User overrides produce a new
parameter fingerprint in output and never silently replace defaults.

## Output Contract

`team_season_forecast.v1` contains:

- schedule authority and forecast `as_of`;
- model/parameter/scenario fingerprints;
- seed and trial count;
- 1,344 game forecasts;
- 32 team summaries;
- projected W-L-OTL, points, rank, and confidence intervals;
- division, conference, playoff, Presidents' Trophy, and last-place odds;
- longest win/loss streak distributions;
- probability each team leads the NHL in longest win streak;
- expected best/worst schedule stretches;
- highest-confidence picks and largest upset opportunities;
- pivotal hunt and spoiler games; and
- warnings for missing, stale, or scenario-based evidence.

`team_season_replay.v1` additionally contains actual results, prediction error,
calibration summaries, biggest correct upsets, worst misses, and the model's
projected standings after each historical date.

When trade simulation is enabled, both contracts include a trial-level
transaction journal and aggregate trade-market summary. Named-player outcomes
retain identity, effective time, source kind (`actual`, `scenario`, or
`simulated`), legality evidence, and downstream strength delta.

Focused text output may render only requested teams, but JSON retains the
league-wide simulation needed to prove consistency.

## “Why Will They Win?” Explanation

Every game explanation starts from a neutral 50% matchup and lists signed
probability contributions, for example:

```text
NYR 57% over SEA
+5.1 roster/depth strength
+3.0 home ice
+2.2 Seattle back-to-back
+1.4 Seattle road-trip game 5
-2.8 goalie uncertainty
-1.0 Rangers recent-form regression
```

Contributions must reconcile, within rounding tolerance, to the reported game
probability. Correlated features may use grouped attribution rather than
pretending to be independently causal.

## Calibration and Backtesting

Before calling the default model predictive, IceLines runs rolling-origin
backtests on completed seasons:

- train/tune only on dates before each evaluated game;
- report Brier score, multiclass log loss, calibration slope, and calibration
  bins;
- compare against home-only, standings-points, and Elo-style baselines;
- report performance by favorite strength, month, back-to-back, travel band,
  and pre/post-break segment; and
- retain losing or neutral ablations instead of hiding them.

No current or future game result, future roster, or later-season statistic may
leak into an earlier forecast.

## Acceptance Criteria

1. The official 2026–27 schedule validates as 1,344 unique games and exactly 84
   appearances per team, split 42 home/42 road.
2. Each trial samples one shared result per game and produces legal W-L-OTL and
   standings-point totals for both teams.
3. Identical input, parameters, seed, and trial count produce identical output.
4. Every game probability is finite, bounded, and normalized.
5. Factor attribution reconciles to the final probability within tolerance.
6. Back-to-back, rest, travel, trip index, and break context are derived from
   schedule chronology with fixture tests.
7. Scenario events cannot affect games before their effective time.
8. Hunt, spoiler, and streak effects are bounded and activate only from the
   trial state that justifies them.
9. Rangers/Kraken focused output is a filtered view of the same 32-team run.
10. Longest-streak leaders are distributions with probabilities, not a single
    unsupported deterministic claim.
11. Rolling-origin tests prove absence of future-result and future-roster
    leakage.
12. Missing roster, goalie, injury, travel, or deadline authority is explicit in
    warnings and uncertainty.
13. A season calendar, rather than hard-coded 2026–27 constants, controls team
    membership, game-count validation, breaks, deadline, and rules.
14. Rolling replay freezes every game forecast before joining its actual result
    and produces identical output from identical historical snapshots.
15. Historical team-count, season-length, relocation, and schedule-format
    fixtures run without 32-team or 84-game assumptions.
16. Point-in-time personnel resolution prevents a later roster, injury return,
    goalie role, coach, or transaction from leaking into an earlier game.
17. Simulated trades are seeded, atomic, deadline-bounded, constraint-checked,
    mutually valued, and explicitly labeled hypothetical.
18. Historical replay uses actual trades by default; plausible trades require
    explicit counterfactual mode and never overwrite the actual transaction
    journal.

## Non-Goals for v1

- betting advice, guaranteed winners, or sportsbook line replication;
- fabricated trade, injury, or starting-goalie news;
- autonomous roster or transaction mutation;
- play-by-play score simulation; and
- claiming motivation, streaks, or spoiler status as proven causal effects.
