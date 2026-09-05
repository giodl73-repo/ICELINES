# Weekly Operations Planner and Decision Journal

**Date**: 2026-09-05

**Status**: Implemented — validated 2026-09-05

**Parent**: [`2026-07-18-fantasy-war-room-roadmap.md`](2026-07-18-fantasy-war-room-roadmap.md)

**Predecessor**: [`2026-09-05-league-aware-daily-decisions.md`](2026-09-05-league-aware-daily-decisions.md)

**Primary contracts**: `fantasy_pickup_sequence.v1`,
`fantasy_decision_journal.v1`

**Role review**:
[`../../signals/roles/check/weekly-operations-planner-roles-check-2026-09-05.md`](../../signals/roles/check/weekly-operations-planner-roles-check-2026-09-05.md)

## Outcome

Turn the daily cockpit's best current action into a legal Monday-Sunday plan.
The manager sees how to spend the remaining acquisition budget, which move to
hold for injury or goalie uncertainty, which starter each bench player covers,
and what fallback to use when a target is claimed.

```powershell
icelines fantasy week-plan --week 2026-11-09
icelines fantasy week-plan --week 2026-11-09 --reserve 1 --json
icelines fantasy decision-record --recommendation-id ID --chosen 1
icelines fantasy decision-review --week 2026-11-09
```

The planner is advisory and read-only. Recording the manager's choice is an
explicit local mutation; no command changes a Yahoo roster or submits a claim.

## Product boundary

- IceLines owns league-neutral schedule, scoring, roster-legality, and decision
  contracts. Personal preferences and private season notes remain in PUCK.
- Yahoo or manual imports may supply league state, but cached official NHL
  schedules and NHL-keyed statistics remain the hockey authority.
- The planner never treats missing ownership, eligibility, waiver, injury,
  lock, or schedule evidence as permission.
- A sequence may contain fewer moves than the available budget. Churn must earn
  its acquisition cost and retained-player loss.
- Decision capture is append-only. Outcomes never rewrite the recommendation
  or evidence that existed when the choice was made.

## Existing foundation reused

- `FantasyAssistantRules` owns roster shape, timezone, weekly acquisition
  limit, waiver duration, reserve policy, and playoff dates.
- `FantasyWeekBudgetView` owns the Monday-Sunday ledger and proactive reserve.
- `FantasyWeeklyMoveInput` owns the current one-move value components.
- `build_fantasy_daily_lineup` owns exact daily slot assignment.
- `FantasyBenchCoverageView` owns quiet-night and collision evidence.
- `FantasyTodayView` owns the current primary decision and material
  fingerprint.
- `FantasyDb` owns local league, roster, eligibility, matchup, waiver, and
  acquisition state.

The new planner factors weekly roster simulation out of the CLI and into core.
No renderer or database adapter may recompute sequence value.

## Domain model

### Planning identity

Every plan is keyed by:

```text
(league_id, fantasy_team_id, week_start, week_end,
 stats_season, season_type, competition_mode, evaluated_at)
```

Player hockey evidence remains keyed by `(player_id, season, season_type)`.
Platform eligibility and ownership remain league-scoped overlays and never
replace canonical NHL position.

### Planner input

```text
FantasyPickupSequenceInput
  context + FantasyAssistantRules + FantasyWeekBudgetView
  roster[] + available_candidates[]
  schedule_dates_by_team
  waiver/availability evidence + locks/status evidence
  matchup objective (points delta or category posture, optional)
  max_candidates + beam_width + alternatives
```

Each player carries a stable player key, display name, NHL team, platform
positions, league-scored per-game value, game dates, ownership state, and
drop/lock/status constraints. Absence is typed; GP=0 is unavailable projection,
not a zero-value player.

The league-scoped platform key and optional canonical NHL `PlayerId` are
separate fields. Display names are never join keys. Every transaction carries
an exact UTC effective instant, applicable UTC lock instant, local date, and a
stable ordinal. The schedule map is an immutable child of the complete
season/type planning context and is rebuilt whenever either axis changes.

### Planner output

