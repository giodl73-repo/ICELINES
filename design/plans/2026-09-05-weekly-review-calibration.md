# Weekly Review and Calibration Loop

**Date**: 2026-09-05

**Status**: Implemented and validated

**Parent**: [`2026-07-18-fantasy-war-room-roadmap.md`](2026-07-18-fantasy-war-room-roadmap.md)

**Predecessor**: [`2026-09-05-weekly-operations-planner.md`](2026-09-05-weekly-operations-planner.md)

**Primary contracts**: `fantasy_decision_outcome.v1`,
`fantasy_decision_review.v1`

**Role review**:
[`../../signals/roles/check/weekly-review-calibration-roles-check-2026-09-05.md`](../../signals/roles/check/weekly-review-calibration-roles-check-2026-09-05.md)

## Outcome

Close the weekly fantasy-management loop without hindsight mutation. IceLines
will preserve the recommendation and evidence shown at decision time, append a
typed observation after the week, and produce a deterministic review that keeps
three questions separate:

1. Was the chosen action supported by the information available at the time?
2. How far did the observed value differ from the frozen projection?
3. What actually happened in the matchup and acquisition-reserve ledger?

```powershell
icelines fantasy decision-outcome-record --decision ID --lane execution --executed true
icelines fantasy decision-outcome-record --decision ID --lane active-value \
  --active-points-delta 8.25 --usable-starts-delta 2 --source manager
icelines fantasy decision-outcome-record --decision ID --lane matchup \
  --matchup-result win --user-final-points 143.5 --opponent-final-points 138
icelines fantasy decision-outcome-record --decision ID --lane reserve \
  --reserve-needed false --reserve-used false

icelines fantasy decision-review --week 2026-11-09
icelines fantasy decision-review --season 20262027 --json
```

The review is descriptive. It does not claim that unexplained error was luck,
infer causality from one result, or automatically change future model weights.

## Product boundary

- IceLines owns reusable, league-neutral outcome and review contracts. Named
  personal teams, private notes, and season narratives remain in PUCK or the
  user's local database.
- Recording an outcome is an explicit local mutation. Review commands and Web
  routes are read-only and perform no network calls.
- Manager rationale and outcome notes are private by default. Ordinary JSON,
  HTML, and terminal output omit them unless an explicit local CLI flag asks for
  private fields.
- The frozen `projection_json` remains byte-for-byte immutable. New decoders
  may interpret a supported schema but may never rewrite old evidence.
- Missing observations remain missing. They do not become zero points, a loss,
  an unused reserve, or a failed decision.

## Existing foundation reused

- `FantasyPickupSequenceView` freezes the evaluated week, selected sequence,
  alternatives, projected value, starts, readiness, evidence, and material
  fingerprint.
- SQLite migration 020 already stores immutable `fl_decisions` and append-only
  `fl_decision_outcomes`, including correction links.
- `FantasyDb::record_decision_outcome` and `list_decision_outcomes` provide the
  initial persistence seam.
- `decision-record` captures the selected alternative and exact projection.
- `decision-review` currently lists raw rows and is the compatibility surface to
  upgrade.

## Identity and time model

A review item is keyed by the stable local `decision_id`. Its frozen planning
context carries:

```text
(league_id, fantasy_team_id, stats_season, season_type,
 week_start, week_end, recommendation_fingerprint, chosen_alternative)
```

An outcome row is a new observation with:

```text
(outcome_id, decision_id, observed_at, outcome_kind,
 outcome_schema, source_kind, source_observed_at?, correction_of?)
```

The selected recommendation is recovered from `chosen_alternative`: zero is
the primary sequence and positive values index the frozen alternatives. The
decoder rejects a choice that is absent from the frozen projection.

`--week` accepts only an ISO Monday and includes decisions whose frozen
`week_start` matches exactly. `--season` matches the frozen `stats_season`, not
the wall-clock year or mutable active-league setting. Supplying both filters is
an error in v1.

## Typed outcome contract

`fantasy_decision_outcome.v1` contains:

