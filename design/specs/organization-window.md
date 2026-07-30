# The Window — Organization Health and Competitive Window

**Status:** Implemented for evaluation — production source and scored future-holdout gates open; future test preregistered
**Date:** 2026-07-27 (consolidated 2026-07-28)
**Owner:** IceCast organization intelligence
**Surface coverage:** CLI implemented · TUI implemented · Web/API implemented · JSON implemented · Card implemented
**Plan:** [`../plans/2026-07-27-organization-window.md`](../plans/2026-07-27-organization-window.md)
**Review:** [`../notes/2026-07-27-organization-window-roles-review.md`](../notes/2026-07-27-organization-window-roles-review.md)

## Purpose

The Window turns IceLines' team, player, lineup, prospect, affiliate,
management, schedule, injury, transaction, fantasy, and simulation authorities
into an explainable 32-organization view of competitive health.

It answers five different questions without pretending they are one:

1. How strong is the NHL team now?
2. How high or low is its plausible near-term range?
3. How sustainable is that strength over one, three, and five years?
4. Which organizational systems are creating or consuming value?
5. Which assumptions, breakouts, injuries, trades, or data gaps could change the
   conclusion?

The product name is **The Window**. The first machine contract is
`organization_window_board.v1`.

## Non-goals

- The Window is not a disguised Cup-probability model. IceCast owns game,
  standings, playoff, and Cup probabilities.
- It is not a single unexplained power ranking.
- It does not infer salary-cap flexibility from contract expiry alone.
- It does not turn podcast, rumor, or qualitative management research directly
  into numeric truth without an independently specified calibration method.
- It does not treat unavailable shift, injury, cap, or deployment evidence as a
  zero.
- It does not let renderers recompute weights, percentiles, classifications, or
  deltas.
- It is not an arbitrary runtime-code plugin system. Extensions enter through
  versioned typed providers and declarative manifests.

## Product vocabulary

| Product term | Meaning |
|---|---|
| **The Window** | The complete organization board and team detail report. |
| **Pane** | One top-level dimension of organizational health. |
| **Line** | One reusable, independently versioned profile observation feeding a pane. |
| **Frame** | The frozen scoring manifest: included lines, weights, gates, horizons, and cohort. |
| **View** | A named interpretation such as `balanced`, `win_now`, `sustainable`, or `rebuild`. |
| **The Shift** | Movement between comparable Window checkpoints. |
| **The Insider** | Evidence, methodology, limitations, and scenario explanation. |

The schema uses neutral names (`dimension`, `profile`, `manifest`) so product
language can evolve without changing data contracts.

## Required separation of concerns

```text
source authorities / sealed IceLines artifacts
                    |
                    v
typed ProfileProvider implementations in icelines-core
                    |
                    v
organization_profile_observation.v1
                    |
                    v
validated organization_window_manifest.v1
                    |
                    v
hierarchical scorer + league normalization
                    |
                    v
organization_window_board.v1
                    |
          +---------+----------+
          |                    |
          v                    v
organization_window_history.v1  organization_window_scenario.v1
          |                    |
          +---------+----------+
                    v
CardDocumentView / CLI / TUI / Web / JSON / durable artifacts
```

`icelines-core` owns profile semantics, normalization, aggregation,
classification, comparison, and validation. `icelines-fetch` assembles source
authorities and saved artifacts. CLI and Web remain thin adapters. No renderer
may derive hockey values.

## Contract family

### `organization_profile_observation.v1`

Every reusable line produces the same envelope:

```text
schema
profile_key
method_version
organization
organization_identity_version
season
season_type
as_of
horizon
signal_family
direction
raw_value + raw_unit
normalized_score       # 0..100 within the frozen comparison cohort
league_percentile      # 0..100; tied values share a documented rank policy
league_rank
sample_size
confidence             # 0..1; epistemic confidence, never visual certainty
coverage               # 0..1; fraction of required evidence represented
status                 # observed | modeled | provisional | blocked | not_applicable
previous_comparable
delta
trend
evidence[]
limitations[]
source_fingerprints[]
```