```text
FantasyPickupSequenceView (`fantasy_pickup_sequence.v1`)
  context + readiness + budget
  primary_sequence? + alternatives[]
  holdback recommendation
  daily coverage summary[]
  evidence[] + warnings[]
  evaluated candidates / beam width / truncation / elapsed
  material_fingerprint

FantasyPickupSequence
  sequence_id + projected_value_delta + incremental_usable_starts
  moves_used + reserve_after + matchup impact
  moves[] + daily coverage[] + reasons[]
  pre_roster_fingerprint + post_roster_fingerprint

FantasyPickupSequenceMove
  ordinal + effective_at + local_date + add + drop?
  legality + waiver/lock deadline
  newly usable dates + displaced/covered starter evidence
  marginal value after prior moves
  fallback add candidates[]
```

The primary sequence is the best-valued legal complete state among the bounded
states actually evaluated. Alternatives
must differ materially by at least one add, drop, or effective date. A no-move
sequence is always evaluated and wins when every legal move is negative.

All numeric inputs must be finite. Invalid values return a typed
`FantasyPickupSequenceError`; adapters add context but never parse error text.

## Search and scoring contract

The initial implementation uses deterministic bounded beam search:

1. Build the baseline daily lineup for every remaining date through Sunday.
2. At each acquisition depth, enumerate legal add/drop transitions from the
   current hypothetical roster and effective time.
3. Re-run daily lineup assignment after every transition; never sum independent
   one-move scores.
4. Reject intermediate roster-shape, ownership, waiver, lock, status, duplicate
   player, and budget violations immediately.
5. Rank partial states by the exact objective below and retain only
   `beam_width` states.
6. Stop at the proactive limit unless an explicit injury-reserve override is
   active. Release the configured reserve on its saved release date.
7. Return the best terminal state, distinct fallbacks, and disclosed truncation.

For points mode, every transition and terminal sequence uses:

```text
net_value = after_active_points - before_active_points
          + matchup_points_delta
          + future_schedule_option_value
          - waiver_reacquisition_cost
          - acquisition_budget_cost
          - uncertainty_discount
```

All terms are league fantasy points over the named horizon except acquisition
cost and uncertainty, which are explicitly scaled penalties supplied by the
versioned planner policy. Goalie minimum/capacity is evaluated separately and
may make an otherwise positive state illegal or conditional; it is never folded
silently into skater value.

Points and category leagues keep separate objective functions. Category mode may consume an existing
matchup posture; if it is absent, category impact is unavailable rather than
zero and the plan is provisional.

`projected_value_delta` is descriptive under the supplied player rates and
schedule. It is not a calibrated probability or guarantee. Candidate cap,
beam width, elapsed time, and truncation are serialized.

## Quiet-night and bench-substitution contract

Daily coverage reports scheduled roster players, usable starters, benched
collisions, open slots, and quiet-slate starts before and after each move. A
bench player's value is credited only on a date where legal slot assignment
starts that player. The explanation names the displaced or newly covered slot;
raw scheduled games never masquerade as usable starts.

For defensemen, the planner explicitly evaluates all three D slots. A bench D
receives no substitution credit on a date where the active D slots are already
filled by higher-valued eligible players.

## Waivers, locks, and contingencies

- Free agents may become effective immediately subject to the league's daily
  lock policy.
- Waiver candidates use their sourced `usable_at`; they cannot contribute to an
  earlier game.
- A locked player cannot be dropped before the applicable game lock.
- A player dropped in a hypothetical sequence cannot be reacquired inside the
  initial horizon unless a complete new waiver window proves legality.
- Transitions at the same instant execute by stable ordinal as atomic
  drop-then-add pairs; duplicate ordinals are rejected.
- If a primary target becomes unavailable, fallbacks are re-simulated from the
  same pre-move state. They are not copied from a global ranking.
- Unknown injury or ownership state makes the affected transition conditional
  or blocked, never firm.

## Decision journal

SQLite migration 020 adds two append-oriented tables:

