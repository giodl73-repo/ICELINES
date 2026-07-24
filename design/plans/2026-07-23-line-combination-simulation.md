# Line Combination Simulation

**Status:** Active child workstream of Team Season Forecast

## Outcome

Add a renderer-neutral lineup comparison and adaptive coaching layer to IceCast,
starting with Rangers lineup experiments while keeping every contract generic for
all teams and seasons.

## Delivery sequence

1. **Contract and evidence fence — complete**
   Define The Blender, The Bench, evidence labels, bounded deltas, validation,
   and UI-neutral output ownership.
2. **Adaptive policy foundation — complete**
   Extend season scenarios with ordered lineup choices, result-window decisions,
   and aggregate switch/usage summaries. Preserve deterministic seeded replay.
3. **Lineup evaluator — complete (bounded one-swap foundation)**
   Rank baseline and legal-swap configurations using player score, position fit,
   role balance, and optional coarse pair evidence. Emit
   `line_combination_forecast.v1`.
4. **Season comparison — complete (paired Rangers foundation)**
   Convert ranked choices into same-seed static and adaptive scenario runs and
   report wins, points, playoff, and Cup deltas.
5. **Rangers workflow — complete (CLI and reproducible examples)**
   Load the authoritative IceLines Rangers roster, accept curated combinations,
   render the best candidates, and expose advance/review behavior.
6. **Calibration and individual outcomes — in progress**
   Replay historical lineup changes only after shift/on-ice source authority is
   available; add player scoring distributions without overstating causality.
7. **Surface parity — planned**
   Add CLI JSON first, then web/TUI/card renderers over the same core documents.
8. **The Cut training-camp simulation — implemented (explicit-input foundation)**
   Seeded 12F/6D/2G selection now emits `training_camp_forecast.v1` with
   make/cut/bubble probabilities, modal rosters, and one-to-one incumbent
   displacement attribution. JSON inputs disclose translated samples,
   readiness, variance, health, waivers/contracts, and management deltas.
   Automatic NHL/AHL/prospect invite-pool assembly remains next.
   Development-role floors are now explicit: a top-six or top-nine prospect
   can be sent down when his sampled camp rank would leave him in an unsuitable
   NHL usage tier. The Rangers fixture now includes Liam Greentree as a natural
   right wing with a top-nine floor, independently from Cole Beaudoin's center
   competition. This closes the missing-acquired-prospect fixture bug without
   silently treating Greentree as a center.
   Prospect status, NHL rookie eligibility, pre-camp roster track, and simulated
   make probability are now orthogonal fields. An optional evidence-backed
   pre-camp probability enters through a disclosed log-odds adjustment, so an
   expected Hartford assignment can influence the camp contest without being
   mislabeled as either the prospect's performance score or its final result.
   The Cut now distinguishes the cap-compliant opening active roster from the
   12F/6D/2G dressed game lineup. Retained branches preserve dressed IDs and
   healthy scratches; the active roster is capped at the configured historical
   limit (23 by default). Optional salary-cap enforcement fails closed unless
   every invitee has a sourced cap hit, rejects over-limit trials, and emits
   branch cap space. Non-exempt cuts expose waiver probability without
   fabricating claim odds. Rangers and Kraken fixtures use 14F/7D/2G opening
   rosters, with Seattle serving as the first extras/scratch regression case.
9. **Roster-branch propagation — implemented (explicit retained branches)**
   `training_camp_lineup_set.v1` now converts retained camp branches into
   complete UI-neutral lineup projections with unchanged probabilities and
   stable player IDs. The CLI writes the reusable set with
   `--lineup-set-out`; the Rangers fixture covers Beaudoin and dual-eligible
   center repair. `training_camp_blender_set.v1` scores every retained branch,
   and `opening_roster_policies` sample exactly one mutually exclusive roster
   for the full regular season and playoffs. Unretained probability remains an
   explicit modal-strength residual. The compact policy path now scores up to
   3,000 outcomes by default without embedding full Blender documents; this
   covers all 2,669 Rangers outcomes with zero residual. Next: add Kraken and
   historical as-known fixtures.
10. **Special-teams deployment — implemented (usage-first foundation)**
    `team_lineup_projection.v1` now owns PP1/PP2 and PK1/PK2 assignments.
    Official prior-season PP/SH seconds per game flow through the shared stats
    repository, are GP-confidence weighted, and propagate through every Cut
    branch. Each PP unit names a defenseman quarterback. Unrated prospects are
    left unrated and incomplete units warn rather than fabricate a role. Next:
    compare alternative unit configurations and add web/TUI unit renderers.