Profile observations are facts about one profile method, not mini-composites.
They must retain the raw metric and unit that produced the normalized score.
Organization identity is a typed, season-aware catalog reference rather than a
free-form abbreviation. Upstream joins must reconcile that identity, season,
season type, and as-of boundary before scoring.

### `organization_window_manifest.v1`

The Frame is a sealed, declarative scoring policy:

```text
schema
manifest_id
label
description
manifest_version
comparison_cohort
normalization_method
horizons[]
dimensions[]
  key
  weight
  minimum_coverage
  rank_required
  profiles[]
    profile_key
    method_version
    weight
    required
    signal_family
  signal_family_caps[]
    signal_family
    maximum_weight
missing_policy
classification_method
created_at
fingerprint
```

Rules:

- A profile key and method version pair is immutable.
- Any formula, source-authority, normalization, horizon, or material threshold
  change creates a new method version.
- Weight-only customization creates a new manifest fingerprint, not a new
  profile method.
- Profile weights sum to 1 within a dimension; dimension weights sum to 1
  within a view, within a small numeric tolerance.
- The configured profile weights assigned to one signal family cannot exceed
  that family's declared maximum weight. Validation rejects the Frame instead
  of clipping contributions at runtime.
- Unknown profile keys, duplicate keys, cycles, negative weights, invalid
  horizons, non-finite numbers, all-zero budgets, and invalid family caps fail
  closed.
- The manifest freezes the full expected profile set. A user cannot improve a
  team score by silently omitting an unfavorable line.

### `organization_window_board.v1`

One document contains the full comparison cohort before any team focus:

```text
schema
season
season_type
as_of
generated_at
manifest
model_versions[]
source_fingerprints[]
league_coverage
expected_organizations
organizations[]
  organization
  overall
    score
    confidence
    coverage
    percentile
    rank
    rank_status
    classification
    trend
    comparable_delta
    scenario_range
  horizons[]
  dimensions[]
  profile_observations[]
  strengths[]
  vulnerabilities[]
  blockers[]
  evidence_summary
fingerprint
```

Team filtering happens only after the complete board is built and
fingerprinted. A focused Rangers or Kraken report retains the league-board
fingerprint and original rank.

The current-season NHL catalog requires exactly 32 organizations. Historical
replay requires the complete canonical organization catalog for that season,
which may contain fewer teams. The expected count and catalog version are
sealed in the board; “all 32” is a current-league product promise, not a false
historical invariant.

### History and scenarios

- `organization_window_history.v1` compares two or more boards only when team
  identity, season semantics, manifest fingerprint, method versions, and
  normalization cohort are comparable. Otherwise it emits an explicit bridge
  requirement instead of a false delta.
- `organization_window_scenario.v1` references one baseline board and one or
  more sealed IceLines scenario artifacts. It reports isolated and combined
  changes without overwriting observed history.
- If a profile method is upgraded, a separately versioned rebase/bridge
  artifact may recompute prior checkpoints. Raw scores from unlike methods are
  never subtracted directly.
- `organization_window_bridge.v1` seals the source and target manifest
  fingerprints and a complete one-to-one profile mapping. Each mapping carries
  a finite affine raw-value transform, rationale, and evidence fingerprints.
  Rebase reruns canonical normalization and aggregation; it never applies an
  aggregate-score correction. Bridged movement reports observed-input,
  method/manifest, and residual components separately.

## Initial pane model

The final profile inventory is a discovery deliverable. The initial pane
model prevents today's implemented producers from dictating the permanent
architecture.

| Pane | Question | Candidate IceLines authorities | Important boundary |
|---|---|---|---|
| NHL strength | How competitive is the current NHL roster? | team strength, depth, position groups, goalies, special teams, lineup forecast | Separate descriptive roster quality from IceCast outcome probability. |
| Sustainability | Can current performance persist? | age/experience curves, player projection confidence, contract horizon, durability | Cap claims remain blocked until verified cap authority exists. |
| Pipeline | Is future NHL value arriving? | prospect board, conversion performance, training camp, hidden risers | Prospect strength and prospect conversion are separate signals. |
| Development system | Does the organization create usable players? | AHL lines, opportunity, recalls, NHL/AHL fit, conversion history | Do not reward a weak NHL roster merely for creating open jobs. |
| Deployment | Is available talent being used effectively? | line combinations, special teams, matchup plans, adaptive bench policy | Shift-derived chemistry remains blocked while shifts are locked. |
| Management | Does decision behavior support the window? | GM/manager research markers, roster tendencies, transaction outcomes | Qualitative research is context-only until outcome-calibrated. |
| Flexibility | Can the organization retain or add talent? | contracts, waivers, roster optionality, trade assets, cap authority | No numeric cap pane from expiry type alone. |
| Resilience | How exposed is the team to shocks? | injury concentration, goalie dependency, depth ladders, schedule/fatigue | Missing injury data is unknown, not healthy. |

