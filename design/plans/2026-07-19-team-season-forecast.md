# Team Season Forecast — Implementation Plan

**Date**: 2026-07-19  
**Status**: In progress — baseline and core league simulation complete
**Specification**: [`../specs/team-season-forecast.md`](../specs/team-season-forecast.md)
**Archive when**: replay authority, calibrated model promotion, scenario
market, and claimed card/surface handoffs are complete or explicitly deferred

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

**First status (2026-07-20):** `--replay-mode rolling` now freezes each date's
picks before applying that date's final results, stores the exclusive evidence
cutoff and prior-game counts, and updates a neutral regressed strength only
from earlier standings points and goal differential. Present-day roster
substitution is explicitly disabled. Dated roster, injury, transaction, and
goalie membership intervals remain open.

**Personnel evidence status (2026-07-20):** modern ESPN transaction archives
now join replay chronologically as **The Wire**. All sourced rows retain date,
team, kind, description, ID, and provenance. One-direction IR placement and
activation rows maintain a conservative availability signal; ambiguous mixed
rows do not alter state. Stable player extraction and audit-safe active-roster
intervals are implemented; dated opening membership and quality-weighted
strength effects remain open.

**Identity status (2026-07-20):** transaction descriptions now resolve exact,
boundary-safe normalized full names to stable NHL player IDs. Multi-player
events retain every unique match and duplicate-name collisions remain explicit
ambiguities. The historical season repository supplies identity only, never a
future statistical feature. Membership direction and prior-season value remain
open.

**Membership-action status (2026-07-20):** each uniquely resolved player is
classified independently within its transaction clause. Recall, waiver claim,
and assignment language produces a signed NHL active-roster delta; trades,
acquisitions, releases, IR, waiver exposure, contract, and ambiguous actions
remain zero-delta at that scope. The ledger builds non-overlapping sourced and
`implied_preexisting` intervals and exposes duplicate transition anomalies.
Opening membership still requires a dated roster authority.

**Opening-authority status (2026-07-20):** **The Crease** now audits sealed
roster snapshots by season, capture timestamp, integrity, scheduled-team
coverage, and non-empty content. Same-day and later snapshots are rejected
under the calendar-date replay contract, and the complete decision is exposed
as `opening_roster_authority`. Player values activate only when the complete
strength join also succeeds.

**Opening-strength status (2026-07-20):** authoritative rosters now join only
to the preceding completed season's regressed player values. Twelve-forward,
six-defense, and two-goalie group scores use 55/30/15 weights, missing histories
remain neutral, overall edges are coverage-regressed, and current results fade
the opening prior over 20 equivalent games. Rejected authority produces no
opening-strength rows and preserves exact neutral replay behavior.

**Transition-state status (2026-07-20):** chronological membership and IR
state is now keyed by stable team/player identity. Duplicate recalls,
assignments, placements, and activations remain in The Wire but are idempotent
for game-level counts, matching the interval anomaly ledger.

**Exact-lineup status (2026-07-20):** authoritative opening rows now retain all
roster player IDs, names, position groups, prior/modeled values, and selected
slots. Later membership and IR actions recompute 12F/6D/2G strength; events on
or before the snapshot date are recognized as already reflected. Per-game
personnel deltas reconcile to zero when no post-snapshot event exists.

**Newcomer status (2026-07-20):** post-snapshot recalls and waiver claims now
add players absent from the opening snapshot when prior-season identity
provides a position group and regressed value. Unknown newcomers remain visible
membership evidence with no fabricated strength contribution.

**Paired-trade status (2026-07-20):** exact same-date acquired/traded-away
links for one stable player ID across two teams now produce auditable
`paired_trades`. A source player already known active transfers atomically to
the destination with value and IR state; unknown-source and unpaired trades
remain organizational-only evidence. Historical 2025-26 stress replay found
25 exact pairs: one supported active transfer and 24 conservative
`source_not_known_active` rows.

**Prior-value status (2026-07-20):** resolved players now carry an optional
0-100 value computed only from the preceding completed season. Skaters use
points per game and goalies save percentage with position-specific credibility
regression. Missing histories stay unknown. These values are disclosed inputs
and alter replay strength only for players with authoritative opening
membership or a later supported active-membership transition.

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