11. **Management behavior — implemented (contract and game-plan foundation)**
    `team_decision_profile.v1` separates GM rookie/veteran/waiver behavior from
    manager deployment traits. `bench_game_plan.v1` assigns opponent-specific
    scoring, matchup, checking/energy, and shutdown-pair jobs, selects a tactical
    response, changes line shares under fatigue, models home last change, and
    compares both teams' back-to-back/travel load. The Cut consumes only GM
    traits and exposes each player's behavioral selection delta. Opponent-
    specific unit suitability now produces a bounded tactical edge that can be
    converted into an exact-date IceCast scenario event; the bridge validates
    the scheduled opponent and deliberately excludes the separate fatigue edge
    because the baseline forecast already models both teams' schedule load.
    `team_season_game_plan_schedule.v1` now authors that bridge across all 84
    team games, carries both teams' home/rest/congestion/travel context, and
    returns the auditable plans beside a simulation-ready scenario. Every
    distinct opponent needs an explicit style input; missing, duplicate, and
    unscheduled inputs fail closed. `player_matchup_role_evidence.v1` now turns
    historical rate facts into forward/defense peer-group percentiles while
    preserving GP for downstream confidence shrinkage, and
    `opponent_style_evidence.v1` classifies league-relative event archetypes or
    returns no-read below the games/coverage floor. Same-game co-appearance is
    explicitly excluded as chemistry evidence. The repository adapter now
    selects current-roster skaters, ranks complete records against the resident
    league window, warns with player IDs for missing realtime/TOI facts, and
    feeds the sealed role/style evidence directly into all 84 Bench plans.
    Opponent-style documents now retain source season, and any scheduled no-read
    stops authorship. `icecast season --game-forecast-out` now exposes the
    underlying per-game baseline, and `icecast bench` joins it to the lineup,
    decision profile, repository-derived player roles, and sealed opponent
    styles before writing both the 84-game schedule and reusable scenario.
    Next: add a true tracked-event source for team style, then expand those
    adapters across all 32 teams.
12. **Historical behavior learning — implemented (core calibration contract)**
    One-, two-, and three-season windows now aggregate auditable team-versus-
    league count facts with recency weights 1.00/0.65/0.40, opportunity-based
    confidence, no-read missing traits, and an as-known target-season fence.
    Simulated observations are rejected. Next: build schedule, transaction,
    roster, stats, and shift adapters for Rangers/Kraken and then all 32 teams.
13. **Cross-team behavior rankings — implemented (core contract)**
    Same-season profiles are now rankable on every GM/manager trait plus
    separate style composites. Rankings use effective confidence-adjusted
    values, preserve raw values and opportunity counts, report coverage, and
    leave no-read teams unranked. Next: populate the table from all-32 source
    adapters rather than synthetic profiles.

## Test gates

- validation rejects malformed policies;
- a losing review window changes the lineup and a successful one retains it;
- max changes and final-choice exhaustion are respected;
- policy deltas affect only the configured team and only after selection;
- aggregate usage reconciles to the team's simulated regular-season games;
- fixed seed produces identical output;
- existing scenario JSON without a policy remains compatible.
- camp trials always reconcile to 12F/6D/2G when the invite pool is sufficient;
- every make/cut probability reconciles to the configured trial count;
- prospects can displace incumbents without retrospective outcome leakage;
- waiver/contract preferences remain disclosed costs, never silent exclusions;
- camp-selected roster IDs are the exact IDs consumed by the lineup trial.
- retained lineup-branch probability equals the sum of its source camp branches;
- dual-eligible center repair produces complete 12F lineups without stale warnings.
- opening-roster selection is deterministic by seed and sums to every season trial;
- configured and sampled branch probabilities are both reported without renormalization.
- PP units contain one explicit dressed-defenseman quarterback and four forwards;
- PK units contain two dressed defensemen and two forwards;
- zero or absent historical usage cannot silently create a deployment role;
- special-team assignments are rebuilt after camp roster completion.
- GM traits change camp odds without manager-trait leakage into roster selection;
- opponent style can change matchup suitability and tactical response;
- matchup and checking lines remain distinct when the roster supports both;
- home last change raises, and relative fatigue lowers, hard-match confidence;
- a four-line fatigue manager increases energy-line share on compressed schedules;
- both teams' rest states produce a signed schedule-fatigue edge.

## Non-goal

Do not infer causal player chemistry from roster order, same-game presence, or
individual player ratings. Those signals can rank simulated fit only when the
document says so explicitly.
