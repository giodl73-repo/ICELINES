# Team Season Forecast — Specification

**Version**: 0.1  
**Date**: 2026-07-19  
**Status**: Implemented — forecast, scenarios, playoffs, rolling/as-of replay, and calibration
**Owner domain**: IceCast season forecast and replay
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
# Forecast the whole league with reproducible baseline defaults.
icelines icecast season --season 20262027 --json

# Focus rendered output on the Rangers and Kraken while retaining all league games.
icelines icecast season --team NYR --team SEA --all-games

# Reproduce a larger chronological league simulation.
icelines icecast season --trials 25000 --seed 20262027

# Show every game pick and its explanation.
icelines icecast season --team NYR --all-games

icelines icecast season --scenario scenario.json --team NYR

# Machine-readable league forecast.
icelines icecast season --json --out forecast-20262027.json

icelines icecast season --season 20242025 --replay-mode rolling --trials 1000 --seed 20242025

# Inspect what the model knew before one historical date.
icelines icecast season --season 20242025 --replay-mode rolling --through 2025-01-31 --trials 1000 --seed 20242025

# Attribute each historical counterfactual from the same fixed boundary.
icelines icecast season --season 20242025 --replay-mode rolling --through 2025-01-31 --scenario historical-counterfactual.json --isolated-impacts --json
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

Point-in-time replay fixes every final result through `--through` in each
trial and samples only the later schedule. Before rolling strengths are built,
all later scores, endings, and final-state labels are removed. Dated personnel
evidence is likewise limited to the cutoff. The season document records the
typed `as_of_date` and `replay_checkpoint`. The checkpoint contains league
completed/remaining games and each team's actual GP, W-L-OTL, points, and
remaining games plus expected remaining W-L-OTL and points. Observed plus
expected remainder reconciles to the projected final averages; text and card
renderers consume those fields without recalculation.
Core rejects missing results at or before the boundary and any result label
after it. When isolated attribution is requested, its
baseline, naturally sampled, forced single-event, and forced-ceiling runs all
reuse that boundary with identical trials and seed.

When puck timestamps are unavailable, the safe cutoff is the calendar date:
all games on that date are forecast as a batch, and none may observe another
result from the same date. A results-only rolling replay must begin from a
neutral disclosed prior rather than contaminate history with a present-day
roster snapshot.

Opening-roster authority is a separate, machine-readable gate. A qualifying
snapshot must be sealed, match the replay season, predate the first scheduled
game's calendar date, pass stored integrity verification, and contain a
non-empty roster for every team in the schedule. When exact game timestamps
are unavailable, opening-day snapshots are ineligible. Output retains the
latest rejected snapshot and reason so unavailable authority is distinguishable
from missing data or failed validation.

After the gate passes, opening team strength uses prior-season regressed player
values over 12 forward, six defense, and two goalie slots, weighted 55/30/15.
Missing histories are neutral, and roster-wide value coverage regresses the
aggregate edge toward 50. Current-season results replace this opening evidence
through the rolling prior; no replay-season player total is admissible.

The authoritative opening row retains exact stable player membership,
position group, modeled value, and selected slot. Active lineup strength is
recomputed after each later recall, assignment, IR placement, or activation.
Events dated on or before the roster snapshot are already reflected and must
not be applied again. Missing active slots are neutral 50, and every game
records its signed personnel-strength delta from the opening roster.

A post-snapshot newcomer absent from opening membership may join the modeled
roster only through stable identity plus a completed prior-season position
group and value. Missing value or group is unknown and has neutral strength
impact; it is not permission to infer from replay-season totals.

Dated transaction prose is admissible evidence only after its operational
date. The raw event, stable source ID, classification, and provenance must be
retained even when its player or direction cannot be resolved. Mixed events
must not be collapsed into a guessed net roster effect; availability or
strength changes require an unambiguous direction and, for player-weighted
effects, stable player identity.

Identity resolution may consult immutable player IDs and canonical names from
a later-built catalog, but must not copy later performance, team stint, role,
or availability fields into the earlier forecast. Duplicate normalized names
remain ambiguous unless contemporaneous evidence disambiguates them.

Action direction is player-specific, not transaction-row-wide. In a mixed row,
each named player resolves against the nearest applicable clause. Waiver
placement, IR placement/activation, and contract extensions do not themselves
prove an NHL active-roster transition. Recall and waiver-claim language may
open an active-roster interval; assignment language may close one. Acquisition,
trade, and release language changes organization context but is not sufficient
by itself to assert active-roster status.

