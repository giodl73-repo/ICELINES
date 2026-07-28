# The Window — Organization Health Implementation Plan

**Date:** 2026-07-27
**Status:** Active — foundation and evaluation surfaces implemented; historical,
scenario-attribution, calibration, and release evidence remain open
**Specification:** [`../specs/organization-window.md`](../specs/organization-window.md)
**Review:** [`../notes/2026-07-27-organization-window-roles-review.md`](../notes/2026-07-27-organization-window-roles-review.md)
**Parent workstreams:** Team Season Forecast, Line Combination Simulation,
Fantasy War Room, and organization/prospect intelligence

## Outcome

Ship a reproducible 32-team organization-health system that composes IceLines'
existing analytical authorities without flattening them into an opaque rank.
Users can select or author a scoring Frame, inspect every contribution, compare
matched checkpoints, and measure scenario sensitivity across CLI, TUI, Web,
JSON, and cards.

## Planning principles

1. Inventory before scoring. IceLines has many candidate lenses, but a command
   name or ViewModel is not automatically a production-grade profile.
2. One profile contract, many typed producers. Extension is deliberate and
   versioned, not stringly typed or renderer-owned.
3. Score, confidence, and coverage stay separate.
4. Aggregate hierarchically to control double counting.
5. Freeze a complete 32-team cohort before focusing on one team.
6. Preserve point-in-time authority and historical comparability.
7. Configuration can alter weights and gates; formulas require reviewed method
   versions.
8. Blocked source claims remain visible instead of receiving proxy values.

## Implementation baseline (2026-07-27)

This plan is incremental. The following baseline is already present and must be
preserved while the remaining work is completed:

| Workstream | State | Proven capability | Remaining gate |
|---|---|---|---|
| W0 | complete | 32-profile machine-readable inventory: 17 ready, 8 evaluation, 4 context-only, 3 blocked | Reclassify only through the promotion protocol below. |
| W1-W2 | complete | Versioned observations/manifests/boards, deterministic fingerprints, validation, normalization, aggregation, confidence/coverage, rank gates | Cross-platform fingerprint matrix in W9. |
| W3-W4 | evaluation-complete | `balanced.v1`, typed source adapters, all-32 partial evaluation board, classifications, focused cards | Fill production source coverage before claiming a complete ranked board. |
| W5 | partial | Comparable movement/history contracts, refusal tests, and immutable bridge/rebase through the canonical scorer | Real point-in-time checkpoints and personnel attribution evidence. |
| W6 | evaluation-complete | Sealed baseline/scenario comparison, typed trade/injury/development/camp/line authorities, direct/cohort/unchanged attribution, and fail-closed fixture | Real multi-source isolated/combined scenario boards and seeded distribution evidence. |
| W7 | evaluation-complete | Leakage gate, per-origin frozen baselines, rolling origins, pane ablations, organization stability, between-origin uncertainty, sealed claim status, and frozen training/validation/retrospective-holdout roles | Real multi-season point-in-time inputs, trial-noise propagation, and future untouched-holdout evidence. |
| W8 | implementation-complete | CLI, two-page TUI all-32 board/focused cards, Web/API, JSON, UI-neutral card, durable Markdown report, desktop/tablet/mobile live review, semantic checks, keyboard skip-focus walkthrough, reduced-motion check, and 390px overflow inspection | Final automated cross-surface golden parity. |
| W9 | in progress | Authoring/compatibility/cache documentation, additive compatibility and registered-profile extension fixtures, canonical loaded-board replay validation, observed three-OS fingerprint CI gate, strict affected-production lint, schema/golden checks, full PR CI, offline release smoke, Windows package verification, dependency audit, measured performance baseline, and live browser/accessibility review | Add the full package matrix, real multi-season production evidence, and automated cross-surface golden parity, then complete release closeout. |

“Partial” is a product state, not a failure state: saved artifacts must expose
missing evidence and withhold ranks or claims when gates are not met.

## Extension and alteration protocol

The Window supports change through six explicit lanes. A change must use the
narrowest lane that represents its semantics; changing a fingerprint without
changing the relevant version is a compatibility defect.