```text
fl_decisions
  id, league_id, fantasy_team_id, kind, recommendation_id,
  recommendation_fingerprint, recorded_at, evaluated_at,
  chosen_alternative, manager_rationale, projection_json

fl_decision_outcomes
  id, decision_id, observed_at, outcome_kind, outcome_json,
  correction_of?
```

`projection_json` is the exact versioned decision projection shown to the user,
not a mutable foreign-key view of current data. Duplicate capture of the same
league/team/recommendation fingerprint is idempotent. Corrections append a new
outcome row pointing to the prior row; they never update history in place.

The v1 review surface separates:

- decision quality at the time;
- projection error after outcomes exist;
- realized matchup result;
- whether the held reserve was eventually needed.

The exact display labels and stable player keys are both preserved in the
immutable projection. Manager rationale is private by default and excluded from ordinary Web/API and
export output unless an explicit include-private option is added later.

The planner and journal use separate service paths. Planning opens the fantasy
database read-only after migrations already exist and is proven not to change
the database, WAL, or SHM state. Recording and outcome attachment are explicit
mutations. Re-reading old journal rows treats their versioned projection JSON as
opaque bytes unless a matching decoder exists.

## Surface contract

### CLI

The 80-column scan path is context, budget/holdback, primary sequence, fallback
sequences, daily coverage, evidence/recovery. `--json` emits the full contract.
Mutation commands state exactly what local row was appended.

### TUI

The fantasy cockpit adds a Week Plan drill-down. It consumes the shared view,
shows the primary sequence first, and exposes daily coverage and alternatives
without recomputation. Narrow layouts preserve dates, add/drop names, reserve,
and firmness before secondary metrics.

### Web

`GET /fantasy/week-plan` and `GET /api/v1/fantasy/week-plan` are bookmarkable,
read-only surfaces with `league`, `team`, and ISO-Monday `week` in the URL.
Invalid or non-Monday weeks return a typed 400 with a canonical recovery URL.
Responses use `Cache-Control: no-store`. The HTML works without
JavaScript and uses semantic ordered lists/tables. Decision recording remains
an explicit mutation and is not mounted as a GET route.

The generated static site is out of scope because private, time-sensitive league
plans do not belong in a durable public artifact.

### Daily cockpit

`fantasy today` may project the first still-pending legal move from the saved or
freshly assembled week plan. It must include the plan fingerprint and may not
silently substitute a newly recomputed independent pickup.

One `icelines-fetch` assembly service owns database reads, stats/schedule
loading, evidence reduction, and core invocation for CLI, TUI, Web, and the
daily cockpit. Ownership, eligibility, schedule, and player-rate inputs carry
source IDs, observed/fetched timestamps, freshness, and a fingerprint. Missing
or stale required evidence produces the shared typed readiness and recovery
rows from `fantasy_today.v2`; complete-week schedule and complete league
ownership are required for a firm sequence.

## Performance boundary

Initial targets, to be measured on the local 16-team fixture:

- core planning: warm p95 below 250 ms for 12 add candidates, 6 drop candidates,
  4 moves, and beam width 64;
- full cached CLI assembly: warm p95 below 2 seconds;
- no network I/O on default reads;
- hard caps on candidates, drop candidates, depth, and beam width;
- timeout produces the best complete state already evaluated with
  `truncated=true`, never an empty fabricated answer.

These are targets until recorded. Complexity is disclosed as bounded beam
search rather than claimed exact global optimality.

Ordering uses validated finite values with Rust `total_cmp`, then fewer moves,
then the canonical tuple of `(effective_at, ordinal, add_player_key,
drop_player_key)`. Elapsed time and generation time are excluded from the
material fingerprint.

## Delivery slices

1. **Plan and role review** — freeze identities, objective, persistence,
   degradation, surface parity, and performance bounds.
2. **Core contract** — pure player/state types, daily recomputation, bounded
   sequence search, deterministic fingerprint, no-move baseline, and L0 tests.
3. **Assembly and persistence** — add one `icelines-fetch` assembly service
   shared by every surface, factor CLI-private weekly simulation into it, add
   migration 020 and typed journal repository methods,
   preserve read-only planner behavior.
4. **CLI vertical slice** — `week-plan`, `decision-record`, and
   `decision-review` with help/docs and L2 tests.
