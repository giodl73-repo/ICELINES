# IceLines Sources — Extraction and Prospect Census Plan

**Date:** 2026-07-31
**Status:** Active authority-closure child workstream
**Progress:** S0-S5 and S7-S8 complete locally; S6 awaits authorized ledgers
**Specification:** [`../specs/icelines-sources.md`](../specs/icelines-sources.md)
**Role review:** [`../notes/2026-07-31-icelines-sources-roles-review.md`](../notes/2026-07-31-icelines-sources-roles-review.md)
**Parents:** Documentation Consolidation, organization/prospect intelligence,
Team Season Forecast, and Fantasy War Room

## Outcome

Create a reusable provider-normalization crate and use it to produce a dated,
auditable all-32 player-source package. The first product proof is an honest
top-ten prospect census for every organization; the architecture must also be
usable by fantasy, simulation, lineup, transaction, and organization-health
consumers.

This is a child workstream and does not consume another top-level active-plan
slot.

## Baseline to preserve

- Canonical identity is `icelines-core::identity::{PlayerId, PlayerIdentity}`.
- FLETCH owns reusable source-byte acquisition where already integrated.
- `icelines-fetch` owns snapshots, cache manifests, active pointers,
  persistence, and stale/offline policy.
- Reviewed AHL identity application is explicit and must remain fail-closed.
- Prospect scoring, graduation, confidence, top-ten completeness, and
  UI-neutral outputs already work.
- The current audited board has 23 complete and nine partial organization top
  tens, with 17 eligible studies missing across FLA, BOS, EDM, LAK, NJD, NYI,
  BUF, COL, and WPG.
- Existing worktree changes and serialized v1 artifacts must remain intact.

## Delivery principles

1. Extract behavior before changing semantics.
2. Build green after every commit-sized slice.
3. Deliver the missing census sources early; do not make product value wait for
   a complete `icelines-fetch` decomposition.
4. Re-export moved public APIs during migration.
5. Separate resolved facts from identity proposals.
6. Separate attendance, control, assignment, contract, and transaction events.
7. Preserve raw source evidence, capture time, effective time, and hashes.
8. No team-specific scoring or eligibility branches.
9. No source-completeness claim from a count alone; every transition has an
   exclusion ledger.
10. Do not update TRACKER submodule pointers until child work is committed,
    pushed, and explicitly selected for a portfolio snapshot.
11. Update authority, schema, command, and surface documentation in the same
    slice that promotes a source or consumer; S8 owns final consolidation and
    release notes, not delayed documentation truth.

## Work packages

### S0 — Freeze inventory and compatibility baseline

**Status:** Complete — see
[`../notes/2026-07-31-icelines-sources-s0-inventory.md`](../notes/2026-07-31-icelines-sources-s0-inventory.md).

Document every `icelines-fetch` module by responsibility:

- acquisition/transport;
- cache/snapshot/persistence;
- provider DTO/parser;
- source normalization/reconciliation;
- feature-domain composition; and
- UI/command orchestration that should move behind a service.

Record public Rust paths, artifact schemas, example documents, commands, cache
keys, and test fixtures affected by extraction. Capture representative outputs
for player landing, roster, AHL identity, and prospect population.

**Acceptance**

- Machine-readable or checked Markdown inventory covers every fetch module.
- Compatibility fixture hashes are recorded before code moves.
- The current 23/9 prospect completeness result is reproducible offline.

### S1 — Scaffold `icelines-sources`

**Status:** Complete locally — see
[`../notes/2026-07-31-icelines-sources-s1-scaffold.md`](../notes/2026-07-31-icelines-sources-s1-scaffold.md).

Add the workspace crate with:

- crate-level ownership and non-goals;
- `SourceAdapter`, `SourceDescriptor`, and `SourceInput` foundations;
- typed adapter errors;
- deterministic ordering helpers; and
- compile-time/CI forbidden-dependency checks.

Correct `design/ARCHITECTURE.md` to the actual dependency edge list in this
slice. Do not introduce the new fact/package schema yet and do not move feature
policy into the new crate.

**First extraction proof:** move the pure NHL player-landing parser behind
`icelines-sources::nhl::player_landing`, retain the existing
`icelines_fetch::career_landing` facade, and prove semantic output equality.

**Acceptance**

- Workspace builds under the default and release feature sets on the CI target
  matrix already declared by the repository workflow.