**Core status (2026-07-19): Implemented.** Seeded trials now sample one shared
result per game and maintain reconciled W-L-OTL, points, league rank, playoff
qualification, and longest win streaks. Chronological personnel/scenario
events now support dated, probabilistic strength changes without future
leakage. Seeded automatic injury/goalie risk now derives from named multi-lens
player records, age, games played, and role. Live status ingestion,
hunt/spoiler context, and full NHL tiebreak data remain open.

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

**First status (2026-07-19):** `--trade-mode plausible` now creates named,
need-based buyer/seller hypotheses from team outlook and player records. Paired
strength effects are deadline-valid and atomic under a shared seeded occurrence
key. A paired same-seed no-trade run now isolates team-level point, playoff,
Presidents' Trophy, and streak deltas. A third forced-completion run separates
conditional trade value from market occurrence probability. Trial-state buyer/seller
reclassification, packages/draft assets, and cap/roster legality remain open.

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

**Partial status (2026-07-19):** average records, point percentiles, playoff and
Presidents' Trophy probabilities, and longest-win-streak distributions are
implemented. Each trial now continues through a seeded divisional bracket to
produce Round 2, conference final, Cup Final, and Stanley Cup probabilities.
The final 45 days now derive trial-state hunt and spoiler classifications,
bounded race/form edges, and scheduled pivotal-game probabilities. Richer
league leaderboards now rank Presidents' Trophy, Stanley Cup, and longest-win-
streak candidates, while five-game Gauntlet windows identify schedule extremes
with road, congestion, and travel context. Multi-window trip narratives remain
open.

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

**First status (2026-07-20):** the game ledger now joins completed official
scores strictly after probability generation and reports pick accuracy and
Brier score overall and by confidence tier. Broader rolling-origin seasons,
parameter ablations, and fully leakage-proven historical personnel remain open.

**Calibration status (2026-07-20):** each graded game now records binary winner
log loss and native three-way regulation-home/regulation-away/OT-SO log loss.
The Review adds explicit coin-flip/uniform skill deltas, ten-point home-win
calibration bins, and expected calibration error. A 2025-26 rolling replay
graded all 1,312 games at 52.3% accuracy, 0.249 Brier, 0.691 binary log loss,
1.079 three-way log loss, and 0.013 calibration error; the narrow positive
baseline deltas are disclosed rather than promoted as strong performance.

**Baseline status (2026-07-20):** The Review now evaluates a configured
home-only forecast and a chronological Elo forecast over identical games. Elo
uses uniform 1500 openings, 22 rating points for home ice, K=20, 0.75/0.25
OT-SO credit, and same-date frozen updates. In the 2025-26 replay IceLines
narrowly beat home-only but trailed Elo: Elo reached 52.6% picks, 0.248 Brier,
and 0.689 log loss versus IceLines' 52.3%, 0.249, and 0.691. These negative Elo
improvement deltas are retained as the next model-quality target.

**Standings-baseline status (2026-07-20):** rolling replay now adds a
points-only baseline with the same neutral 20-game regression and prior-date
freeze, excluding goal differential and every roster/schedule/personnel
feature. On 2025-26 IceLines narrowly beat it: 0.2489 versus 0.2491 Brier and
0.6910 versus 0.6914 binary log loss. The small delta prevents unsupported
claims that the richer feature set is already a major forecasting advantage.

**Factor-ablation status (2026-07-20):** The Review now removes each frozen
factor contribution without refitting and rescores identical games. In the
2025-26 replay, away back-to-back (+0.0005 Brier), rolling strength (+0.0004),
home ice (+0.0002), and away 3-in-4 (+0.0002) helped. Travel (-0.0001), home
back-to-back (-0.0001), and timezone (-0.0002) hurt slightly; home 3-in-4 was
approximately neutral. The small magnitudes and activation counts remain in
the output so this single-season result is a tuning lead, not a default-weight
decision.

**Three-season audit status (2026-07-20):** rolling replays now pass for
2023-24, 2024-25, and 2025-26 (3,936 graded games). A discovered relocation bug
was fixed by restoring Arizona Central alignment and Mullett Arena travel/time
context alongside modern Utah. Across all three seasons, home ice, rolling
strength, and away back-to-back effects helped; timezone was neutral-to-negative.
IceLines beat home-only and points-only standings each season but trailed
chronological Elo each season. This establishes Elo integration/tuning as a
higher-value next step than reacting to any one small schedule-factor result.

