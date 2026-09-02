# UI-Neutral Card System — Implementation Plan

**Date**: 2026-07-21
**Status**: Complete
**Specification**: [`../specs/ui-neutral-card-system.md`](../specs/ui-neutral-card-system.md)
**Archive when**: all claimed card kinds and surface-parity gates are complete,
or a named successor plan supersedes the remaining waves
**Related plans**:
[`2026-07-19-team-season-forecast.md`](2026-07-19-team-season-forecast.md),
[`2026-07-18-fantasy-war-room-roadmap.md`](2026-07-18-fantasy-war-room-roadmap.md),
and
[`2026-07-21-documentation-consolidation.md`](2026-07-21-documentation-consolidation.md)

## Outcome

Ship one versioned card document built entirely by IceLines and consumed by
CLI, TUI, web, image, PDF, and future third-party renderers. Prove the boundary
with Rangers and Kraken two-page team cards, then extend the same grammar to
fantasy and full-season simulation.

This plan does not authorize a renderer to source or calculate hockey data.
Every wave begins in core and ends with surface parity evidence.

## Current Foundation

The implementation reuses:

- Campbell platform contracts and typed ViewModels;
- Prince semantic tokens and visual review rules;
- `TeamDepthView`, team-ceiling lenses, and official headshot fields;
- IceCast game/season forecasts, scenario impacts, and realization buckets;
- development calibration v2 and current-player lookup;
- fantasy league, roster-shape, scoring, daily lineup, matchup, draft, pickup,
  goalie, trade, and simulation views;
- CLI JSON patterns, axum HTML/JSON routes, and TUI experience launchers; and
- the Rink/Insider/Depth Chart brand vocabulary.

No new parallel data loader, scoring engine, simulation engine, or scenario
format is introduced.

Fantasy and simulation outputs must be promoted into the mainline identity and
provenance streams before cards consume them. The card layer is not an
integration shortcut: stable team/player/game identities, evidence cutoffs,
calendar fingerprints, injuries, and transactions are shared upstream. A
side-by-side comparison is then a core projection over compatible documents,
not renderer-local alignment or subtraction.

## Delivery Principles

1. Core contract before renderer.
2. One serialized object for all surfaces.
3. Domain-specific builders over a small shared section grammar.
4. Missing data remains typed state, not presentation fallback.
5. Every scenario result is fingerprinted and reproducible.
6. Every player impact shown to users comes from an isolated paired run.
7. Renderer code has dependency and test fences against hockey computation.
8. Each visual artifact retains its source document ID and schema.
9. Bugs found while building cards are fixed at the authoritative layer and
   receive regression coverage before visual regeneration.
10. Documentation consolidation proceeds alongside the feature so this plan
    does not become another orphan roadmap.

## Workstream Map

| Wave | Deliverable | Primary crate/surface | Exit gate |
|---:|---|---|---|
| 0 | Authority and fixture freeze | core/fetch/docs | NYR/SEA inputs and known gaps recorded |
| 1 | Shared card primitives | `icelines-core` | schema round-trip and validation pass |
| 2 | Scenario registry and fingerprints | core/fetch/CLI | stable IDs; web cannot read arbitrary paths |
| 3 | Team score and lineup authority | core | legal 4F/3D/2G projection with evidence |
| 4 | Isolated player impact engine | core | paired deltas reconcile for every event |
| 5 | Team prognosis and comparison builders | core | NYR/SEA golden documents and aligned comparison pass |
| 6 | CLI and independent JSON | CLI/schema | text/JSON render without recomputation |
| 7 | Web card surface | web | HTML and API share one object |
| 8 | TUI card experience | CLI/TUI | 80/120/160 snapshots preserve meaning |
| 9 | Reference image/PDF renderer | tooling/report | artifacts validate against JSON |
| 10 | Fantasy card family | core/CLI/web/TUI | roster + draft/morning vertical slices |
| 11 | Simulation card | core/CLI/web/TUI | focused card fingerprints league run |
| 12 | Parity, docs, and release gate | all | matrix, captures, tests, docs complete |

## Wave 0 — Authority and Fixture Freeze

**Status**: Complete as a discovery freeze. See
[`../notes/2026-07-21-card-authority-freeze.md`](../notes/2026-07-21-card-authority-freeze.md).
The recorded CARD-001 through CARD-008 gaps remain gates in their owning waves.