- A checked dependency script plus `cargo tree -p icelines-sources` proves the
  explicit edge allowlist and forbidden dependencies;
- The new crate has no network, runtime, database, filesystem, or renderer
  dependency.
- Existing player-landing call sites and artifacts remain compatible.
- The existing player-landing output type remains unchanged and semantic
  equality is frozen; any byte golden names its exact serializer/options.

### S2 — Canonical fact and package contracts

**Status:** Complete locally — see
[`../notes/2026-07-31-icelines-sources-s2-contracts.md`](../notes/2026-07-31-icelines-sources-s2-contracts.md).

Implement and validate:

- `SourceEvidence`;
- validated typed IDs, hashes, URLs, versions, and non-empty evidence;
- typed `FactSubject` and immutable `FactAssertion<T>` with fact domain time,
  supersession, and retraction;
- `ProviderIdentityProposal`;
- separate append-only `IdentityReviewDecision`;
- player-organization events;
- participation facts and participation authority;
- typed conflict/exclusion rows;
- `icelines_source_package.v1`; and
- deterministic package fingerprints.
- requested-scope/run manifests, freshness state, typed coverage, and stable
  disclosure codes.

Add a compatibility lowering for `prospect_population_overlay.v1` without
claiming that its legacy relationship field is the final general model.

**Acceptance**

- Canonical facts reject zero or unresolved player IDs.
- Private constructors and deserialization reject invalid IDs and evidence.
- Attendance alone cannot create control authority.
- Capture, knowledge cutoff, and effective time remain distinct.
- Package order does not change fingerprints.
- v1 overlays round-trip and lower with explicit compatibility disclosure.

### S3 — Prospect population vertical slice

**Status:** Complete locally — official draft, camp-publication,
contract-publication/termination, trade, NHL assignment, reviewed AHL
assignment, and all-organization scope slices are
recorded in
[`../notes/2026-07-31-icelines-sources-s3-prospect-population.md`](../notes/2026-07-31-icelines-sources-s3-prospect-population.md).
The fetch-owned acquisition/replay path, versioned source catalog, explicit
PTO fixture, and sealed real all-32 audit package are complete. The package is
honestly incomplete for the three uncataloged families; closing that authority
belongs to S5-S6 rather than being hidden inside this vertical slice.

Implement source adapters in value order:

1. official NHL multi-year draft selections;
2. official NHL club development/rookie/training-camp publications;
3. official contract/signing facts;
4. official transactions and rights transfers/releases; and
5. current NHL/AHL roster and assignment facts already available in IceLines.

Club-publication support is layout-driven, not team-driven. Start with the
common NHL.com structured article/table payload and add named layout variants
only when frozen fixtures prove a genuinely different representation.

The output is an all-32 candidate source package, not a ranked board.

**Acceptance**

- Every season-canonical organization is present even when its candidate set is
  empty or blocked.
- The sealed run manifest distinguishes authoritative empty,
  not-applicable, acquisition failure, quarantine, and incomplete pagination;
- Drafted, signed, assigned, controlled camp attendee, free-agent invite, and
  unknown attendee paths are independently tested.
- Camp absence never becomes a release.
- Rights transfers and same-name players fail closed without reviewed identity.
- No branch names FLA, BOS, EDM, LAK, NJD, NYI, BUF, COL, or WPG.

### S4 — Identity and current-state reconciliation

**Status:** Complete locally — see
[`../notes/2026-07-31-icelines-sources-s4-current-state.md`](../notes/2026-07-31-icelines-sources-s4-current-state.md).

Feed unresolved provider people into the existing reviewed identity workflow.
Do not create a second canonical identity system.

Build versioned state resolvers for:

- current organization control;
- current assignment;
- participation-only status;
- transferred/released/expired rights; and
- conflicting or stale evidence.

Any rights-duration policy derived from the CBA is separately versioned from
source parsing and must support historical replay.

Rights output is `Supported`, `Expired`, `Transferred`, `Unknown`, or
`Conflicted` with reasons and required-input disclosures. A draft event alone
never establishes current control. Replay exposes `effective_cutoff` and
`knowledge_cutoff`; optional reconstructed-identity mode may use a later review
only without importing later hockey facts.

**Acceptance**

- Provider IDs never enter canonical facts directly.
- Review decisions retain reviewer, review time, evidence, and original
  proposal.