**Elo-blend status (2026-07-20):** rolling replay now scores eleven frozen
IceLines/Elo mixtures from 0% to 100% Elo without changing production picks.
Season minima were 80% Elo in 2023-24, 70% in 2024-25, and 60% in 2025-26.
Across all 3,936 games, 70% Elo / 30% IceLines minimized Brier at 0.24285 and
log loss at 0.67858, versus 0.24699/0.68711 for IceLines and
0.24345/0.67973 for pure Elo. The blend therefore contains complementary
signal, but remains evaluation-only until held-out validation and explicit
model versioning justify promotion.

**Leave-one-season-out result (2026-07-20):** each season was then held out
while the other two selected the Brier-minimizing grid weight. Training chose
60% Elo for the 2023-24 holdout, 70% for 2024-25, and 80% for 2025-26. Every
held-out blend beat unblended IceLines by 0.00197 to 0.00612 Brier. It beat
pure Elo on the 2024-25 and 2025-26 holdouts and trailed pure Elo by only
0.00036 on 2023-24. This clears an initial generalization check but not the
broader-era, roster-complete, explicitly versioned promotion gate.

**Five-season expansion result (2026-07-20):** 2021-22 and 2022-23 added 2,624
held-out games and both preferred pure Elo in their individual sweeps. Across
6,560 games, the pooled minimum shifted to 90% Elo / 10% IceLines at 0.23983
Brier and 0.67234 log loss, versus 0.24694/0.68700 for IceLines and
0.23997/0.67264 for pure Elo. Five-fold leave-one-season-out selection beat
IceLines on every holdout and pure Elo on the three newest holdouts, while
trailing pure Elo slightly on the two oldest. This invalidates promotion of
the earlier 70% in-sample candidate and keeps blending evaluation-only.

**Historical-playoff boundary (2026-07-20):** a 2020-21 probe proved that the
868-game ledger was gradeable but current alignment could silently produce a
plausible-looking, incorrect 16-team bracket. Season simulation now refuses
pre-2021-22 calendars until temporary/legacy division and playoff rules are
modeled. Historical focus validation uses loaded schedule membership, allowing
`ARI` and removing absent pre-expansion `SEA` defaults.

**Automated validation status (2026-07-20):** `icecast backtest` now consumes
three or more graded season JSON files and emits
`team_game_forecast_validation.v1`. The typed core rejects duplicate seasons,
empty grading, and incompatible/non-finite grids; it computes game-weighted
pooled curves and leave-one-season-out rows without holdout leakage. The live
five-file command reproduced 6,560 games, pooled 90% Elo, and the expected
80/80/90/90/90 holdout selections with explicit model/Elo deltas.

**Promotion-gate status (2026-07-20):** cross-season validation now reports six
named checks covering sample size, opening-roster authority, holdout
generalization versus IceLines and pure Elo, pooled Elo improvement, and weight
stability. The live five-season audit passes all five statistical checks but
has authoritative opening rosters for 0/5 historical artifacts, so its status
is `evaluation_only_missing_roster_authority`. A future all-pass result is only
`candidate_for_versioned_evaluation` and cannot mutate production defaults.

**Historical-roster ingestion status (2026-07-20):** the shared NHL membership
helper now drives bulk roster fetches, NHL client batch fetches, and roster
overlays. Coyotes seasons use `ARI`, Utah seasons use `UTA`, and the 2020–21
audit boundary omits pre-expansion Seattle. A 2023–24 operator dry run produced
32 requests with ARI and SEA present and UTA absent. Current downloads are not
backdated, so incomplete timestamped archive coverage leaves the five-season
opening-roster promotion gate honestly unresolved.

**Archive-import status (2026-07-20):**
`icecast import-opening-rosters` now validates a complete season manifest of
immutable Internet Archive `id_` captures targeting exact official NHL roster
endpoints. It derives rather than trusts capture timestamps, rejects identity
and coverage defects, downloads/parses all teams before creating state, stores
the provenance manifest inside the integrity-sealed snapshot, and records
archive evidence time separately from local import time. The Crease accepts
that evidence class while continuing to reject arbitrary backdated snapshots.