### Tasks

- Inventory the exact current IceLines paths for roster, lines, player scores,
  headshots, forecast, scenario, and calibration evidence.
- Map fantasy and simulation outputs onto the shared season/team/player/game
  identities and record every remaining parallel-stream gap.
- Record the evidence cutoff and source state for the 2026-27 NYR and SEA
  showcase.
- Freeze canonical scenario IDs for baseline and development-variance runs.
- Identify missing exact line assignments, role certainty, headshots, score
  coverage, and isolated event impacts.
- Create a bug ledger with owner layer and expected regression test.
- Preserve existing user worktree changes and keep implementation commits
  separated from TRACKER submodule-pointer updates.

### Exit gate

- One input manifest per showcase team.
- No manually entered value is required by a renderer.
- Every known missing input is a typed warning or tracked bug.
- Fantasy and simulation inputs have named mainline joins; gaps have an owner
  and regression gate.

## Wave 1 — Shared Card Primitives

**Status**: Complete. The first executable slice is implemented in
`icelines-core/src/view_model/card/mod.rs` with focused and full-core regression
coverage. Asset records, metric strips, renderer capability metadata, and theme
contrast validation are also implemented. The full bounded section grammar and
checked JSON Schema are now in place.

### Core modules

```text
icelines-core/src/view_model/card/mod.rs
icelines-core/src/view_model/card/context.rs
icelines-core/src/view_model/card/section.rs
icelines-core/src/view_model/card/asset.rs
icelines-core/src/view_model/card/theme.rs
icelines-core/src/view_model/card/validate.rs
```

### Tasks

- Add `CardDocumentView`, `CardKind`, `CardPageView`, and typed section enums.
- Reuse `ViewContext`, `ViewWarning`, `EmptyState`, `SemanticToken`, metric units,
  and precision policy rather than cloning them.
- Add theme roles, asset state/fallback, accessible labels, and renderer
  capability metadata.
- Add deterministic document fingerprinting from canonical serialized inputs.
- Add schema-version refusal and required-section validation.
- Publish a checked JSON Schema artifact generated or tested against Rust
  serialization.

### Tests

- JSON round-trip and stable field names;
- duplicate page/section ID refusal;
- missing required section refusal;
- invalid metric/probability refusal;
- missing asset with deterministic fallback;
- semantic theme contrast metadata; and
- stable fingerprint with fixed clock.

### Exit gate

`card_document.v1` can be constructed, serialized, validated, and consumed
without importing team, fantasy, or simulation code.

### First executable slice

**Checkpoint**: Complete on 2026-07-21. The envelope, page, shared identity and
simulation context, theme identity, provenance, state notice, SHA-256 content
fingerprint, validation errors, public exports, and five focused tests are in
place. The full `icelines-core` suite passes.

This implementation checkpoint was deliberately smaller than the whole wave:

1. add the `card` module with envelope, page, context, provenance, and a
   `state_notice` section only;
2. reuse `ViewContext`, `ViewWarning`, `EmptyState`, `MetricCell`,
   `MetricUnit`, `ValuePrecision`, and `SemanticToken` directly;
3. add the shared join/fingerprint fields needed to identify roster snapshot,
   evidence cutoff, calendar, scoring scheme, scenario, model, seed, and
   trials without importing any domain builder; and
4. cover JSON round-trip, stable fingerprint, duplicate IDs, unsupported
   schema, and deterministic missing-state behavior.

This slice has no NYR/SEA constants, no player scoring, no lineup projection,
and no renderer. Its purpose is to prove the mainline contract boundary before
team, fantasy, or simulation adapters are allowed to depend on it.

### Second executable slice

**Checkpoint**: Complete on 2026-07-21. Added authoritative `CardAssetView`
records with external/local references, source state, observation time,
integrity, accessible text, and deterministic fallbacks. Added `metric_strip`
using the existing `MetricCell`/`MetricValue` contract, typed comparison and
evidence fields, renderer capability refusal, and text/surface contrast
validation. Eight focused card tests and all 762 core tests pass.

The third slice completes the bounded semantic section enum and checked schema
artifact. Domain builders remain out of Wave 1 so no renderer-facing primitive
can become a back door for team, fantasy, or simulation calculations.

### Third executable slice