The default production Frame may include only panes that clear source,
coverage, and calibration gates. Blocked panes remain visible in the board's
coverage report; they are never silently removed.

The Window exposes three related but non-interchangeable composite products:

- **organization health** is a descriptive checkpoint score assembled from a
  named Frame;
- **competitive success** is a target- and horizon-specific forecast that needs
  its own calibration evidence; and
- **window timing** is a separately versioned classification over current and
  future-horizon evidence.

A health score or league percentile is never relabeled as a Cup probability.
A contender/rebuild label names its classification method and horizon rather
than being inferred by a renderer from one number.

## Profile registry and extension contract

IceLines maintains a registry of `ProfileDescriptor` records. A descriptor
declares:

- stable `profile_key` and `method_version`;
- input artifact schemas and accepted versions;
- organization, season, season-type, as-of, and horizon axes;
- raw unit, score direction, normalization method, and comparison cohort;
- signal family and dependency declarations;
- minimum sample and evidence gates;
- whether the profile can be ranked, can be scenario-adjusted, and can be
  compared historically;
- known limitations and blocked claims.

Providers accept typed, already assembled inputs and remain pure. Manifest and
artifact file parsing, source fetch, and cache I/O stay outside core. A provider
may depend on other sealed observations, but its descriptor must declare those
dependencies and the registry must reject cycles.

Adding a profile requires:

1. a typed provider in core;
2. a registry descriptor;
3. known-value and failure-path tests;
4. source-authority and calibration documentation;
5. a manifest opt-in; and
6. parity fixtures before surface promotion.

Changing weights or enabling an already registered profile requires only a new
manifest. Changing hockey logic requires a new method version. Removing a
profile from a later Frame does not invalidate older saved boards because each
board embeds its manifest and fingerprints.

Profiles may consume existing sealed documents through adapters. They may not
reach into renderer output, scrape terminal text, or duplicate another
producer's formula.

### Registry lifecycle amendment

Evidence readiness, observation status, and registry lifecycle are separate
axes. The sealed `organization_window_registry_lifecycle.v1` amendment adds
lifecycle metadata over the immutable v1 descriptor inventory with these
semantics:

- `active`: eligible for newly authored Frames, subject to readiness and Frame
  gates;
- `deprecated`: still readable and replayable, but a new official Frame must
  select the declared replacement or record an explicit hold rationale; and
- `retired`: replayable only through an already sealed artifact/Frame and not
  selectable by a newly authored Frame.

Supersession points from one immutable `profile_key@method_version` to another.
It is not an alias: loading an old board never substitutes the replacement.
Demoting source readiness does not retire a method, and retiring a method does
not rewrite its historical readiness. A lifecycle amendment records rationale,
effective date, affected official Frames, replacement when present, and review
evidence. A deprecated-method hold is part of that sealed amendment and names
the exact official Frame ID and manifest fingerprint, rationale, approver, and
review date; an unsealed runtime exception is not authority.

The amendment is separate from `organization_window_registry.v1`, so replaying
an old board continues to use its embedded Frame and original descriptors.
New official builders and custom rebases seal and bind the lifecycle
fingerprint. Retired methods fail new authoring; deprecated methods require an
explicit reviewed hold in an official Frame; readiness overrides may only
demote. A production official Frame may select only methods whose effective
readiness is `ready_for_adapter`; an IceLines evaluation Frame may also select
`evaluation` methods, and a custom Frame may inspect evaluation/context
methods. No new Frame may select a blocked method, and both production and
evaluation IceLines Frames require sealed deprecated-method holds. The validator rejects
unknown/self/retired/cyclic replacements and never substitutes a replacement
into a saved artifact.