**Archive-discovery status (2026-07-20):**
`icecast discover-opening-rosters` now derives the opening boundary and team set
from the season schedule, queries exact official roster URLs through CDX, and
selects the latest strictly pre-opening capture. Missing captures and transport
errors remain distinct, and an importer manifest is emitted only at 100%
coverage. The first live 2024–25 scan found 5/32 captures, 13 confirmed gaps,
and 14 request failures; it correctly produced no import manifest. The initial
sequential scan exposed a 7¾-minute degraded-service path, so discovery now
uses a bounded four-request concurrency ceiling. A second degraded-service run
finished in 121.1 seconds (nearly 4x faster) but returned 2 captures, 8 gaps,
and 22 request failures. Coverage is therefore not inferred from any single
outage-heavy scan; confirmed captures remain usable candidates while errors
remain unresolved.

**Cumulative discovery status (2026-07-20):** per-team CDX responses are now
atomically cached after successful parsing and revalidated before fallback use.
In a two-pass 2024–25 live audit, pass one found 6 captures with 9 unresolved
request errors. Pass two completed in 72.3 seconds, reused cached BUF and CGY
responses, eliminated request errors, and raised confirmed coverage to 7/32.
The remaining 25 teams are confirmed gaps, so no import manifest was written.

**Current-endpoint archive status (2026-07-20):** discovery now checks an
archived official `/roster/{team}/current` response only when the season URL has
no usable capture. Manifests carry the schedule-derived opening date, current
captures must fall between July 1 and the day before opening, and The Crease
requires that manifest boundary to match the replay schedule. Season-endpoint
evidence remains preferred.

Two cumulative live 2024–25 scans found no additional qualifying `current`
captures. Coverage remained 7/32; the second pass classified 23 teams as
confirmed gaps, retained 2 request errors, and used 8 cache fallbacks. This
source remains supported for other seasons, but it does not close the 2024–25
authority gap.

**Partial-evaluation status (2026-07-21):** discovery can now emit the verified
capture subset separately from its fail-closed complete manifest, and import
requires an explicit `--allow-partial-evaluation` opt-in for that subset. The
sealed snapshot preserves identical archive provenance and integrity checks.
Rolling replay applies opening player weights only to verified teams, keeps
uncovered teams neutral, labels the authority `partial_evaluation`, and leaves
the production-promotion check blocked until full league coverage exists.
Cache-only discovery can reproduce the audited subset without contacting the
archive; absent endpoint caches remain unresolved errors rather than inferred
gaps.

The first cache-only 2024–25 execution reproduced 7/32 teams: CBJ, FLA, NSH,
NYR, SEA, UTA, and VAN. ANA and SJS remained unresolved because no cached
`current` response existed; 23 other teams remained confirmed gaps. Applying
the manifest exposed headerless gzip from Wayback despite an
`application/json` response. Import now performs three bounded attempts,
recognizes gzip magic bytes, limits decompression to 4 MiB, and reports a short
payload signature on failure. The corrected import sealed all seven rosters.

A 1,000-trial rolling replay then graded all 1,312 games with exactly seven
opening-strength rows and `partial_evaluation` authority. The full five-season,
6,560-game backtest retained
`evaluation_only_missing_roster_authority`; its opening-roster promotion check
remained 0/5 because partial evidence is intentionally excluded.

The replay now splits the prior combined strength attribution into reconciled
`strength`, `opening_roster`, and `personnel` factors. The first two partial
replays exposed that every raw recovered-team strength exceeded neutral 50,
creating an unsupported shared advantage because archive coverage is not a
random league sample. Opening strengths are now cohort-centered with one
shared offset, preserving relative team differences while forcing the verified
cohort mean to 50. Uncovered-only games remain unchanged.

After normalization, the five-team 2023–24 cohort produced a +0.0000223 Brier
improvement across 384 affected games; the seven-team 2024–25 cohort produced
+0.0000033 across 521 games. Both effects are tiny but now directionally
positive and methodologically comparable. They remain evaluation-only.

**2023–24 archive execution (2026-07-21):** the bounded live scan found CGY,
NJD, PHI, PIT, and SJS (5/32), with five request errors and 22 confirmed gaps.
Import exposed historical optional location fields encoded as `{}`. The shared
roster schema now maps an empty object to missing only for optional birthplace
fields while continuing to reject empty required player names. The corrected
five-team snapshot sealed and passed full-season rolling replay.