**Checkpoint**: Complete on 2026-07-21, closing Wave 1. Added typed identity,
lineup, player-list, scenario-bridge, probability-range, decision, timeline,
methodology, and provenance sections. Core validation now refuses dangling
asset/provenance references, duplicate lineup assignments, malformed scenario
bridges and ranges, empty decisions, and duplicate semantic IDs. The checked
[`card_document.v1` JSON Schema](../schemas/card_document.v1.schema.json) is
embedded in core and tested against a document containing every section type.
Ten focused card tests and all 764 core tests pass.

## Wave 2 — Scenario Registry and Fingerprints

**Status**: Complete. Stable scenario identity now crosses core, fetch, and CLI;
interactive surfaces have no arbitrary-path input.

### First executable slice

**Checkpoint**: Complete on 2026-07-21. Added stable lowercase scenario IDs,
league/season/type/team/calendar scope, evidence labels, canonical SHA-256
content identity, imported-at/source metadata, compact references, and
cross-scope refusal in core. Added an atomic `~/.icelines/scenarios` store in
fetch with content-addressed JSON, sorted registry index, idempotent import,
immutable ID conflict refusal, integrity verification, and team-season scenario
validation. Four core and two fetch registry tests pass.

The full regression run exposed and fixed five season-rollover test bugs:
completed 2025-26 bundles were incorrectly required to equal the live 2026-27
roster season, two FLETCH assertions hard-coded the former current season, and
two team-coverage tests queried the live season instead of the newest completed
bundle. All 768 core and 378 fetch tests now pass.

### Second executable slice

**Checkpoint**: Complete on 2026-07-21, closing Wave 2. Added `icecast scenario
import|list|show`, import-ready current NYR/SEA JSON fixtures, and mutually
exclusive `icecast season --scenario-id` versus the explicit CLI-only
`--scenario PATH` experiment path. Registry resolution checks NHL league,
season type, season, team scope, and a canonical regular-season calendar
fingerprint. Forecast JSON carries both the effective scenario fingerprint and,
for registered inputs, the immutable registry reference.

The command contract and calendar hashing have focused tests. A full CLI run
also exposed and fixed a 2026-27 rollover bug: the TUI cold start and playoff
test fixtures assumed the live roster season already had a completed stats
bundle. The TUI now opens on the newest completed bundled season. Final suite
results: 768 core, 378 fetch, and 1,331 CLI unit tests pass.

### Tasks

- Add a local scenario registry with stable ID, season/team scope, schema,
  evidence label, hash, and timestamps.
- Import the current NYR/SEA scenario fixtures without changing their event
  semantics.
- Preserve an explicit CLI-only ephemeral-file path for experimentation.
- Make web/TUI resolve scenario IDs only.
- Add parameter, roster, calendar, and model fingerprints to card context.
- Refuse scenario/team/season mismatches.

### Tests

- idempotent import;
- changed content changes the hash;
- arbitrary web filesystem paths are impossible;
- cross-season misuse is refused; and
- historical and simulated evidence labels remain distinct.

## Wave 3 — Team Score and Lineup Authority

**Status**: Complete. Core owns the score, legal assignment, portraits,
warnings, card lineup conversion, and card headshot assets.

### First executable slice

**Checkpoint**: Complete on 2026-07-21. Added `icelines_player_score.v1` and
`team_lineup_projection.v1`, with position-aware multi-lens normalization,
explicit `NR`, sample/coverage/evidence metadata, multi-position eligibility,
four lines, three pairs, two goalie roles, extras, official face references,
deterministic initials, assignment authority, and typed integrity failures.
Added `icelines report team-lineup --team ABBR` as the official-roster plus
completed-production adapter; text and JSON are renderers of the same core
document.

### Second executable slice

**Checkpoint**: Complete on 2026-07-21, closing Wave 3. Added lossless
conversion from `team_lineup_projection.v1` to the generic card lineup section
and headshot asset grammar. Generated canonical NYR and SEA projection fixtures
from the installed 2026-27 official roster snapshots and 2025-26 production.
NYR contains 26/26 official faces, 25 rated players, one explicit `NR`, and Tye
Kartye on NYR. SEA contains 21/21 official faces and truthfully warns about four
open forward slots because roster authority supplies primary positions but no
wing eligibility for its surplus centers; the builder does not invent it.

Three focused authority tests, two canonical fixture tests, all 771 core unit
tests, and all 1,332 CLI unit tests pass. CLI test compilation used a single-job
build to avoid Windows paging-file exhaustion during the large test link.