- Current-state output names its policy version and complete input fact set.
- Default historical replay rejects observations/reviews after the knowledge
  cutoff and events after the effective cutoff; reconstructed-identity tests
  prove later reviews cannot import later performance or organization facts.

### S5 — Coverage funnel and prospect census

**Status:** In progress — the UI-neutral funnel, typed loss ledger, independent
authority/depth gates, dimensional counts, and strict publication guard are
recorded in
[`../notes/2026-07-31-icelines-sources-s5-prospect-census.md`](../notes/2026-07-31-icelines-sources-s5-prospect-census.md).

Add a provider-neutral census consumer with per-team and league totals for:

```text
discovered
canonical identity
controlled relationship
prospect eligible
career evidence usable
study built
ranked
```

Every loss between stages produces a typed row. Compose the census with the
existing AHL and player-landing career adapters, then feed only canonical,
eligible studies to the unchanged prospect ranking builder.

Add two independent publication states:

- `population_authority_status`, proven by the complete enumerated source
  acquisition matrix and resolved/excluded conflicts; and
- `ranking_depth_complete`, proven by the requested number of eligible studies.

Add skater/goalie coverage dimensions and a UI-neutral `ProspectCensusView` for
the first CLI text/JSON consumer, `icecast prospect-census --source-package
PATH [--json]`. It carries status, counts, exclusions, freshness, disclosures,
both cutoffs, policy versions, and remediation. Numeric program scores and
ranks are withheld for `population_incomplete`,
`depth_incomplete`, `score_withheld`, or `rank_withheld` states.

**Acceptance**

- The nine current partial organizations expose causal stage-level shortfalls.
- No missing player receives an imputed score or placeholder rank.
- Rangers and Kraken output remains stable unless new source facts legitimately
  expand their populations.
- Publication depth does not alter organization scores.
- The strict gate writes no ranking artifact on incomplete population authority
  or depth, while the audit/source package remains writable;
- Team and league counts reconcile by source family, player class, and
  skater/goalie group;
- CLI rows project from `ProspectCensusView`; other surfaces remain explicitly
  deferred until their parity rows and tests exist.

### S6 — Close all-32 authority through reusable adapters

**Status:** In progress — reusable audit seams now admit official
`ahl_roster_stats.v1`, reviewed AHL identity batches, exact NHL roster-player
landing joins, multi-year official draft fan-in, and a provider-neutral
`contract_control_ledger.v1`. Contract coverage is exhaustive and fail-closed;
`camp_participation_ledger.v1` preserves attendance-only semantics, and the
generic `identity_review_ledger.v1` now resolves any staged proposal family
without provider-specific package logic. Authorized all-32 upstream ledgers
are still required before population or control authority can be claimed. See
[`../notes/2026-07-31-icelines-sources-s6-authority-closure.md`](../notes/2026-07-31-icelines-sources-s6-authority-closure.md).

Use S5 diagnostics to add only provider-level capabilities:

- official CHL current roster/stat adapters when landing history is stale or
  incomplete;
- official NCAA assignment/stat adapters;
- official SHL, Liiga, KHL, and other European assignment/stat adapters in the
  order justified by measured exclusions; and
- goalie-specific source facts where skater-shaped histories are insufficient.

Do not add a league adapter merely because it is plausible. Promote it only
when the exclusion audit identifies affected canonical players and the source
has a fixture, freshness policy, identity join, and replay contract.

Build a frozen human-reviewed validation corpus across teams, positions,
leagues, traded/expired/unsigned rights, and camp invite states. It measures
candidate recall, false-control claims, unresolved identity, and rank changes.

**Acceptance**

- Strict prospect publication passes both population authority and requested
  ranking depth for every season-canonical organization from sealed packages.
- Source-package audit has zero unexplained candidate losses.
- Remaining exclusions are legitimate policy exclusions, not missing source
  plumbing.
- The all-32 result is reproducible offline and from a clean cache bootstrap.
- A sealed all-32 reference package rebuilds offline to the identical
  fingerprint from frozen raw fixtures; a separate live canary is informative;
- The validation corpus has zero false control claims and publishes remaining
  recall limitations rather than asserting an unsupported percentage.

### S7 — Incremental source extraction