**Five-season archive boundary (2026-07-21):** two-pass convergence scans now
cover the full calibration window. The official modern roster endpoint yielded
0/32 captures for 2021–22 (31 confirmed gaps, NYI unresolved), 0/32 for 2022–23
(29 gaps, CAR/FLA/NYR unresolved), 5/32 for 2023–24, and 7/32 for 2024–25.
The existing 2025–26 artifact also lacks pre-opening archive authority. Empty
partial manifests are refused, so the two zero-coverage seasons created no
snapshot.

The official legacy `statsapi.web.nhl.com` source was probed before adding any
implementation: exact season roster, current roster, expanded team roster,
hydrated roster, and a wildcard NYR inventory over the 2021 pre-opening window
returned no immutable captures (two complex-query probes timed out). It is not
a viable evidence source on current proof and no dead importer was added.

Official first-game boxscores remain a possible separate retrospective lineup
evaluation lane because they identify dressed skaters and goalies for every
team. They cannot satisfy pregame opening-roster promotion authority: their
fact date and later observation/import time must remain distinct, and the
product must label them retrospective rather than backdating evidence.

A live feasibility probe of game `2021020004` (NYR at WSH, October 13, 2021)
returned exactly 12 forwards, six defensemen, and two goalies for each club,
with stable player IDs and position codes. Player display names are abbreviated
but completed prior-season records can resolve full identity by ID. Before this
lane can enable transaction effects, the replay authority contract needs a
per-team first-game cutoff; one league-wide cutoff would either double-apply
early transactions or suppress valid later changes.

**Retrospective lineup execution (2026-07-21):**
`--retrospective-opening-lineups` now reuses the existing per-team `as_of_date`
contract, extracts identity/position only from official first-game boxscores,
requires 15–18 unique skaters plus two goalies, and atomically caches the raw
source by season/game. The full 2021–22 run covered 32/32 teams through 23
unique boxscores, centered opening strength at exactly 50, and graded 1,312
games. Its opening-roster factor improved Brier by 0.0002465. A second run used
23/23 cache hits and reproduced every game probability exactly.

The first 2022–23 expansion exposed Edmonton dressing 17 skaters in its opener.
The parser now accepts legitimate 15–18-skater short benches with two goalies;
unfilled modeled slots remain neutral rather than rejecting real NHL evidence.

The completed 2021–22 through 2025–26 matrix covered 32/32 teams and 1,312
games in every season (6,560 games total). Opening-lineup strength improved
Brier in all five seasons by 0.0002465, 0.0001849, 0.0004406, 0.0001790, and
0.0000911 respectively; binary log loss also improved in every season. The
leave-one-season-out backtest selected 0.8–0.9 chronological-Elo weights, beat
unblended IceLines in all five holdouts, beat pure Elo in three, and produced a
pooled 0.23981 Brier at 0.9 Elo versus 0.23997 for pure Elo. All statistical
gates passed, but the result remains
`evaluation_only_missing_roster_authority` with 0/5 authoritative seasons.
That is the intended proof that useful retrospective player evidence cannot
promote itself into pregame authority.

**2026–27 live authority execution (2026-07-21):** the first gate audit found
that the existing July 16 snapshot was sealed, hash-valid, and complete but had
no explicit source provenance. Live roster fetches now seal a versioned source
manifest containing the observation timestamp and canonical official NHL API
URL for all 32 teams; unmanifested local snapshots fail closed. A fresh July 21
capture passed snapshot integrity and the hardened IceCast gate against the
September 29 opener with 32/32 verified teams and player-value effects enabled.

Standard preseason simulation now emits the same structured authority record,
not only rolling replay. Current-season team strength is cleared to neutral if
the gate fails. A seeded 10,000-trial July 21 baseline retained all 1,344 league
games and 84 games each for NYR and SEA: NYR averaged 45.01 wins and 99.29
points with a 67.05% playoff probability and 5.21% Cup probability; SEA
averaged 39.72 wins and 89.37 points with a 40.80% playoff probability and
1.29% Cup probability. These are reproducible as-of-capture forecasts and must
be regenerated as preseason rosters change.