| Requested change | Required artifact | Version/fingerprint effect | Minimum review and evidence |
|---|---|---|---|
| Reweight or enable existing Lines | New Frame manifest | New manifest and board fingerprints; profile methods unchanged | PACE, SCOUT; manifest validation, cap/missingness sensitivity |
| Add a new Line/profile | Descriptor, typed provider, observation schema fixture, Frame opt-in | New registry revision; opt-in manifest fingerprint changes | HART, TAPE, PACE, BENCH; authority, identity, known-value, missing-data tests |
| Change a formula or direction | New profile `method_version` | Existing observations/boards remain immutable; comparison refused absent bridge | PACE, BENCH, EDGE; known-value, boundary, replay, bridge decision |
| Add or split a pane/view | New manifest structure using registered profiles | New manifest fingerprint; no source recomputation unless profile set changes | HART, KEEL, GLASS; aggregation, parity, narrow-surface review |
| Add or replace a source | Provider dependency/version declaration and source fingerprint | Observation identity changes; method version changes if semantics change | TAPE, WIRE, EDGE; offline fixture, schema drift, freshness/fallback tests |
| Evolve a document schema | New major schema or documented additive-compatible minor change | Old artifact stays readable or fails explicitly; never rewritten in place | KEEL, WIRE, FORGE, BENCH; compatibility and migration fixtures |

Profile promotion is one-way only when evidence supports it:

```text
blocked -> context-only -> evaluation -> ready
```

Each promotion records source authority, cohort coverage, point-in-time safety,
method version, calibration claim, limitations, and verification evidence.
Demotion is always allowed when authority or freshness regresses and must not
silently retain the previous rank eligibility.

Every pull request that alters Window semantics includes a change note naming:

1. the lane above;
2. affected profile, manifest, schema, and source fingerprints;
3. compatibility behavior for saved artifacts;
4. newly valid and invalid comparisons; and
5. VTRACE evidence added or intentionally still open.

## Workstream map

### W0 — Authority inventory and profile readiness

Create a machine-readable and documented catalog of candidate organization
profiles. For each existing producer, record:

- core type and schema;
- source authority and freshness;
- organization/season/as-of/horizon axes;
- observed, modeled, heuristic, context-only, or blocked status;
- 32-team coverage;
- historical availability;
- known dependency/signal family;
- scenario support;
- calibration target and evidence; and
- promotion gaps.

Seed the inventory from team strength, position/depth, goaltending, special
teams, organization lineup, training camp, prospect program, prospect
conversion, AHL development, line combinations, management behavior, injury,
transactions/trades, schedule/fatigue, and IceCast outputs.

Do not promise “multiple dozen runnable profiles” until the catalog counts
which ones satisfy the common contract. Publish exact totals by readiness
class.

**Exit:** every candidate is uniquely keyed, versioned, dependency-labeled,
and assigned a promotion state; cap and shifts are explicitly blocked where
authority is absent.

### W1 — Contract and registry foundation

Add pure core types for:

- `ProfileKey`, `ProfileMethodVersion`, and `ProfileDescriptor`;
- `OrganizationProfileObservationV1`;
- `OrganizationWindowManifestV1`;
- typed status, direction, horizon, normalization, trend, confidence,
  coverage, evidence, limitation, and rank-status enums;
- registry lookup and validation; and
- deterministic canonical fingerprints.

Create JSON Schemas under `design/schemas/`. Add parse/validate helpers that
reject unsupported versions, duplicates, cycles, incomplete axes, invalid
weights, illegal caps, non-finite values, degenerate budgets, and unknown
profile references. Canonical fingerprints normalize field order, decimal
representation, and negative zero and never depend on map iteration order.

The first implementation uses a compile-time typed provider registry plus
declarative JSON/TOML manifests. Dynamic native plugins and runtime scripts are
out of scope.

**Exit:** fixture observations and manifests round-trip; invalid states fail
with typed errors; adding a registered profile does not require editing the
aggregator.

### W2 — Normalization and hierarchical scorer

Implement deterministic cohort normalization, tied ranks, pane aggregation,
view aggregation, signal-family caps, coverage/confidence propagation, and
rank gates in core.

Required tests include:

- all scores/confidence/coverage remain in bounds;
- input order does not change output;
- tied inputs receive deterministic tied ranks;
- inverse and target-range directions normalize correctly;
- missing evidence reduces coverage rather than score by fiat;
- a blocked required profile withholds rank;
- correlated variants cannot exceed their family cap;
- zero-variance and below-minimum cohorts follow their explicit policies;
- current-season boards require 32 canonical teams while historical boards
  require the complete catalog for their season;
- custom weights create a distinct fingerprint;
- equivalent canonical manifests share a fingerprint;
- duplicate, cyclic, and unknown profile declarations fail; and
- a team cannot improve rank merely because an unfavorable input disappears.

**Exit:** a synthetic 32-team board is fully explainable and deterministic.

### W3 — First production profile adapters

Promote a deliberately small first set spanning the initial panes. Adapters
consume sealed core ViewModels or source-authority records; they do not
reimplement upstream formulas.