**First completed slice:**
[`../notes/2026-08-01-icelines-sources-s7-contracts-csv.md`](../notes/2026-08-01-icelines-sources-s7-contracts-csv.md).
The shared team catalog/provider-normalization slice is recorded in
[`../notes/2026-08-01-icelines-sources-s7-teams.md`](../notes/2026-08-01-icelines-sources-s7-teams.md).
The first non-prospect reuse proof is
[`../notes/2026-08-01-icelines-sources-s7-yahoo-eligibility.md`](../notes/2026-08-01-icelines-sources-s7-yahoo-eligibility.md).
The player-level MoneyPuck parser extraction is
[`../notes/2026-08-01-icelines-sources-s7-moneypuck.md`](../notes/2026-08-01-icelines-sources-s7-moneypuck.md).
The AHL transaction transport/parser split is
[`../notes/2026-08-01-icelines-sources-s7-ahl-transactions.md`](../notes/2026-08-01-icelines-sources-s7-ahl-transactions.md).
The provider DTO schema extraction that completes the original pure-move set is
[`../notes/2026-08-01-icelines-sources-s7-schema.md`](../notes/2026-08-01-icelines-sources-s7-schema.md).
The first mixed NHL API parser slice is
[`../notes/2026-08-01-icelines-sources-s7-nhl-landing-contract.md`](../notes/2026-08-01-icelines-sources-s7-nhl-landing-contract.md).
The standings parser/core-projection slice is
[`../notes/2026-08-01-icelines-sources-s7-nhl-standings.md`](../notes/2026-08-01-icelines-sources-s7-nhl-standings.md).
The schedule-game DTO/parser slice is
[`../notes/2026-08-01-icelines-sources-s7-nhl-schedule.md`](../notes/2026-08-01-icelines-sources-s7-nhl-schedule.md).
The playoff-bracket DTO/parser slice is
[`../notes/2026-08-01-icelines-sources-s7-nhl-playoff-bracket.md`](../notes/2026-08-01-icelines-sources-s7-nhl-playoff-bracket.md).
The coupled boxscore/play-by-play gamecenter family is
[`../notes/2026-08-01-icelines-sources-s7-nhl-gamecenter.md`](../notes/2026-08-01-icelines-sources-s7-nhl-gamecenter.md).
The official shift-chart provider-contract slice is
[`../notes/2026-08-01-icelines-sources-s7-nhl-shift-chart.md`](../notes/2026-08-01-icelines-sources-s7-nhl-shift-chart.md).
The position-boxscore/game-log provider-contract slice is
[`../notes/2026-08-01-icelines-sources-s7-position-boxscore.md`](../notes/2026-08-01-icelines-sources-s7-position-boxscore.md).
The MoneyPuck team-game and goalie-game parser slices are
[`../notes/2026-08-01-icelines-sources-s7-moneypuck-games.md`](../notes/2026-08-01-icelines-sources-s7-moneypuck-games.md).
The opt-in CapWages provider-contract split is
[`../notes/2026-08-01-icelines-sources-s7-capwages.md`](../notes/2026-08-01-icelines-sources-s7-capwages.md).
The embedded/installed artifact and historical playoff-bundle parser splits are
[`../notes/2026-08-01-icelines-sources-s7-bundled-artifacts.md`](../notes/2026-08-01-icelines-sources-s7-bundled-artifacts.md).
The ESPN transaction conversion/page-parser split is
[`../notes/2026-08-01-icelines-sources-s7-transactions.md`](../notes/2026-08-01-icelines-sources-s7-transactions.md).
The Tier-1 stats report decoder split is
[`../notes/2026-08-01-icelines-sources-s7-stats-loader.md`](../notes/2026-08-01-icelines-sources-s7-stats-loader.md).
The official NHL search/landing identity parser split is
[`../notes/2026-08-01-icelines-sources-s7-official-identity.md`](../notes/2026-08-01-icelines-sources-s7-official-identity.md).
The AHL HockeyTech catalog/roster/stat parser split is
[`../notes/2026-08-01-icelines-sources-s7-ahl-hockeytech.md`](../notes/2026-08-01-icelines-sources-s7-ahl-hockeytech.md).
The player-landing career/organization/awards parser split is
[`../notes/2026-08-01-icelines-sources-s7-career-landing.md`](../notes/2026-08-01-icelines-sources-s7-career-landing.md).
The closing mixed-module responsibility audit is
[`../notes/2026-08-01-icelines-sources-s7-boundary-audit.md`](../notes/2026-08-01-icelines-sources-s7-boundary-audit.md).
The consumer-reuse and release-gate closeout is
[`../notes/2026-08-01-icelines-sources-s8-closeout.md`](../notes/2026-08-01-icelines-sources-s8-closeout.md).