### Tasks

- Define one versioned 0-100 IceLines player score for card display using the
  existing multi-lens team/depth evidence.
- Keep skater and goalie component methods position-aware while normalizing the
  final display scale.
- Separate player score from NHL projected points, fantasy points, and team
  strength.
- Project four forward lines, three defense pairs, starter/backup goalies, and
  extras through a core builder.
- Carry assignment evidence: actual, reported, estimated, or scenario.
- Join official headshot references and deterministic initials fallbacks.
- Refuse duplicate assignments and expose incomplete shape warnings.

### Required bug checks

- multi-position identity is not collapsed incorrectly;
- goalies never enter skater slots;
- traded/current-team identities agree with roster cutoff;
- missing score is `NR`, not zero;
- names and diacritics survive JSON and renderer projections; and
- headshot identity matches stable player ID.

### Exit gate

NYR and SEA Page 1 data contains faces/fallbacks, names, scores, and legal slot
assignments with no renderer logic.

## Wave 4 — Isolated Player Impact Engine

**Status:** Complete (2026-07-21).

Checkpoint: core now owns paired, same-seed one-event attribution, the natural
probability-weighted scenario, the forced positive-event ceiling, and a
full-input-fingerprint cache. `icecast season --isolated-impacts` exposes the
same document to adapters. The NYR development-variance smoke run produced
eight isolated events, 32 baseline/natural team rows, and a reconciled
`+15 Path` from a raw `+15.4855` team-strength sum; the earlier visual's `+16`
is not retained as an unreconciled claim. Four focused tests and all 775 core
tests pass.

### Tasks

- For every highlighted breakout/downturn event, run baseline versus one-event
  paired simulations with identical schedule, parameters, seed, trials, and
  all unrelated events disabled.
- Report average points, playoff, round, conference, Cup-final, and Cup deltas.
- Retain raw team-strength delta as a separately labeled model input.
- Add an all-hit forced ceiling run and a naturally sampled distribution run.
- Reconcile the rounded `+16 Path` label to its raw strength sum.
- Preserve correlations through explicit `correlation_key`; never infer them
  from ID prefixes or multiply marginal probabilities in presentation code.
- Cache repeated paired runs by full input fingerprint.

### Tests

- paired baseline identity;
- one event cannot affect another team before its date;
- deltas reconcile exactly to stored baseline/scenario summaries;
- forced ceiling and sampled likelihood remain distinct;
- event ordering does not change independent draws; and
- cached and uncached results are byte-equivalent.

### Exit gate

Every player shown on The Insider has an intuitive isolated team impact, not
only an unexplained strength delta.

## Wave 5 — Team Prognosis and Comparison Builders

**Status:** Complete (2026-07-22).

Checkpoint: `build_team_prognosis_card` now emits the sealed two-page
`card_document.v1` from lineup, season forecast, isolated impact, and optional
event-role/score projections. Page 1 is The Depth Chart; Page 2 is The Insider
with baseline range, forced ceiling bridge, natural realization, positive and
negative player rows, methodology, warnings, and provenance. The
`card_comparison_set.v1` builder retains complete documents, aligns only
compatible headline metrics, and returns typed season/model/cutoff/scenario
warnings instead of deltas when dimensions differ. Canonical NYR fixture tests
verify Kartye, current/scenario scores, conditional impact, and the raw
`15.4855` / `+15 Path` pair. Canonical fixed-clock/fixed-seed NYR and SEA
documents now validate their transport-stable seals, reconcile their bridges,
retain Kartye on NYR, and produce a warning-free core comparison through the
explicit `development-variance` comparison key. All 778 core unit tests and
both canonical fixture tests pass.

### Tasks

- Add `TeamPrognosisCardInput` and `build_team_prognosis_card` in core.
- Build Page 1 `depth_chart` and Page 2 `insider` from shared primitives.
- Add baseline, p10-p90, playoffs, Cup, internal ceiling, and bridge metrics.
- Add breakout rows with current-to-hit score, probability, strength delta,
  isolated outcome deltas, role, and evidence.
- Add downside rows and combined realization probabilities.
- Add methodology, warning, and provenance sections.
- Generate canonical NYR and SEA JSON fixtures with fixed clock/seed.
- Add `card_comparison_set.v1` and a core comparison builder that validates
  compatible season, model, cutoff, and scenario dimensions before aligning
  metrics or calculating deltas.