Recommended first candidates, subject to W0 findings:

1. current IceCast/team strength;
2. forward depth;
3. defense depth;
4. goalie stability/dependency;
5. organization lineup/recall depth;
6. prospect program strength;
7. prospect conversion performance;
8. training-camp arrival depth;
9. lineup/deployment optionality using supported evidence only;
10. schedule/fatigue exposure; and
11. roster concentration/resilience.

Keep management research context-only at first. Keep cap flexibility blocked
until a verified source, identity join, point-in-time store, and methodology
exist. Keep shift-derived chemistry blocked while the shifts capability is
locked.

Build an official `balanced.v1` Frame only after coverage and dependency review.

**Exit:** a real, frozen all-32-team evaluation board is reproducible from
saved inputs; every row exposes raw value, evidence, status, confidence,
coverage, and limitations.

### W4 — Window board, team detail, and classifications

Build `OrganizationWindowBoardV1` and the first multi-axis classification
method. Preserve the complete cohort and board fingerprint before filtering.
Generate strengths, vulnerabilities, blockers, and the evidence summary from
typed profile results.

Classification must distinguish current quality from sustainability. Boundary
tests cover contender, rising, fragile, plateau, retooling, rebuilding, and
incomplete states.

**Exit:** Rangers, Kraken, and every other team use the same league artifact;
the focused view can explain its rank without local calculations.

### W5 — Comparable history and The Shift

Build history validation and matched-checkpoint movement:

- same manifest and method versions;
- same normalization policy and valid cohort;
- matched season phase/as-of convention;
- season-aware team identity; and
- complete source fingerprints.

Add a bridge/rebase contract for intentional method upgrades. Do not implement
automatic cross-version subtraction.

Decompose movement into observed inputs, personnel, confidence/coverage,
method/manifest, and residual revaluation.

**Exit:** at least three historical checkpoints produce explainable movement;
incomparable checkpoints fail with actionable reasons.

### W6 — Scenario sensitivity

Connect existing IceCast, player development, trade, injury, training-camp,
and line-combination scenario artifacts through typed adapters. Report
isolated and combined profile/pane deltas and retain baseline/scenario
fingerprints.

Start with deterministic fixture scenarios, then seeded distributions. Add
monotonicity tests where the underlying scenario has an ordered expectation;
do not require monotonicity for genuinely interacting lineup changes.

**Exit:** users can see what must go right or wrong for a team's Window to
move, without scenarios rewriting observed history.

### W7 — Calibration and historical replay

Construct rolling-origin boards from historical point-in-time inputs. Establish
separate targets and baselines for current strength, sustainability, pipeline,
development, resilience, and later flexibility.

Required evidence:

- leakage audit for every profile;
- continuous-target error/rank correlation where appropriate;
- Brier/log loss and calibration only for probability targets;
- simple baseline comparison;
- ablation by pane and signal family;
- stability across seasons and organizations;
- sensitivity to Frame weights and missingness; and
- uncertainty intervals that distinguish trial noise from season variation.

If the balanced Frame does not improve on simple baselines, ship it as
descriptive/heuristic and do not market it as predictive.

**Exit:** a sealed validation artifact states which claims are calibrated,
inconclusive, or blocked.

### W8 — Surfaces and UI-neutral cards

Add thin commands and routes only after the core board is stable:

- CLI board/team/history/scenario/explain commands;
- TUI 32-team board and team drilldown;
- Web HTML/JSON routes with bookmarkable context;
- `card_document.v1` projection for focused team pages; and
- durable JSON plus optional Markdown report output.

Update `COMMANDS.md`, clap help, `README.md`, surface parity, visual docs, and
release fixtures together. The default display shows score, rank status,
confidence, coverage, panes, and primary drivers—not dozens of columns.

The Web surface exposes only registered saved Frames by stable ID/fingerprint;
a local manifest file is a CLI input, not an unbookmarkable GET upload. Web
query state includes season, as-of, view, and Frame ID. Fingerprinted JSON may
use ETag/conditional GET; stale or partial boards use conservative cache
headers. HTMX fragments preserve context and have semantic no-JavaScript
fallbacks.

The visual design avoids a giant master-score gauge. The board uses a calm,
hockey-native comparison table; team detail uses panes and evidence hierarchy.
Color is supplemental, the selected horizon is unmistakable, and screenshot,
80-column, narrow-browser, keyboard, and reduced-motion reviews are release
gates.

**Exit:** every renderer consumes the same sealed board, works without color,
shows active context, and exposes recovery for partial/stale/blocked states.

### W9 — Hardening, release, and extension kit

Publish:

- profile-author guide;
- manifest customization guide and examples;
- schema compatibility and deprecation policy;
- official Frame changelog;
- all-32 replay and surface parity fixtures;
- performance measurements and cache policy;
- migration behavior for saved boards; and
- release checklist additions.

Run focused L0/L1/L2, schema, JSON round-trip, surface parity, no-network,
historical replay, full CI, clippy, fmt, audit, release smoke, and package
verification gates.

**Exit:** a new profile can be added through the documented typed-provider
path, a user can safely alter weights through a manifest, and old sealed boards
remain readable or fail with an explicit version message.

## Proposed build order

```text
W0 inventory
  -> W1 contract/registry
  -> W2 scorer
  -> W3 first adapters
  -> W4 board/detail
  -> W5 history
  -> W6 scenarios
  -> W7 calibration
  -> W8 surfaces/cards
  -> W9 release/extension kit
```

W5 and W6 may proceed in parallel only after W4 seals board identity. W8 may
prototype against fixtures but cannot own business logic or claim parity before
W4-W7 gates are met.

## Crate ownership

| Work | Owner |
|---|---|
| Profile contracts, registry, scorer, board, history, scenario comparison, classification | `icelines-core` |
| Source assembly, saved-artifact loading, point-in-time orchestration, source cache | `icelines-fetch` |
| CLI args, file loading/writing, text and JSON handoff | `icelines-cli` |
| HTML/API projection and route state | `icelines-web` |
| Shared card projection and semantic tokens | core ViewModels/card system |

No `icelines-core` I/O, renderer-local formula, live-network test, or
`StatsRepository` ownership relaxation is permitted.

## VTRACE and documentation work

Before W1 implementation, add requirement, design, interface, verification,
validation, trace, work-package, review, and change-control entries under
`docs/vtrace/`. The trace must map every production claim to source authority,
core type, test tier, and user surface.

The documentation consolidation workstream owns archival placement; this plan
stays in the canonical active set while implementation is active and moves to
history only after release and evidence closeout.

## Risk register

| Risk | Mitigation |
|---|---|
| Attractive but meaningless master score | Lead with panes, evidence, coverage, and scenarios; keep rank secondary. |
| Double counting correlated profiles | Hierarchical aggregation, signal families, contribution caps, and ablations. |
| Missing data biases teams differently | Frozen expected set, explicit coverage, rank gates, no zero fill. |
| Weight tuning leaks future outcomes | Freeze manifests per backtest origin; evaluate later periods untouched. |
| Method upgrades destroy YoY meaning | Immutable method versions and explicit bridge/rebase artifacts. |
| Qualitative research becomes pseudo-data | Context-only default; numeric promotion requires a calibrated method. |
| Cap/shift proxies overclaim authority | Keep panes blocked until verified sources and joins exist. |
| One team is computed outside league context | Build/fingerprint all 32 before focus. |
| Surface drift | Core-built documents and parity fixtures; renderers cannot recompute. |
| Custom Frames create incomparable rankings | Label and fingerprint every Frame; compare only like-for-like. |
| Slow repeated all-profile runs | Fingerprint-keyed observations and boards; benchmark before cache design. |
| Historical relocation/expansion errors | Season-aware canonical team catalog and identity validation. |
| Canonical fingerprints drift by platform | Canonical serialization, finite-number validation, cross-platform golden vectors. |

## Release slices

1. **Foundation preview:** W0-W2 with synthetic fixtures and manifest tooling.
2. **Evaluation board:** W3-W4 with saved all-32 inputs, explicitly not yet a
   predictive release.
3. **Movement and scenarios:** W5-W6.
4. **Calibration candidate:** W7 and default Frame decision.
5. **User release:** W8-W9 with docs, surfaces, packages, and extension kit.

Each slice must be build-green and useful on its own. A release note names
which panes are production, heuristic, context-only, or blocked.

## Definition of done

- The common profile inventory reports exact readiness counts.
- At least one official Frame produces a deterministic, explainable all-32
  board with comparable evidence coverage.
- All ranks and deltas satisfy cohort, coverage, freshness, and method gates.
- Historical replay is point-in-time safe and honestly calibrated or labeled.
- Scenario sensitivity reuses sealed IceLines authorities.
- Users can alter weights without changing hockey logic.
- Developers can add a profile without changing the scorer or renderers.
- CLI, TUI, Web/API, JSON, and cards agree on every hockey value.
- Current boards contain all 32 organizations; historical boards contain the
  complete season-canonical league rather than fabricating 32.
- VTRACE, specs, plans, commands, surface parity, and release docs match the
  running build.