A real-team trade may change active membership only when exactly one
`traded_away` link and exactly one `acquired` link share the same effective
date and stable player ID across two different teams. The pair is one atomic
transfer: the source must already be known active, the destination addition
and source removal occur together, and any active IR state follows the player.
Unpaired, ambiguous, duplicate, or unknown-source trades remain organization
evidence with no lineup-strength effect.

An assignment without an earlier observed addition creates an
`implied_preexisting` interval with an unknown start. Repeated additions or
removals are structured anomalies and must never create overlapping intervals.
The same idempotency applies to game-state counters and IR state: duplicate
source rows remain evidence but cannot transition a player twice.
Neither interval metadata nor prior value may alter a forecast until dated
opening membership and coverage are adequate for that game.

Player value used at a replay boundary may derive from completed seasons before
that boundary, with disclosed regression, but never from replay-season totals.
Missing prior history is unknown—not zero and not permission to backfill later
performance. A resolved prior value does not affect strength until the player's
membership/availability interval is valid for that game.

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

Completed official results are downstream evaluation labels. A ledger row may
add the final score, winner, REG/OT/SO ending, hit/miss, and Brier score only
after its probability has been computed. Pending games remain ungraded, and a
historical evaluation using present-day strength inputs must not be labeled a
point-in-time replay.

Calibration evaluates both the binary game winner and the native three-way
pregame outcome: home regulation win, away regulation win, or overtime/shootout.
The model reports binary Brier and log loss, three-way log loss, explicit skill
deltas against 50/50 and equal-three-outcome baselines, and home-win probability
bins with expected calibration error. Missing REG/OT/SO ending metadata makes
only the three-way score unavailable; it must not be inferred from score margin.
With at least 20 graded binary outcomes spanning home wins and losses, core fits
logistic calibration intercept and slope by regressing the outcome on forecast
home-win log odds. Ideal values are intercept 0 and slope 1. The numerical fit
is a retrospective diagnostic, is omitted for undersized or single-class
samples, and must never alter the probabilities being evaluated.
Core also retains parameter standard errors and approximate 95% Wald intervals
using the inverse fitted information matrix. These intervals describe sampling
uncertainty under the logistic recalibration model; they do not correct for
season-to-season dependence or authorize in-sample probability rewriting.

Model-family comparisons must consume the same graded games and obey the same
exclusive evidence boundary. The minimum rolling comparison set is a home-only
prior, points-only regressed standings, and chronological Elo. The standings
baseline must exclude goal differential, roster, schedule, and personnel
features. Elo ratings initialize uniformly, update only from completed earlier
dates, treat all games on one date as a frozen batch, and disclose home
advantage, K-factor, and OT/SO result credit. Signed improvement is baseline
loss minus IceLines loss, so a negative value explicitly records a baseline
win.

Factor ablation removes one reconciled probability contribution from frozen
forecasts and scores the resulting counterfactual over the identical completed
games. It must not refit parameters or use outcomes to alter probabilities.
Every row reports activation count, mean absolute probability movement, and
signed Brier/log-loss improvement. Zero or negative contributions remain
visible; an ablation result is diagnostic evidence, not automatic permission
to change a production weight.

Franchise relocation must preserve the team identity, alignment, arena, and
timezone carried by the replay season. Historical Arizona schedules use `ARI`
and their contemporary arena context; modern Utah schedules use `UTA`. Current
franchise naming must not reject or silently relocate historical games.

An Elo blend sweep may combine two already-frozen, leakage-safe probabilities
over a fixed grid. It must report every tested weight and select its historical
minimum by a declared proper scoring rule, with deterministic tie-breaking.
The winning historical weight is evidence, not permission to mutate production
defaults; promotion requires multi-season validation and an explicit model
version change.

A historical game ledger does not itself authorize a historical season
simulation. When division membership, qualification, reseeding, or bracket
rules differ from the supported calendar, the simulator must refuse the season
explicitly. It must never apply current playoff alignment merely because every
team abbreviation is recognized.