## Normalization and scoring

### Frozen league cohort

Every ranked run uses the complete season-aware set of 32 organizations.
Expansion, relocation, and historical replay use the canonical team catalog
for that season. An incomplete cohort can produce an evaluation artifact but
cannot publish league ranks.

Profile-level normalization also requires the descriptor's declared cohort
gate. If only 26 of 32 current organizations have eligible observations, that
profile may remain provisional but cannot publish a misleading 26-team league
percentile unless the method explicitly defines and labels that cohort.

### Profile normalization

Each method declares one normalization policy. The first supported policies
should be:

- empirical percentile with deterministic tied ranks;
- robust z-score mapped to 0..100 with a frozen median and scale; and
- calibrated probability or rate mapped by an explicit domain function.

Direction is declared (`higher_is_better`, `lower_is_better`, or
`target_range`). Winsorization, transforms, and target bands are method-owned
and versioned. Renderers never infer them.

Degenerate cohorts are explicit. When every eligible raw value is equal,
empirical-percentile methods emit the neutral score 50, a shared tied rank, and
a `no_between_team_variance` limitation. Fewer than the method's minimum cohort,
no eligible observations, or a non-finite transformed value blocks
normalization.

### Hierarchical aggregation

The Window aggregates profiles into panes, then panes into a view. It never
places dozens of correlated raw lines into one flat weighted sum.

For eligible observations in one pane:

```text
pane_score = sum(profile_score * configured_weight) / sum(eligible_weight)
weight_coverage = sum(eligible_weight) / sum(configured_weight)
pane_confidence = weighted_mean(profile_confidence) * weight_coverage
```

The same structure aggregates panes into the overall view. Signal-family caps
prevent multiple variants of one underlying fact from consuming more than the
declared budget: manifest validation requires the sum of configured weights in
each family to be less than or equal to its cap. Missing observations do not
become zero. They reduce coverage and confidence.

A score may be shown as provisional below a pane's minimum coverage. League
rank is withheld if:

- any rank-required profile is blocked;
- minimum pane or overall coverage is not met;
- the cohort is incomplete;
- manifest or method versions differ across organizations; or
- source freshness violates the profile gate.

The rank status must state the reason. Renormalization across available inputs
is never presented as fully comparable when material configured weight is
missing.

Board presentation order is also rank-gated. Core supplies one display-order
projection to every renderer: rows with official ranks use rank order, while
rank-withheld rows use canonical organization order. A renderer must not sort
`NR` rows by partial score, because that would create an unlabeled shadow rank.

Classification publication uses the same core gate. A sealed evaluation board
may retain the raw descriptive classification produced from its available
panes for replay compatibility, but `published_classification()` withholds it
unless the row is officially ranked. Cards and renderers display `Under review`
in that state and must not reproduce this decision locally.

### Default views

The first release should ship small, reviewable presets rather than one
universal truth:

- `balanced`: present quality plus sustainable three-year health;
- `win_now`: current NHL strength, resilience, and near-term flexibility;
- `sustainable`: pipeline, conversion, development, age curve, and flexibility;
- `rebuild`: asset growth, opportunity quality, pipeline depth, and conversion.

Presets share profile observations. Only their manifest weights and gates
differ. User manifests may alter weights, but IceLines must label custom ranks
with the manifest name and fingerprint.

Each Frame declares one primary decision horizon. A multi-horizon board embeds
separately fingerprinted Frame results rather than applying one set of weights
indiscriminately to present, three-year, and five-year questions.

## Confidence, coverage, and evidence

Score, confidence, and coverage are independent fields:

- **score**: what the available evidence estimates;
- **confidence**: reliability of those estimates given sample size,
  calibration, and method uncertainty;
- **coverage**: how much of the declared Frame has usable evidence.

Every evidence item includes source kind, source identifier or URL when
available, captured-at/as-of timestamps, freshness state, and the profile
input it supports. Saved upstream IceLines artifacts retain their own
fingerprints.

Context-only inputs may explain a score but cannot change it. Modeled inputs
may affect it only when the descriptor names the model and validation status.
Blocked inputs remain visible in `blockers`.

## Comparable movement