### Showcase acceptance

- Rangers card correctly places Tye Kartye on NYR.
- Rangers baseline and internal ceiling reconcile with their source reports.
- Kraken baseline and internal ceiling reconcile with their source reports.
- The core-supplied rounded Path label is explicitly team strength, not
  standings points.
- Every visual number exists exactly once in the source document.
- Side-by-side output retains both complete documents and core-supplied aligned
  deltas.
- Incompatible cutoffs or scoring/model versions produce typed warnings rather
  than renderer guesses.

## Wave 6 — CLI and Independent JSON

**Status:** Complete (2026-07-22).

Checkpoint: `icelines report team-card` resolves a stable scenario ID, calls
the shared IceCast season builder, and renders only the resulting core card.
It supports complete JSON, compact text, fixed seed/trials, an optional fixed
RFC 3339 timestamp, and an explicit cross-team scenario comparison key. The
CLI contains no replacement player scoring or simulation formula. Clap parsing
and real NYR/SEA JSON generation pass; the text renderer wraps every line to
80 columns. `scripts/validate-card-document.ps1` independently validates and
summarizes either fixture from only the JSON document and published schema.
The renderer lives in a document-only module with a forbidden-builder import
test.

### Tasks

- Add `icelines report team-card` with season, team, scenario ID, text, JSON,
  and output-path options.
- Make JSON the complete canonical card document.
- Add a compact two-page text renderer for inspection and CI.
- Ship the JSON Schema and a small independent read-only example renderer or
  validation script that uses only the document.
- Document stable schema and exit behavior.

### Tests

- Clap parsing and invalid scenario errors;
- 80-column no-color capture;
- JSON golden fixtures;
- independent schema validation; and
- proof that CLI renderer imports no team scoring/simulation functions.

## Wave 7 — Web Card Surface

**Status:** Complete for the sealed 2026-27 NYR/SEA showcase (2026-07-22).

Checkpoint: the HTML and JSON routes resolve the same transport-validated
`CardDocumentView` from a read-only provider. The initial provider deliberately
supports only the two sealed canonical team/scenario combinations; unsupported
seasons, teams, and scenarios return explicit typed errors rather than falling
back to invented data. The Depth Chart and Insider are bookmarkable server-
rendered pages with team themes, headshot fallbacks, semantic headings,
responsive desktop/tablet/phone layouts, source warnings, and methodology.
The JSON response is semantically identical to the canonical core document.
Both surfaces use the document fingerprint as an ETag and honor
`If-None-Match`. Four router tests cover JSON identity/seal, HTML tabs and
Kartye, Kraken/error dimensions, responsive CSS, and cache behavior.

### Routes

```text
GET /icecast/:season/:team/card?scenario=:scenario_id
GET /api/v1/cards/team-prognosis/:season/:team?scenario=:scenario_id
```

### Tasks

- Resolve one document in the handler/service layer and pass it to HTML or JSON.
- Render The Depth Chart and The Insider as two tabs/pages from one object.
- Add desktop, tablet, and mobile layouts using shared semantic tokens.
- Make page and scenario state bookmarkable.
- Surface warnings and evidence authority above model conclusions.
- Add print styles that preserve both pages.
- Avoid a new SPA/build pipeline unless separately approved.

### Tests and artifacts

- HTML/API parity from one fixture object;
- route/parameter/scenario validation;
- keyboard and screen-reader labels;
- 1440x900 and 390x844 captures for both teams and both pages;
- missing-headshot and partial-roster states; and
- no renderer-local numeric calculations.

## Wave 8 — TUI Card Experience

**Status**: complete (2026-07-22)

Checkpoint: `icelines tui team-card NYR` and the command-bar forms
`team-card NYR` / `team-card SEA` now open the same validated
`card_document.v1` fixtures consumed by the other surfaces. The TUI renderer
imports document section types only and performs no player scoring or season
simulation. `p` changes between The Depth Chart and The Insider, `t` changes
teams, and `c` provides NYR/SEA comparison. Comparison is stacked below 120
columns and side by side at 120 and above. Six focused tests exercise 80, 120,
and 160 column terminal buffers, page and comparison interaction, exact fixture
ordering/values, warning visibility, long names, `NR`, and fingerprint
immutability.