Cross-season validation accepts three or more immutable graded-season
artifacts with a common blend grid. It pools metrics by game count and performs
leave-one-season-out selection, choosing each holdout's weight solely from the
other inputs. The artifact must preserve training-season identity and signed
comparisons against both unblended IceLines and pure Elo, and reject duplicate
seasons or incompatible grids.
Cross-season validation also carries one binary calibration observation per
graded game. In a separate rolling-origin sequence, the second and later
seasons are scored using intercept/slope fitted exclusively on earlier supplied
seasons. Each row retains training seasons/game count, frozen coefficients,
uncalibrated and recalibrated Brier/log loss, and signed improvement. Future
seasons must never enter a chronological calibration fit. The validation view
must also expose a core-owned summary that pools holdout losses by game count
and counts improved holdouts; surfaces must not average per-season scores or
recompute calibration evidence independently. The summary carries paired
per-game standard errors and 95% normal-approximation intervals for both loss
improvements. It must separately carry delete-one-holdout-season jackknife
intervals for season-clustered variation, disclosed as conditional on the
fitted chronological sequence, unstable with few holdouts, and not inclusive
of model-selection uncertainty. Additive summary fields in the v1 artifact must
deserialize with neutral defaults so sealed pre-interval validation JSON remains
readable. A core-owned evidence label must remain `insufficient_holdouts` below
four holdout seasons—the number implied by the five-season promotion gate—and
then classify the clustered interval as `positive`, `negative`, or
`inconclusive` relative to zero.

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

For teams named in a probabilistic scenario, IceCast emits
`scenario_outcomes` grouped by the number of sampled positive and negative
events. Each bucket includes its trial count/probability, average sampled
strength delta, average points, playoff probability, and Stanley Cup
probability. Events without a shared `correlation_key` use independently mixed
seeded draws even when their IDs share long prefixes; explicitly correlated
events continue to share one occurrence decision.

Training-camp outcomes use `opening_roster_policies`, not independent events.
Each team policy contains mutually exclusive choices whose probabilities must
sum to 1.0. IceCast makes one seeded draw at the start of each trial, applies
that choice's strength delta for the full season and playoffs, and emits
`opening_roster_summaries` with configured probability, sampled probability,
strength delta, and stable roster IDs. A residual choice may have no roster IDs
when retained camp branches do not cover the complete outcome distribution.
Opening-roster cumulative probabilities are built once per forecast and sampled
with binary search. This keeps high-coverage camp policies practical for many
teams and large season-trial counts.

Historical player-event calibration is produced by
`icecast calibrate-development`. Each label uses season `t` and the immediately
following completed season `t+1`; no later record may enter the transition.
The default workload gates are 20 target-season games for skaters and 15 for
goalies. The 2012-13 lockout and 2019-20/2020-21 pandemic seasons, plus adjacent
transitions touching them, are excluded. Cohorts combine position, age band,
prior NHL workload, and prior-value tier, then shrink empirical event rates
toward the global rate with a configurable pseudo-sample. The output must
disclose that entry rates are conditional on reaching the workload gate and
cannot estimate the probability that an unobserved prospect reaches the NHL.
The `development_calibration.v2` value model normalizes each feature within its
season and position group before applying position-specific weights and
workload credibility. Skater features are points/game, ice time/game,
shots/game, power-play points/game, and plus/minus/game. Goalie features are
save percentage, inverse GAA, starts, and shutouts/game. Missing optional
features contribute a neutral z-score and feature z-scores are capped at ±3.
The report retains all newest-target-season qualified players as a lookup table
so following-season scenario cohort assignments are auditable. Blocks, xG,
possession, matchup quality, and detailed special-teams deployment remain out
of scope until comparable historical inputs exist.

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

- schedule authority and optional point-in-time `as_of_date`;
- optional `replay_checkpoint` with league completed/remaining games and each
  team's observed GP, W-L-OTL, points, remaining games, and expected remainder;
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

`team_season_forecast_movement.v1` compares two sealed league artifacts from
the same season, schedule, trials, seed, and team set. It records both complete
run fingerprints and cutoff dates, then reports later-minus-earlier team
deltas for projected points, playoff/Cup probability, longest streak, newly
completed games, observed standings points, and expected remaining points.
Checkpoint-only fields remain null unless both inputs carry typed checkpoints.
`build_forecast_movement_card` projects one selected team into a sealed
`card_document.v1` with The Shift and Insider pages while preserving both
complete source fingerprints. It rejects malformed fingerprints, schema or
season drift, and missing team rows before any renderer receives the card.

`team_season_forecast_history.v1` chains two or more strictly chronological,
typed replay checkpoints. It preserves every source fingerprint and absolute
team projection, observed standings, and expected-remainder level, plus
core-computed deltas from the immediately preceding checkpoint. All inputs
must share season, schedule, teams, trial count, and seed.
The history also owns first-to-last points/playoff/Cup movement and deterministic
top-five league riser/faller rankings by projected-points change. Every team
also carries its 1-through-league-size projected-points movement rank; ties are
resolved by team abbreviation for reproducible output.
Core classifies each multi-checkpoint path as improving, declining, mixed, or
stable using a 0.05-point noise tolerance and identifies the largest signed
adjacent-checkpoint swing with its date interval.
Each history checkpoint retains the simulation's points P10/P50/P90. A
descriptive materiality ratio divides absolute net movement by the average
first/last P10-P90 width: below 10% is small, 10%-25% moderate, and 25% or more
large. A zero-width reference is indeterminate. This is not a statistical
significance test.
The first-to-last movement bridge is an identity, not a narrative guess:
projected-points change equals confirmed standings points gained plus the
change in expected remaining points. Core retains a reconciliation residual
and card projection rejects a residual above `1e-6`.
Core also derives a pace-normalized attribution:

`prior expected interval = first expected remaining points / first remaining games * newly completed games`

`net movement = (realized interval points - prior expected interval) + (last expected remaining points - (first expected remaining points - prior expected interval))`

The first term is realized performance versus prior average remaining pace; the
second is revaluation of the still-unplayed outlook. Core retains and validates
a second reconciliation residual. This attribution is descriptive, not causal:
uniform average pace does not account for schedule difficulty or identify why a
forecast changed.
Every checkpoint after the first retains the completed-game count, prior
expected interval points, realized-versus-prior-pace term, still-unplayed
revaluation term, and reconciliation residual for its adjacent interval. The
first checkpoint must carry none of those fields. Card construction recomputes
every adjacent forecast and probability delta plus both attribution terms and
rejects mismatches or residuals above `1e-6`.
`build_forecast_history_card` projects a selected team into The Tape and
Insider pages while retaining every checkpoint fingerprint. CLI, TUI, web,
and reference renderers consume those typed levels and deltas without
recomputing forecast movement.

Focused two-page team and simulation artifacts project this contract through
[`ui-neutral-card-system.md`](ui-neutral-card-system.md). The card builder may
select and explain a team subset, but it retains the league-run, parameter,
scenario, seed, and roster fingerprints and performs no forecast calculation
inside CLI, TUI, web, or image renderers.

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

Rolling replay must attribute the results-derived strength, verified
opening-roster prior, and post-opening personnel adjustment as distinct
reconciled factors. This permits independent `strength`, `opening_roster`, and
`personnel` ablations without changing the published probability.
Verified opening-strength rows must be cohort-relative: apply one shared offset
so their mean is 50 before replay. This retains between-team player-value
differences while preventing incomplete, non-random archive coverage from
creating a shared advantage versus neutral uncovered teams.

No current or future game result, future roster, or later-season statistic may
leak into an earlier forecast.

Cross-season validation must expose promotion as a gated decision, not infer it
from the pooled minimum alone. A candidate requires at least five seasons,
authoritative opening-roster/player-value evidence for every season, positive
Brier improvement over unblended IceLines on every held-out season, improvement
over pure Elo on at least 60% of holdouts and in the pooled score, and a span of
no more than 0.20 among holdout-selected blend weights. Passing these checks
means only `candidate_for_versioned_evaluation`; changing a production default
still requires an explicit model version. Missing roster authority or a failed
generalization check must retain an evaluation-only status with named failures.

Historical opening-roster recovery may satisfy promotion authority with an
archived official NHL API capture only when the archive URL is immutable and
timestamped, targets the exact team and season endpoint, and complete non-empty
coverage exists for every scheduled team. The maximum source-capture time is the roster evidence cutoff; local
import time remains separate and must never be backdated. The source manifest
is retained inside the sealed snapshot. A non-empty partial manifest may be
sealed only through an explicit evaluation-only import. Its player-value
effects are limited to teams with verified manifest captures; uncovered teams
remain neutral, and the artifact cannot satisfy cross-season promotion.
Mutable, non-official, or identity-mismatched archives cannot enable effects.
An archived official `current` roster is admissible only from July 1 through
the day before the schedule-derived opening date. The manifest opening date
must match the replay schedule; this prevents a generic current-roster capture
from being reassigned to another season.

An official historical boxscore supports the separately labeled
`retrospective_evaluation` opening-lineup lane by using only dressed-player
identity and position. Each team uses its own first-game date as the personnel
cutoff. The input must contain 15–18 unique skaters and two goalies; short
historical benches leave unfilled modeled slots neutral. Cached raw
boxscores remain reproducible source evidence. This lane cannot satisfy pregame
opening-roster authority and must never consume game results or boxscore
performance statistics as forecast inputs.

A live official NHL API roster capture may satisfy pregame authority only when
the sealed roster snapshot includes an
`icelines.official_roster_capture.v1` provenance manifest. Its season and
observation timestamp must match the snapshot index, its captures must cover
every scheduled team exactly once, and every source URL must be the canonical
official roster endpoint for that team and season. A sealed snapshot with
missing or unrecognized provenance is not authoritative.

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