The Shift reports:

- absolute score change;
- percentile and rank movement;
- observed-data contribution;
- roster/personnel contribution;
- changed coverage or confidence;
- model/manifest changes; and
- residual revaluation.

Year-over-year movement uses a consistent as-of convention and matched season
phase. July 27 preseason cannot be compared naively with March 5 post-deadline.
When no matched checkpoint exists, the board says `not_comparable`.

## Scenario sensitivity

The Window consumes, rather than replaces, IceCast and lineup scenarios.
Supported scenario families include:

- player breakout or downturn;
- injury/return and goalie availability;
- prospect arrival or failed arrival;
- trade completion and outgoing cost;
- line, special-team, or deployment change;
- manager/GM policy change; and
- schedule/fatigue stress.

Every scenario reports the changed profiles and panes, the unchanged profiles,
baseline and scenario fingerprints, and isolated versus combined deltas.
Scenario ranges are distributions when the upstream artifact is stochastic;
they are not displayed as symmetric error bars unless the distribution supports
that interpretation.

Typed scenario authorities identify their source schema/fingerprint,
organization scope, affected profile methods, kind, and rationale. First-party
adapters cover team-season trade, injury/return, goalie and form/development
events, training-camp league forecasts, and line-combination candidates.
Direct raw/evidence changes require an authority matching the organization and
profile. Normalized-only movement may be attributed to a declared league-cohort
effect for the same profile. An overall change with no profile change or any
unattributed changed profile is rejected.

## Classifications

Classifications are derived from multiple axes and remain secondary to scores:

- contender;
- rising contender;
- fragile contender;
- plateau;
- retooling;
- rebuilding; and
- evaluation incomplete.

The classification method is versioned. It must state the rule that separates
current strength from sustainability; a high present score with weak pipeline,
resilience, or flexibility should be eligible for `fragile contender`, not
silently averaged into the middle.

## Surface behavior

### CLI

Planned command family:

```text
icelines icecast window [--season] [--as-of] [--view balanced] [--manifest file]
icelines icecast window-team NYR [same context flags]
icelines icecast window-movement --earlier <file> --later <file> [--bridge <file>]
icelines icecast window-rebase --input <file> --target-manifest <file> --bridge <file>
icelines icecast window-history --input <file> --input <file> ...
icelines icecast window-scenario --baseline <file> --scenario <file> ...
icelines icecast window-explain [--profile <key>] [--organization NYR]
```

Text output leads with rank status, coverage, confidence, panes, and primary
drivers. `--json --out` preserves the complete contract.

### TUI

The league view is a compact 32-row board. Team detail shows overall state,
horizons, panes, strongest/weakest lines, coverage, and scenario range. A
drilldown opens The Insider evidence and methodology. Narrow terminals favor
one pane at a time; no horizontal wall of dozens of profile columns.

### Web/API

Planned routes:

```text
/icecast/window
/icecast/window/:team
/api/v1/icecast/window
/api/v1/icecast/window/:team
```

Season, as-of, view, and manifest fingerprint remain visible and bookmarkable.
HTML, fragments, and JSON consume the same board. Tables remain semantic and
horizontally contained on narrow screens.

### Cards and artifacts

The 32-team board is a UI-neutral ViewModel. A focused team can be projected
into `card_document.v1` only after the full board is sealed. Page one is the
scoreboard; page two is The Insider analysis. Cards retain the board,
manifest, and source fingerprints. The Insider projects the two leading and
two lowest non-overlapping available panes as shared metric strips. A
rank-withheld board labels these as available-pane observations; only an
officially ranked board may call them strengths and vulnerabilities.
Renderers resolve every Insider provenance reference against the card's sealed
source rows and expose its authority kind, completeness, observation time,
fingerprint, and note; a renderer must not replace these with local source
claims.

## Data and cache identity

The Window cache/artifact key includes:

```text
season
season_type
as_of
historical/live mode
manifest fingerprint
profile registry version
upstream source/artifact fingerprints
team-catalog version
```

Fingerprints use a documented canonical serialization: stable field ordering,
UTF-8, explicit enum strings, finite decimal values, normalized negative zero,
and no NaN or infinity. Fingerprints never depend on Rust `HashMap` iteration,
locale formatting, filesystem path, or renderer output.