5. **Today integration** — expose the first pending move and week-plan
   fingerprint through the shared daily decision service.
6. **TUI/Web parity** — shared projection, 80/120-column fences, bookmarkable
   HTML/JSON, accessibility and no-JS degradation tests.
7. **Performance and closeout** — fixture benchmarks, role re-review, complete
   workspace gates, docs, commit, PR, and merge.

Every implementation commit must compile and pass its focused tests.

## Verification matrix

- **L0**: no-move wins; one/two/four moves; reserve; reserve release; exact D
  substitution; waiver clears before/after game; locked drop; duplicate add;
  negative churn; fallback re-simulation; deterministic ties/fingerprint;
  category posture missing; GP=0; truncation; NaN/Infinity rejection; same-time
  ordinals; DST and Sunday rollover; open slot and occupied IR. Property tests
  assert legality after every sequence prefix and monotonically consumed budget.
- **L1**: migration idempotence; immutable projection bytes; duplicate record;
  correction chain; complete local league assembly; missing schedule/eligibility/
  ownership/status; database unchanged by planner read.
- **L2**: help; 80-column text; JSON schema; actionable missing-input errors;
  explicit mutation confirmation; review with and without outcomes.
- **Parity**: one sealed core-owned fixture is consumed by CLI, TUI, and Web and
  projects identical primary moves, reserve,
  alternatives, daily coverage, readiness, and fingerprint in CLI/TUI/Web.
- **Performance**: cold/warm p50/p95, evaluated states, configured caps,
  truncation behavior, and database read-only hash proof.

## Acceptance gates

- Every recommended transition is legal at its effective time and every
  intermediate roster satisfies the active league shape.
- Sequence value comes from recomputed daily assignments after prior moves.
- The planner respects weekly budget, reserve, waiver delay, locks, ownership,
  and evidence freshness.
- The primary plan may recommend fewer moves or no move.
- Fallbacks are legal from the exact pre-move state.
- All three interactive surfaces consume one core-owned projection.
- The daily cockpit consumes the week plan rather than independently disagreeing.
- Recommendation evidence is immutable after decision capture; outcomes append.
- Missing data is explicit, no read path mutates state, and no default path
  performs network I/O.
- Measured performance meets the accepted bound or reports deterministic
  truncation with useful output.

## Non-goals

- Automatic Yahoo adds, drops, claims, lineup changes, or trades.
- Claim-success probabilities without league outcome history.
- Exact exhaustive optimization over the entire NHL free-agent pool.
- Hindsight rewriting of recommendations.
- Publishing private league data or manager rationale.

## Role-review amendment log

The 2026-09-05 twelve-role review approved the direction with three conditions,
all incorporated before implementation:

1. Exact transaction-time state now carries UTC effective/lock instants,
   ordinals, pre/post roster fingerprints, typed errors, and fallback
   re-simulation from the precise prefix.
2. The plan now freezes the finite points objective, separate goalie/category
   semantics, bounded-optimum wording, deterministic ordering, and fingerprint
   exclusions.
3. One fetch-owned assembly service, typed readiness/recovery, opaque immutable
   journal bytes, no-store Web behavior, a shared parity fixture, property
   tests, and read-only database proof are explicit acceptance requirements.

## Implementation closeout

- Core now owns deterministic bounded sequence search, daily lineup
  recomputation, reserve/waiver/lock legality, alternatives, and material
  fingerprints.
- One fetch-owned source assembler feeds Today, CLI, TUI, and Web; the old
  CLI-private sequence assembler was removed.
- Migration 020 stores private immutable decisions and append-only outcomes;
  planner and review reads open SQLite immutable/read-only.
- CLI commands, TUI summary, no-JavaScript HTML, JSON, no-store headers,
  non-Monday recovery, user documentation, and the public module inventory are
  wired and tested.
- Focused optimizer/property, journal, read-only byte-stability, CLI help, TUI,
  and Web route tests pass. Full affected-crate tests and strict clippy pass;
  bounded-search truncation remains disclosed when the configured beam is hit.