After the prospect vertical slice proves the boundary, migrate existing pure
provider parsers from `icelines-fetch` in bounded families:

1. NHL roster/stats/contracts;
2. schedule/gamecenter/shift payloads;
3. AHL snapshots, affiliations, and identity proposals;
4. transactions;
5. MoneyPuck; and
6. other provider imports.

Keep transport, FLETCH, snapshot, manifest, database, and feature assemblers in
their owning crates. Move downstream prospect builders only through a separate
reviewed `icelines-prospects` decision.

**Acceptance per family**

- old and new API paths compile during the compatibility window;
- golden normalized output is unchanged or explicitly versioned;
- cache and snapshot identity are unchanged by a parser-only move;
- offline, stale, drift, and malformed fixtures pass; and
- no UI surface imports provider DTOs.

### S8 — Consumer reuse, documentation, and release

Prove at least one non-prospect consumer reads the same normalized facts. Good
initial candidates are training camp, fantasy injury/availability, transaction
analysis, or organization-health source packages.

Update architecture, data-source, FLETCH, cache, command, surface-parity, and
schema documentation not already updated with each slice. Publish migration and
authoring guidance plus release notes.

**Acceptance**

- CLI, TUI, Web, card, fantasy, and simulation boundaries are accurately
  documented.
- Full workspace tests, strict Clippy, schema/golden checks, offline smoke,
  dependency audit, and packaging gates pass.
- Release notes distinguish architectural extraction from newly promoted data
  authority.

## Verification matrix

| Risk | Required evidence |
|---|---|
| Crate cycle or wrong dependency direction | `cargo tree`, architecture test, workspace check |
| Semantic drift during extraction | before/after golden normalized artifacts |
| Identity leakage | ambiguous/same-name/provider-ID fixtures and review application tests |
| Attendance promoted to control | camp invite/unknown/controlled fixtures |
| Historical leakage | independent effective/knowledge cutoff fixtures, including retroactive reports and later identity review |
| Provider schema drift | additive, missing required, type-change, malformed, empty, and partial fixtures |
| False population completeness | enumerated source matrix, terminal pagination, frozen human-reviewed corpus |
| Rights overclaim | CHL/NCAA/Europe, unsigned, transferred, expired, unknown, and conflicted fixtures |
| Goalie omission | separate skater/goalie funnel counts and score-withholding fixtures |
| Cache regression | mock fetch, cache hit, stale fallback, seal/activate refusal, offline replay |
| Special casing | static search plus all-32 permutation/order-invariance tests |
| Rank inflation | incomplete census, graduate boundary, missing career, and no-output strict-gate tests |
| Surface divergence | CLI projection from `ProspectCensusView`; parity tests for every later promoted surface |

## Commit-sized sequence

1. Inventory, crate scaffold, actual-DAG docs, and dependency gates.
2. Player-landing extraction with compatibility re-export.
3. Core fact contracts and source-package schema.
4. Draft adapter and fixtures.
5. Club-publication adapter and fixtures.
6. Contract/transaction reconciliation.
7. Coverage funnel and existing prospect composition.
8. Measured league adapters required by exclusions.
9. All-32 strict publication proof.
10. Non-prospect reuse, remaining parser extractions, documentation
    consolidation, and release.

Each commit must compile and preserve existing tests. No commit combines crate
scaffolding, semantic schema change, and consumer cutover.

## Rollback and compatibility

- Parser extraction is reversible while `icelines-fetch` re-exports the new
  implementation.
- New source packages are additive; existing sealed inputs remain readable.
- Active snapshot pointers are never rewritten by source-package migration.
- A failed new adapter leaves the prior production path intact and reports an
  unavailable source family.
- The old prospect overlay path remains supported until the new census has an
  offline all-32 proof and explicit deprecation release.

## Completion definition

This plan is complete only when the crate boundary is enforced, the source
package is reusable, all season-canonical organizations pass independent
population-authority and ranking-depth gates without team exceptions, the
validation corpus reports zero false control claims, a non-prospect consumer
reuses the facts, and existing saved artifacts retain documented compatibility.