### Tasks

- Add a `team-card` experience using existing TUI launch grammar.
- Toggle Page 1/Page 2 without rebuilding hockey logic.
- Page 1 uses compact face initials/name/score cells by line and pair.
- Page 2 uses headline metrics, ceiling bridge, breakout/downside tables, and
  warnings.
- Define density behavior at 80, 120, and 160 columns.
- Preserve ASCII-safe labels and keyboard help.

### Tests

- one fixture object drives every snapshot size;
- selection/paging does not mutate ViewModel content;
- order and values match CLI/web fixture;
- warnings remain visible at narrow width; and
- long names and `NR` scores remain comprehensible.

## Wave 9 — Reference Image/PDF Renderer

**Status:** complete (2026-07-22)

### Tasks

- Build or document a renderer that accepts only `card_document.v1` JSON.
- Keep official headshots source-resolved by IceLines.
- Record document and renderer versions in metadata.
- Validate rendered text against the document before acceptance.
- Generate separate NYR and SEA Page 1/Page 2 artifacts.
- Treat images as regenerable outputs, never the source of truth.

### Exit gate

Deleting the generated images loses no hockey information; the documents can
reproduce them.

### Checkpoint

- `scripts/render-card-document.ps1` consumes only `card_document.v1` JSON and
  emits one deterministic SVG per semantic page plus a render manifest.
- The default asset mode renders supplied player labels as initials. Optional
  `-ResolveAssets` verifies only HTTPS references already present in the
  document, embeds successful image responses, and falls back to initials.
- SVG metadata and PDF sidecars preserve document ID, schema, fingerprint,
  renderer ID, page ID, and asset mode. The manifest records resolved and
  fallback asset counts.
- Exact source-derived text is tagged and reparsed after write; missing or
  changed text fails the render.
- `scripts/test-card-reference-renderer.ps1` validates the 20 canonical
  prognosis, fantasy, and season-simulation SVG pages, metadata, applicable
  warnings, scores/`NR`, fingerprints, and network-independent default output.
- Regenerable SVG/PDF artifacts live under ignored `dist/cards/` and are not a
  hockey-data source.

## Wave 10 — Fantasy Card Family

**Status:** complete — all four fantasy card vertical slices are surface-complete (2026-07-22)

### Order

1. `fantasy_roster_card.v1`;
2. `fantasy_draft_card.v1`;
3. `fantasy_morning_card.v1`;
4. `fantasy_trade_card.v1`.

### Tasks

- Project existing fantasy ViewModels into shared card sections.
- Preserve the configured 2 C / 2 LW / 2 RW / 3 D / UTIL / 2 G / four-bench
  roster, 2 IR, 2 IR+, four pickups, same-day free agents, and two-day waivers.
- Carry the active scoring scheme and schedule equivalence classes.
- Ensure draft recommendations include multi-position flexibility and roster
  gaps.
- Ensure morning cards carry locks, injuries, goalie evidence, and pickup
  budget.
- Ensure trade cards carry before/after legal roster state and matchup/playoff
  impact.

### Exit gate

At least fantasy roster and fantasy draft/morning cards render on CLI, web, and
TUI from their identical documents. Remaining kinds may ship incrementally but
cannot fork the grammar.

### Roster-card checkpoint

- Added backward-compatible persisted `free_agent_same_day` league authority;
  configured rules retain four pickups and two-day waivers.
- Enriched `FantasyDailyLineupView` with stable rich bench assignments while
  retaining its legacy bench-name list.
- Added core `fantasy_roster_card.v1` projection in the shared
  `card_document.v1` envelope with sealed lineup/schedule fingerprints.
- Added `icelines fantasy roster-card [--date ...] [--json]`, sourcing the
  marked roster, eligibility/status evidence, acquisition ledger, scoring
  scheme, and official schedule classes.
- CLI, TUI, web, SVG, and PDF document renderers preserve decision rationale and all
  schedule-class alternatives without recalculation.
- Added a sealed Sample Multicategory fixture sourced from historical workbook names,
  with deterministic status/projection disclaimers and a fixture generator.
- Added fingerprint/ETag-preserving JSON and bookmarkable roster/Insider HTML
  routes plus the `team-card DEX` terminal projection.
- Added core, web, TUI, schema, and reference-renderer regressions over the same
  document. The roster-card vertical slice is surface-complete; the trade card
  builder remains pending.