**Rangers/Kraken showcase execution (2026-07-21):** the authoritative baseline
is published as a readable 168-row game report with factor explanations,
easiest/hardest five-game windows, pivotal hunt/spoiler games, playoff path,
and league streak/Cup leaders. A deterministic injury stress fixture places
Shesterkin outside NYR's hardest window and Daccord outside SEA's hardest road
trip. Paired 10,000-trial deltas were -0.14 points/-0.54 playoff percentage
points for NYR and -0.11/-0.47 for SEA.

The first plausible-trade showcase proposed young core and franchise-level
players because the trade input does not yet carry contract expiry. The
generator now fails conservative: only ages 27–36 with meaningful workload and
non-elite modeled value enter its rental-like proxy pool. A regression test
proves a higher-rated young core defender is skipped for an eligible veteran.
The regenerated market sends John St. Ivany to NYR at 30% occurrence; completion
adds 0.14 expected points and 0.50 playoff percentage points in the paired run.
SEA makes no modeled acquisition in this market and remains effectively flat.

Authored and automatic scenarios now populate `scenario_impacts` too, rather
than requiring manual subtraction. Trade mode retains separate market-weighted
and forced-completion deltas. All comparisons reuse schedule, seed, trials, and
non-target scenario inputs.

**NYR 10% Cup threshold (2026-07-21):** a paired preseason sweep at +8, +12,
+16, and +20 net strength produced 7.21%, 8.68%, 9.99%, and 11.53% Cup odds.
The practical preseason threshold is therefore about +16: 103.77 expected
points, 82.52% playoffs, 49.70% second round, 28.56% conference final, and
16.23% Cup Final. That is approximately four +4 top-six-equivalent net
upgrades, or a balanced elite-forward/top-pair-defense/secondary-depth package;
one ordinary top-six forward is not enough.

Waiting until March 5 requires about +22 net strength. Deadline sweeps at +8,
+12, +16, and +20 reached only 6.59%, 7.50%, 8.61%, and 9.50%; +22 reached
10.13% Cup odds. The deadline threshold yields 100.52 expected points and
71.68% playoffs because only 17 regular-season games benefit, but the full
upgrade remains active in every playoff series. Both thresholds are 10,000-run
estimates with roughly three-tenths of a percentage point of sampling error at
a 10% event rate, not exact guarantees.

**NYR internal breakout path (2026-07-21):** Alberts Smits is not present in
the official active-roster endpoint, so the scenario treats his NHL arrival as
an explicit net addition rather than silently inserting prospect value. An
immediate positive top-four-quality Smits season is modeled at +3: NYR rises
from 99.29 to 100.13 points, 67.05% to 70.31% playoffs, and 5.21% to 5.98% Cup.
Smits alone is meaningful but leaves roughly +13 strength to reach the 10%
tier.

One concrete all-internal +16 path requires five simultaneous development
wins: Smits +3, Gabe Perreault +4, Noah Laba +3, Tye Kartye +2, and Alexis
Lafrenière +4. Conditional on all five, the paired run reproduces 103.77
points, 82.52% playoffs, and 9.99% Cup. Equivalently, four major +4 breakouts
could reach the same modeled tier. These are conditional ceiling scenarios,
not independent breakout probabilities or prospect guarantees.

**NYR/SEA development distributions (2026-07-21):** probabilistic scenario
fixtures combine individually sampled breakouts with full-season veteran and
goalie downturns. The first heuristic run was intentionally optimistic; it
landed NYR at 99.87 points/5.55% Cup and SEA at 89.92/1.42%.

The reusable `icecast calibrate-development` command now replaces those hand
rates using 11,156 consecutive-season player transitions from 2005-06 through
2025-26, excluding transitions touching shortened lockout/pandemic seasons.
Calibration v2 uses season- and position-normalized scoring, deployment, shot,
power-play and plus/minus lenses for skaters plus save percentage, inverse GAA,
starts and shutout rate for goalies. At the default ±2 team-strength thresholds,
the global performance-conditional rates are 16.58% breakout and 25.40%
downturn, with median realized deltas of +3.24 and -3.19 across 89 cohorts.
Position/age/experience/prior-value cells use 20 global pseudo-observations for
shrinkage. Workload gates are 20 target games for skaters and 15 for goalies,
so injuries/non-arrivals remain separate risks. A 785-player newest-season
lookup makes each following-season cohort assignment reproducible.