`StatsRepository` remains keyed by `(player_id, season, season_type)` and is not
mutated to store organization composites. Long-lived TUI caches must invalidate
when any key axis changes. CLI and Web handlers remain one-shot.

A loaded board is not trusted solely because its content hash matches. The
wire-boundary validator seals the embedded manifest, verifies exact cohort and
profile structure, validates numeric and source-evidence invariants, and
replays raw observations through the canonical scorer. Stored normalized
scores, pane/overall aggregates, classifications, drivers, blockers, and rank
state must match the replay within the declared floating-point tolerance.
CLI/card/Web/TUI projections, comparisons, rebases, and calibration all use
this same core gate.

`organization_window_source_package.v1` is the portable upstream handoff for
`balanced.v1`. It embeds typed source documents rather than filesystem paths,
freezes season/cutoff/team-catalog axes, sorts repeated team authorities
canonically, and fingerprints the resulting package. Core revalidates all
nested authorities before every package-to-board build. Partial packages are
valid evaluation evidence; a production-ranked build additionally requires
every organization to pass the board's rank gate.

All-league assembly may resolve configured caches at the CLI boundary, but it
loads shared configuration and stores once per package build and passes owned
typed documents into core. A sealed `team_game_forecast.v1` may be embedded as
schedule authority; core derives represented-team rest/fatigue profiles from
its per-game home/away contexts. It rejects schema/season mismatch, duplicate
game identity, non-canonical opponents, and duplicate explicit/derived team
profiles. It never invents observations for teams absent from the schedule.

Sealed AHL affiliate projections may also be embedded independently of final
organization-lineup documents. Core joins them to matching NHL lineup
authorities through the existing `organization_lineup_forecast.v1` builder,
preserving its affiliation, season, development-rule, complete-lineup, and
cross-level identity gates. Missing counterparts remain missing; supplying an
explicit organization lineup for a derived team is rejected as competing
authority. Provider-local AHL IDs never become canonical player IDs without a
reviewed crosswalk.

Prior-season AHL player value is a separate, versioned evaluation authority.
`ahl_player_value_policy.v1` produces confidence-weighted within-position
ordering from official AHL skater points/game or goalie save percentage. The
skater prior is position-specific and workload confidence is game-based; the
goalie prior is shot-based. The method is not an NHL equivalency and remains
uncalibrated until rolling historical replay validates its priors. Its sealed
league ledger joins only through reviewed canonical NHL identities, aggregates
multi-team stints, and can fill only a missing `projected_score` blocker. A
missing statistical row or position-group conflict remains explicit; score
application cannot clear assignment, status, waiver, prospect, recall, game,
or development-rule authority.

`organization_window_source_coverage.v1` audits acquisition independently from
scoring. For all 17 configured profile methods it records source-observation
count, score-eligible value count, exact missing organizations, and required
status, then reports required-profile and rank-eligible-team totals. Board
league coverage must not substitute for this report.

Cache-built NHL lineups use typed `SeasonStats.time_on_ice` evidence for
special-teams roles. The fetch boundary joins official
`skater/timeonice` rows by NHL player ID and preserves DI-09 semantics: a
missing report or player row is missing evidence, while a present row with
zero seconds is a real zero. The adapter rounds fractional per-game seconds to
the nearest whole second and retains exact aggregate seconds and shift rates.

## Calibration and validation

Production promotion requires rolling-origin historical evaluation. The
composite must publish separate evidence for distinct targets rather than tune
one score against everything:

- current-strength panes against next-season points and playoff advancement;
- sustainability panes against three-year competitive results;
- pipeline/development panes against later NHL workload and position-normalized
  performance;
- resilience panes against downside frequency and concentration of missed
  value; and
- flexibility panes against verified retention/addition capacity once cap
  authority exists.

Backtests freeze every input at the historical as-of boundary. Future stats,
future rosters, later injuries, actual playoff results, and retrospective
prospect labels are leakage. IceLines reports Brier/log loss only for genuinely
probabilistic targets and rank correlation/error for continuous targets.

The default Frame is not promoted merely because it tells a plausible hockey
story. It must beat named simple baselines or remain labeled heuristic.