```text
decision_id
executed: true | false | unknown
actual_active_points_delta?: finite number
actual_usable_starts_delta?: integer
matchup_result?: win | loss | tie
reserve_needed?: boolean
reserve_used?: boolean
source_kind: manager | platform_import | derived_boxscores
source_observed_at?: RFC3339 UTC timestamp
notes?: private string
```

The first implementation exposes manager-supplied outcomes. Provider imports
and deterministic boxscore derivation may write the same contract later, but
must preserve source kind, source time, completeness, and provenance.

Validation rules:

- numeric values must be finite;
- an unexecuted decision cannot claim realized add/drop value or usable starts;
- `reserve_used=true` with `reserve_needed=false` is allowed and remains visible;
- a correction must point to an outcome on the same decision and cannot correct
  itself or create a correction cycle;
- duplicate submissions with the same decision and material outcome
  fingerprint are idempotent;
- notes are stored separately from the public material payload or are stripped
  by every public projection.

Execution, active value, matchup, and reserve facts are independent typed
observation lanes. Each lane has one linear correction chain. A second child of
the same parent, a cross-lane or cross-decision correction, a missing parent, or
a cycle is rejected. The effective review composes the latest valid leaf from
each lane. Superseded rows remain visible in audit mode and are excluded from
ordinary calibration counts.

Every stored v1 envelope includes its schema, lane, source kind, completeness,
and optional source-observed time. Its material fingerprint covers normalized
public material plus decision and lane, while excluding insertion time and
private notes. Duplicate material is idempotent. Final points and matchup lanes
recorded before the frozen week ends remain explicitly provisional; execution
facts may be observed earlier.

## Review contract

`FantasyDecisionReviewView` (`fantasy_decision_review.v1`) contains:

```text
league + optional week/season filter + generated_at
summary + calibration readiness
items[] ordered by frozen week, evaluated_at, decision_id
warnings[]
```

Each item contains:

```text
decision identity + frozen context
selected recommendation id
projected active-points delta
projected net-value delta
projected usable-starts delta
readiness at decision time
execution state
effective outcome summary
active-points error?: actual - projected active points
usable-starts error?: actual - projected usable starts
process assessment
result assessment
projection assessment
audit counts + recovery message
```

The three assessments are independent:

- **Process**: `supported`, `unsupported`, or `insufficient_evidence`. It uses
  only frozen, pre-outcome facts: the selected sequence's projected value,
  legality/firmness, and decision-time readiness. Outcome values cannot change
  this label.
- **Result**: `positive`, `neutral`, `negative`, or `unknown`. It uses observed
  active-points delta first; otherwise it reports the explicit matchup result
  without pretending the matchup was caused by this decision.
- **Projection**: `aligned`, `above`, `below`, or `unknown`. For v1, aligned
  means the absolute active-points error is no more than the serialized
  `display_alignment_tolerance` of 1.0 fantasy point. This is an operational
  display band, not a confidence interval, probability, or calibration claim.

The UI may say “supported process, negative result, below projection.” It may
not say “bad luck” because v1 has no causal random-effects model.

## Calibration summary

Calibration groups only comparable, effective observations by:

```text
(decision kind, projection schema, competition mode, stats season,
 decision lane)
```

The frozen selected sequence determines `decision_lane` as `no_move`,
`skater_only`, `goalie_only`, or `mixed`; unsupported projections remain
`unknown` and are not pooled. Goalie and skater decisions are never combined.

For each group, calculate from rows with both projected and observed active
points:

- sample count;
- mean signed error (observed minus projected);
- mean absolute error;
- root mean square error;
- aligned/above/below counts;
- execution and outcome completeness counts.

All calculations use full finite precision and serialize deterministic rounded
display values separately. Signed error is `sum(actual - projected) / n`, MAE
is `sum(abs(actual - projected)) / n`, and RMSE is
`sqrt(sum((actual - projected)^2) / n)`. Metrics are absent when `n=0`. A group
is `descriptive_ready` at five comparable observations and remains
`retuning_blocked` in v1. Below five, metrics may be shown with
`insufficient_sample`; they cannot drive recommendations.