### Draft-card checkpoint

- Added core `fantasy_draft_card.v1` projection from the existing
  `FantasyDraftBoardView`; ranking and roster-fit logic remain single-sourced.
- Preserved the next-pick recommendation, fallback, position alternatives,
  open starter gaps, multi-position eligibility, schedule/playoff fit, risk,
  and taken/eligibility import resolution in the sealed document.
- Added `icelines fantasy draft-card`, including pasted taken-player input and
  JSON or shared text-card output.
- Added a deterministic Sample Multicategory pick-seven fixture plus schema, seal,
  and reference-SVG regression coverage.
- Added fingerprint/ETag-preserving draft JSON and bookmarkable Draft
  Board/Insider HTML routes, plus `team-card DRAFT` and `:draft-card` terminal
  entry points. Core, CLI, web, TUI, schema, and SVG regressions consume the
  identical fixture; the draft-card vertical slice is surface-complete.

### Morning-card core/CLI checkpoint

- Added core `fantasy_morning_card.v1` projection from the existing
  `FantasyMorningBriefingView`, retaining legal lineup, action priority,
  pickup reserve, goalie evidence/checkpoint timing, injury refreshes,
  warnings, and methodology.
- Added `icelines fantasy morning-card`, which runs the same injury, goalie,
  weekly-pickup, sleeper, and material-fingerprint pipeline as `morning` before
  sealing JSON or shared text-card output.
- Added a deterministic Sample Multicategory fixture with an IR+ refresh, protected
  fourth pickup, Darren Raddysh candidate, and uncertain Igor Shesterkin/Juuse
  Saros evidence. Seal and reference-SVG regressions now validate ten pages
  across prognosis, roster, draft, and morning documents.
- Added fingerprint/ETag-preserving morning JSON and bookmarkable Morning
  Skate/Insider HTML routes. The generic web projection now renders timeline
  sections, including goalie refresh, safety-check, and lock deadlines.
- Added `team-card MORNING` and `:morning-card` terminal entry points. Web and
  TUI regressions consume the identical sealed fixture; the morning-card
  vertical slice is surface-complete.

### Trade-card core/CLI checkpoint

- Moved the trade player, team-impact, and evaluation contracts out of the CLI
  into core as `fantasy_trade_evaluation.v1`; the evaluator and trade finder now
  share those public types.
- Added core `fantasy_trade_card.v1`, preserving both packages, fairness gap,
  roster legality, before/after values, remaining games, roster capacity, open
  slots, warnings, methodology, joins, and source fingerprint.
- Added read-only `icelines fantasy trade-card`, which runs the existing trade
  evaluator but cannot save or execute the offer.
- Added a deterministic Sample Multicategory/Blue Line Bandits Fox–Rantanen fixture
  with explicit non-current-advice warnings. Reference SVG now validates twelve
  pages across prognosis, roster, draft, morning, and trade cards.
- Added fingerprint/ETag-preserving trade JSON and bookmarkable Trade
  Board/Insider HTML routes, plus `team-card TRADE` and `:trade-card` terminal
  entry points. Core, CLI, web, TUI, schema, and renderer regressions consume
  the identical fixture; the trade-card vertical slice is surface-complete.

## Wave 11 — Season Simulation Card

Status: complete. Prospective 2026-27 and completed 2024-25 NYR/SEA fixture
pairs exercise the same core/CLI/web/TUI contract.

Implemented:

- `season_simulation_card.v1` projects a focused team only after sealing and
  fingerprinting the complete 32-team/1,344-game league run;
- The Scoreboard preserves record, P10/P50/P90 points, playoff/Cup path,
  streak outlook, and paired scenario deltas;
- The Insider preserves schedule pressure, pivotal hunt/spoiler games, dated
  personnel/trade/form events, downside/middle/upside sampled event buckets,
  methodology, disclosures, and provenance;
- `icecast season-card`, HTML/JSON routes, and `season-card` TUI entry points
  consume the same neutral NYR/SEA fixtures, including side-by-side mode;
- regression tests prove that both focused documents share one run fingerprint
  and that changing an unrelated league row changes that fingerprint.
- completed rolling replay cards add confirmed actual W-L-OTL/points, focused
  and league pick accuracy, Brier score, calibration error, coin-flip skill,
  and the best tested chronological Elo blend; the sealed 2024-25 run covers
  all 32 teams and 1,312 final games with zero pending.