A rolling-origin artifact exposes every origin and board fingerprint, a
baseline frozen independently for each origin, pooled and per-origin metrics,
leave-one-pane-out ablations, organization stability, and between-origin
uncertainty. Trial noise and season variation remain separate; absent
trial-level inputs are labeled `not_provided`, deterministic origins are
`not_applicable`, and complete sealed per-origin MAE standard errors produce a
separate propagated interval. Mixed or malformed trial evidence never produces
a partial interval. Mixed Frame fingerprints,
incomplete pane scores, invalid outcome cohorts, and incomplete leakage audits
fail closed rather than being pooled.

Before a genuinely future outcome is observable,
`organization_window_future_holdout_registration.v1` seals the complete ranked
feature board, target, neutral baseline, leakage audit, outcome-eligibility
date, and acceptance rule. The closed document has no outcome or claim-status
field. Its timestamp must fall between the feature cutoff and outcome
eligibility date; the board must use the registered observed-history method and
all 32 ranks must be present. A final result is scored once after eligibility
against separately sealed official standings and is retained regardless of
whether it passes, fails, or is inconclusive.
`organization_window_future_holdout_result.v1` embeds both sealed inputs,
recomputes the single-origin calibration, refuses early standings/capture/score
dates, derives the committed acceptance result, and seals the complete replay.

`organization_window_completion_status.v1` is the final lifecycle proof. It
embeds and validates one current `organization_window_source_coverage.v1`
audit, the exact preregistered holdout, and the optional scored result. The
status is complete only when the source audit is production-ranked with zero
carry-forward observations and the bound holdout result is present. Calendar
eligibility without a result remains `holdout_eligible_unscored`. Predictive
acceptance remains an independent field and cannot change lifecycle evidence.

## Failure policy

- Unsupported schema or method version: hard error.
- Incomplete 32-team cohort: artifact allowed, league rank withheld.
- Missing required profile: score provisional or blocked per manifest; never
  zero-filled.
- Stale evidence: explicit stale status; rank withheld when the descriptor's
  freshness gate fails.
- Team identity mismatch: hard error with season-aware relocation context.
- Duplicate profile observation: hard error.
- Incomparable history: movement withheld with a bridge reason.
- Scenario without matching baseline identity: hard error.
- Custom manifest: allowed after validation and always labeled.

Saved-document compatibility is explicit:

- a reader supports the exact major schema versions it names;
- additive optional fields within a supported version use documented defaults;
- a newer unsupported major version is refused with an upgrade message;
- a migration produces a new fingerprinted artifact and never mutates the
  original sealed board; and
- profile method versions are never migrated in place.

Lifecycle changes follow the same rule: deprecation, supersession, demotion, or
retirement may change what a newly authored official Frame accepts, but cannot
change the meaning or readability of an already sealed board.

## Acceptance criteria

1. One command deterministically produces 32 organization rows from one frozen
   Frame and source set.
2. Every overall score drills into panes, profiles, raw values, evidence,
   confidence, coverage, and limitations.
3. Missing or blocked inputs cannot masquerade as poor performance or complete
   evidence.
4. Weight changes require no hockey-logic rewrite and produce a new manifest
   fingerprint.
5. Formula changes cannot alter an existing profile method version.
6. Correlated profile families cannot exceed configured contribution caps.
7. Team focus preserves the complete-board fingerprint and rank.
8. Historical deltas are emitted only for comparable methods, manifests,
   cohorts, and season phases.
9. Scenario output names every changed and unchanged profile.
10. CLI, TUI, Web/API, JSON, and cards consume core-built documents without
    recomputation.
11. Historical backtests prove point-in-time integrity and compare against
    simple baselines.
12. The released default Frame has no silent cap, shift, injury, or qualitative
    research substitution.
13. A lifecycle change cannot rewrite a sealed profile observation, Frame, or
    board, and a replacement method is never silently substituted.
14. Organization health, competitive-success forecasts, and window-timing
    classifications remain distinctly named and cannot inherit one another's
    validation claims.
15. One sealed completion-status document validates both final product gates,
    names every remaining action, and can fail automation until both are
    satisfied without treating a failed predictive threshold as missing work.