No global statistic combines incompatible scoring schemes, competition modes,
seasons, or projection schemas. No calibration changes planner weights in this
phase.

## Source and trust contract

- Frozen projections are first-party IceLines evidence with their original
  evaluated time and fingerprint.
- Manager outcomes are explicit assertions, not official platform facts.
- Future platform imports must preserve provider snapshot IDs and capture times.
- Future boxscore derivation must identify the exact finalized game set and
  distinguish scheduled games from fantasy-usable starts.
- Partial evidence lowers completeness. It never becomes a complete outcome.
- Review rendering always names the outcome source and observation time.

## Architecture

### `icelines-core`

Own pure serializable outcome/review types, validation, correction reduction,
selected-sequence decoding, assessments, deterministic ordering, and calibration
math. Core receives data; it performs no SQLite, filesystem, clock, or network
I/O.

### `icelines-fetch`

Own persistence DTO conversion and a single `fantasy_decision_review_service`
that reads decisions/outcomes, preserves opaque unsupported projections, invokes
core, and returns the shared view. Strengthen persistence with material
fingerprints and correction-integrity checks through an additive migration if
needed.

### `icelines-cli`

Own argument parsing and terminal rendering only:

- `decision-outcome-record` validates CLI input through the shared core
  contract before appending locally;
- `decision-review --week/--season` consumes the shared service;
- `--include-private` remains opt-in and local;
- `--legacy-json` temporarily preserves the existing unversioned raw array for
  scripts while default `--json` advances to the versioned contract;
- text leads with process/result/projection, then quantitative evidence and
  recovery.

### TUI

The Fantasy Today screen adds a compact “Last review” section sourced on screen
entry or explicit refresh from the shared review service. It never maintains a
second computed projection. At 80 columns it shows context, primary process
assessment, result/projection labels, and recovery; at 120 it adds quantitative
error and source detail. It shows no private notes.

### Web

Add bookmarkable read-only routes:

```text
GET /fantasy/decision-review?league=&week=
GET /api/v1/fantasy/decision-review?league=&week=
```

Both routes also accept sticky `season=`. Week and season together are rejected
with a typed 400 and recovery links.

They use semantic HTML, work without JavaScript, return `Cache-Control:
no-store`, omit private fields, and provide the exact local CLI recovery command.
No outcome mutation is exposed over HTTP in this phase.

The static site remains out of scope because these are private, changing league
records.

## Compatibility and degradation

- Existing migration-020 databases open without destructive changes.
- Existing raw outcome kinds remain readable as audit rows. Unsupported payloads
  produce `unknown` assessments plus a warning rather than failing the complete
  review.
- Existing `decision-review --limit --include-private` arguments retain their
  meaning. Default `--json` advances from an unversioned raw array to
  `fantasy_decision_review.v1`; `--legacy-json` preserves the old array during
  the documented transition and cannot be combined with new filters.
- A missing database, league, decision, projection decoder, outcome, source time,
  or comparable sample has a typed state and actionable recovery.
- Review reads use the immutable/read-only database path and are proven not to
  alter the DB, WAL, or SHM files.

## Performance boundary

The service performs one bounded league decision query and one bounded outcome
query rather than an N+1 query per decision. Default and maximum review limits
are explicit. Sorting and aggregation are `O(D + O + D log D)` for decisions
`D` and outcomes `O`.

Targets, to be measured rather than assumed:

- 20-decision cached CLI review p95 below 100 ms;
- 500-decision season review p95 below 500 ms;
- Web response below 500 ms on the same local fixture;
- no network I/O and no write lock on review paths.

## Delivery slices

1. **Plan and role review** — freeze semantics, privacy, correction reduction,
   calibration gates, surface behavior, and compatibility.
2. **Core contract** — typed outcome/review schemas, validation, projection
   decoder, assessments, aggregation, deterministic fixtures, and L0 tests.