### Tasks

- Build `season_simulation_card.v1` from the existing league forecast view.
- Preserve the full league-run fingerprint in every focused team card.
- Present baseline/scenario distribution, best/median/worst paths, event
  buckets, trades, injuries, streaks, and playoff impact.
- Support completed-season replay with actual-result/calibration sections.
- Prove season-generic behavior beyond 2026-27.

### Exit gate

The Rangers/Kraken focused cards are auditable projections of the same
32-team/1,344-game run rather than separate simulations.

## Wave 12 — Parity, Documentation, and Release Gate

**Status:** complete (2026-07-22). Canonical specs, indexes, architecture,
commands, release/rollover guidance, and the archive manifest now describe the
implemented card portfolio and its actual CLI/web/TUI entry points. Schema
validation passes for both prospective and historical NYR/SEA simulation
cards, and the PowerShell 5/7-compatible reference-renderer gate validates 20
canonical SVG pages plus its decision-section fixture. The documentation audit
reports zero broken local links; 146 legacy documents remain recorded for the
separate archive migration, which is intentionally excluded from this mixed
code-and-documentation change set.

### Required updates

- `README.md` and `COMMANDS.md`;
- `design/ARCHITECTURE.md` and `design/IceLines.md`;
- platform, ViewModel, visual, surface-parity, forecast, fantasy, and brand
  specs by reference rather than copied field lists;
- spec and plan indexes;
- release checklist and current-season rollover documentation; and
- the documentation archive/consolidation manifest.

### Validation matrix

| Gate | Required evidence |
|---|---|
| Core | unit/property/golden/schema tests |
| CLI | text/JSON and 80-column captures |
| Web | HTML/API parity, accessibility, desktop/mobile captures |
| TUI | 80/120/160 snapshots and interaction tests |
| Image/PDF | document/text/metadata validation |
| Fantasy | legality, scoring, locks, waivers, pickup-budget fixtures |
| Simulation | paired deltas, event buckets, league reconciliation |
| Docs | link check, index truth, no duplicated canonical schema |

### Definition of done

- one core document drives every claimed surface;
- renderer-local hockey calculations are absent and fenced;
- NYR and SEA visual cards regenerate entirely from IceLines;
- fantasy and simulation prove extensibility;
- all discovered bugs are fixed or explicitly blocked with typed warnings;
- surface parity is recorded honestly; and
- active documentation points to a small canonical set.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Generic card grammar becomes a dashboard DSL | bounded typed sections and domain builders |
| Page 1 becomes too dense | one score only; semantic pages; responsive density |
| `+16` is misunderstood | explicit team-strength unit plus standings bridge |
| Scores mix unlike meanings | one versioned 0-100 player-score contract |
| Renderers drift | golden document and cross-renderer parity tests |
| Web exposes local paths | scenario registry IDs only |
| Images fabricate faces/text | official assets; source JSON; text validation |
| Simulation cost grows | fingerprinted paired-result cache |
| Missing sources become fake zeros | null/NR plus warnings and completeness state |
| Documentation expands again | consolidation policy and active-plan cap |

## Forecast History Showcase (completed 2026-07-22)

- Core owns `team_season_forecast_history.v1` and
  `forecast_history_card.v1`, including chronology validation, every sealed
  checkpoint fingerprint, absolute levels, consecutive and first-to-last
  deltas, and league riser/faller rankings.
- CLI `icecast history-card`, TUI `history-card NYR|SEA`, and web HTML/JSON
  routes project the same two-page The Tape/Insider documents.
- The sealed Jan. 31 / Feb. 28 / Mar. 31, 2025 NYR/SEA fixtures provide parity evidence
  for page switching, team toggling, side-by-side display, ETags, and source
  preservation. A year-parameterized PowerShell generator reproduces the
  history and both cards while keeping intermediate checkpoints temporary.

## First Executable Slice

The first implementation slice should stop after Waves 0-5 and produce:

- shared Rust card primitives;
- scenario registry foundation;
- canonical player-score and lineup builder;
- isolated NYR/SEA player impacts;
- validated `team_prognosis_card.v1` golden JSON for both teams; and
- no UI renderer beyond a temporary JSON inspection path.

That slice proves the architecture before visual work begins.