After remapping each named player through the v2 lookup, the paired 10,000-trial
NYR run is 98.93 points, 65.25% playoffs, and 4.93% Cup (-0.36 points, -1.80pp
playoffs, -0.28pp Cup versus baseline). Three or more of the five upside events
occur in 9.97% of trials, while two or more of the
Miller/Zibanejad/Shesterkin downturns occur in 44.47%. SEA remains almost
exactly neutral at 89.35 points, 40.75% playoffs, and 1.36% Cup (-0.02 points,
-0.05pp playoffs, +0.07pp Cup); three-plus breakouts occur in 16.88% and
two-plus downturns in 47.49%. The larger two-sided realization rates mostly
cancel in the unconditional mean but materially widen the causal paths.
Smits retains an explicit 30% scouting prior: NHL-only transition data can
estimate performance conditional on arrival but cannot observe drafted
prospects who never reach the workload gate.

Seattle's separate all-five `+16` ceiling run removes downturns and forces all
five internal wins. It reaches 93.84 points, 60.58% playoffs, 30.45% second
round, and 2.63% Cup (+4.47 points, +19.78pp playoffs, +1.34pp Cup). Equivalent
aggregate growth therefore makes Seattle a solid playoff contender, but not a
10% Cup team from its lower baseline.

The new `scenario_outcomes` report groups every team trial by positive and
negative event counts with conditional points/playoff/Cup results. Its first
stress run exposed artificial correlation between similarly prefixed event
IDs caused by consuming the first xorshift output from raw XOR/FNV seeds.
SplitMix-style avalanche mixing now precedes event draws, and a regression test
requires all zero-through-five success-count combinations plus bounded event
marginals. Explicit `correlation_key` events remain atomic.

**Top-six acquisition sensitivity (2026-07-21):** paired 10,000-trial runs
model the same +4.0 strength top-six forward for both showcase teams, acquired
for non-roster assets. A preseason acquisition affects all 84 games: NYR rises
to 100.38 points and 71.36% playoffs (+1.09 points, +4.31pp), while SEA rises
to 90.46 and 45.76% (+1.08, +4.96pp). The identical March 5 acquisition has
only 17 NYR and 18 SEA games to act: NYR gains 0.23 points/+0.91pp playoffs and
SEA gains 0.24/+1.05pp. These are player-only gross upgrades; including an NHL
roster player in the return requires subtracting that player's strength.

- Add rolling-origin completed-season backtests.
  The validation contract now performs chronological logistic-calibration
  holdouts from compact graded-game observations: every season after the first
  is recalibrated only from earlier supplied seasons and reports held-out Brier
  and log-loss deltas. Core now also emits a game-weighted pooled summary with
  before/after losses, signed gains, and improved-holdout counts so every
  surface shares one verdict. Paired per-game standard errors and 95% intervals
  now distinguish a measured gain from its sampling uncertainty while clearly
  excluding selection uncertainty. A separate delete-one-holdout-season
  jackknife interval now exposes season-clustered variation, with explicit
  small-sample and conditional-fit caveats. Core now withholds the corresponding
  evidence verdict as `insufficient_holdouts` until four holdout seasons exist,
  then classifies the clustered interval relative to zero. Producing a
  promotion-grade artifact still requires the remaining historical replay
  seasons and roster authority.
  The summary uses serde defaults and has a regression fixture for pre-interval
  v1 JSON, preserving sealed artifacts as fields evolve additively.
  `scripts/generate-icecast-validation.ps1` now operationalizes the missing
  evidence run: it plans or generates the default five chronological replays,
  derives prior stats seasons, validates each graded artifact, and invokes the
  backtest without masking partial roster authority. Valid sealed replays are
  reused for resumability unless `-ForceReplay` is set. Its fake-executable
  regression covers ordering, flags, derived seasons, complete input wiring,
  initial generation, resume behavior, and forced regeneration.