3. **Persistence and service** — batched reads, idempotent material fingerprint,
   correction integrity, shared read-only service, migration tests, and byte
   stability proof.
4. **CLI vertical slice** — `decision-outcome-record`, filtered versioned
   `decision-review`, help, docs, and L2 tests.
5. **TUI and Web parity** — compact TUI review, HTML/JSON routes, privacy,
   accessibility, no-store, 80/120-column, and route tests.
6. **Closeout** — performance evidence, surface parity fixture, role re-review,
   workspace format/clippy/tests, roadmap/index updates.

Every implementation commit must compile and pass its focused tests.

## Verification matrix

- **L0**: primary/fallback selection; supported/unsupported/insufficient process;
  positive/neutral/negative/unknown result; exact ±1.0 projection boundary;
  missing values; NaN/infinity; unexecuted contradiction; deterministic order;
  five-observation readiness boundary; signed/absolute/RMS hand calculations;
  incompatible-group separation; correction-chain reduction/cycle rejection;
  unsupported projection schema.
- **L1**: migration idempotence; decision ownership; idempotent duplicate outcome;
  cross-decision correction rejection; append-only correction; batch query;
  opaque old projection; private-note suppression; read-only byte/WAL/SHM proof.
- **L2**: help; record confirmation; bad decision ID; filters; non-Monday recovery;
  season mismatch; text at 80 columns; versioned JSON; include-private opt-in;
  no outcome and corrected-outcome review.
- **Parity**: one neutral synthetic fixture produces the same item assessments,
  metrics, warnings, and effective outcome in CLI, TUI, HTML, and JSON.
- **Performance**: warm p50/p95 for 20 and 500 decisions, bounded query count,
  and no-network proof.

## Acceptance gates

- A recorded decision and its frozen projection are never updated or regenerated.
- Outcome corrections append and retain an auditable, acyclic chain.
- Process assessment is invariant when outcomes change.
- Missing observations remain unknown; unsupported old rows do not poison other
  review items.
- Calibration combines only compatible decisions and never retunes v1 weights.
- CLI, TUI, and Web consume one core-owned review projection.
- Private rationale and notes are absent from default and all Web output.
- Review reads perform no writes or network requests.
- Every warning names an actionable recovery when one exists.

## Non-goals

- Automatic Yahoo roster or transaction mutations.
- Automatic claims that a result was caused by luck or manager skill.
- Automatic planner-weight changes from a small or mixed sample.
- Publishing private rationale, notes, league records, or personal-team fixtures.
- Reconstructing official Yahoo matchup scoring before provider sync exists.
- Rewriting old opaque projection JSON into a new schema.

## Role-review amendment log

The 2026-09-05 twelve-role review approved the direction with three conditions,
all incorporated before implementation:

1. Outcomes now use independent typed lanes with source/completeness metadata,
   normalized material fingerprints, and linear per-lane correction chains.
2. The plan now serializes its display-only tolerance, defines metric formulas,
   separates goalie/skater/mixed/no-move cohorts, and hard-blocks automatic
   retuning.
3. One shared service owns all projections; legacy JSON has an explicit bridge,
   Web filters are sticky and no-store, the TUI has an 80/120 hierarchy, and
   privacy is verified negatively on every surface.

## Implementation closeout

The 2026-09-05 vertical slice is complete across `icelines-core`,
`icelines-fetch`, `icelines-cli`, and `icelines-web`. Migration 021 is additive
and leaves legacy opaque outcomes readable. Final active-value and matchup
observations are rejected until the frozen local week has ended; provisional
observations remain visible but do not enter calibration. Focused contract,
persistence, CLI, TUI, and Web tests are retained beside the implementation.

The role-review conditions were rechecked against the implementation. No P1
issue was introduced: immutable evidence, independent lanes, linear correction
reduction, private-field suppression, descriptive-only calibration, shared
surface projection, sticky filters, no-store responses, and compact/expanded
TUI hierarchy are all represented in code and tests.