**Chronological calibration execution (2026-07-23):** the default runner
generated five 1,312-game replays and sealed
`icecast-validation-20212022-20252026.json` (12,625 bytes; SHA-256
`00069a517045a3aa4689892cfc3ce844cd208ea5fb74cca249ecd07234fb0ff9`). The
90% Elo pooled blend again scored 0.23981 Brier versus 0.23997 for pure Elo;
all blend-quality gates passed, but 0/5 authoritative opening rosters kept the
artifact `evaluation_only_missing_roster_authority`. The four chronological
calibration holdouts graded 5,248 untouched games. Recalibration improved Brier
0.24691 → 0.24438 (+0.002531) and binary log loss 0.68695 → 0.68178
(+0.005168), while delete-one-season intervals crossed zero at
[-0.000277, 0.005339] and [-0.000618, 0.010954]. Both evidence labels are
therefore `inconclusive`, not promotion evidence. Three of four chronological
holdouts improved; applying the earlier-season correction to the newest
2025-26 holdout worsened Brier by 0.001246 and log loss by 0.002637. This latest
miss is direct evidence against deploying one static historical correction.
- Report Brier score, multiclass log loss, calibration bins/slope, and baseline
  deltas. Complete for the sealed completed-season replay: binary/multiclass
  losses, skill scores, decile bins/ECE, logistic calibration intercept/slope,
  chronological baselines, factor ablations, and Elo blend sweeps are core-owned.
  The sealed 2024-25 fit is intercept -0.378 (95% interval -0.687 to -0.069)
  and slope 4.417 (2.380 to 6.454). Because the slope interval remains wholly
  above the ideal 1, the replay provides evidence that this model version was
  materially too compressed toward 50/50; this remains evaluation evidence,
  not permission for in-sample recalibration.
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

## Point-in-Time Replay Checkpoint (2026-07-22)

- `icecast season --replay-mode rolling --through YYYY-MM-DD` strips later
  result labels before the rolling model is built and limits dated personnel
  evidence to the same boundary.
- Confirmed games through the cutoff seed every Monte Carlo trial; only the
  remaining schedule is sampled, so standings, recent form, and streaks begin
  from the actual point-in-time state.
- `team_season_forecast.v1` records `as_of_date`, and core rejects incomplete
  prior results or any future result label that crosses the cutoff.
- The typed `replay_checkpoint` carries league and team observed records plus
  games remaining into CLI text and the UI-neutral Scoreboard without
  renderer-local standings calculations.
- Each team checkpoint also carries expected remaining W-L-OTL and points;
  regressions require observed plus remainder to reconcile to projected final
  games and points.
- `icecast movement` compares two sealed league artifacts through the core
  `team_season_forecast_movement.v1` contract. It retains both full-run
  fingerprints and exposes forecast, observed, and remaining-points movement
  before any CLI team filtering.
- `icecast movement-card` projects one team through the core
  `forecast_movement_card.v1` builder. The Shift/Insider pages retain both
  source fingerprints and carry typed deltas into generic card renderers.
- Sealed Jan. 31 and Feb. 28, 2025 1,000-trial checkpoints back the first
  NYR/SEA movement showcase. Named TUI launchers and read-only HTML/JSON routes
  consume the identical focused cards with shared source fingerprints.
- `icecast history` chains any number of dated replay artifacts through
  `team_season_forecast_history.v1`. The Tape retains every absolute checkpoint
  and immediate prior delta for all teams before text focus is applied.
- Every adjacent history interval now reconciles its forecast change into
  realized standings points versus the prior checkpoint's expected remaining
  pace plus revaluation of the still-unplayed outlook. Core and card validation
  reject inconsistent deltas, regressing replay progress, or reconciliation
  residuals above tolerance.
- The first multi-checkpoint showcase seals Jan. 31, Feb. 28, and Mar. 31,
  2025. `scripts/generate-icecast-history-showcase.ps1` parameterizes season,
  prior stats season, dates, trials, and seed so the same flow rolls forward.
- Focused regressions cover fixed known outcomes and the future-label leakage
  fence; the CLI parser covers the dated option.
- Paired isolated attribution supports the same cutoff. Its baseline, natural,
  one-event, and forced-ceiling simulations expose `as_of_date` and share the
  exact fixed-result state.

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

**Status (2026-07-19): Complete.** `icelines icecast season` now loads the
official schedule, validates the 2026–27 1,344/84/42+42 shape, derives schedule
context, applies roster/depth strength, and renders or serializes
`team_game_forecast.v1` for all teams with Rangers/Kraken defaults. Monte Carlo
state and the later milestones below remain open.

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
