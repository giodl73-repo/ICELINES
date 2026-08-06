# IceLines Sources — Provider Normalization Architecture

**Date:** 2026-07-31
**Status:** Proposed
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Role review:** [`../notes/2026-07-31-icelines-sources-roles-review.md`](../notes/2026-07-31-icelines-sources-roles-review.md)

## Decision

Add one workspace crate named `icelines-sources`.

`icelines-sources` owns provider payload parsing, semantic validation, and
normalization into source-neutral IceLines facts. It does not fetch bytes,
cache or activate snapshots, resolve product policy, calculate scores, or
render a user surface.

The crate is deliberately shared. Draft, roster, contract, assignment, injury,
transaction, camp, career, and league-stat facts describe the same player
universe used by prospects, fantasy, simulation, lineups, trades, and
organization health. They must not be implemented as prospect-only sources.

## Problem

`icelines-fetch` currently combines several responsibilities:

1. HTTP and FLETCH acquisition;
2. retry, cache, lock, snapshot, and activation policy;
3. external provider DTOs and parsers;
4. canonical identity proposals and review workflows;
5. provider-to-IceLines normalization;
6. feature-specific source assembly; and
7. some downstream prospect and prediction composition.

That concentration makes source reuse difficult and obscures why a downstream
row is absent. The prospect-program proof demonstrates the consequence: the
ranking engine works, but nine organizations lack 17 eligible studies because
candidate-population coverage is not yet a first-class upstream contract.

The missing capability is not a team exception list. IceLines needs a dated,
auditable player-fact layer that can distinguish:

- a player discovered by a provider from a canonically resolved player;
- attendance from organizational control;
- an event from the state inferred from a sequence of events;
- current evidence from historical evidence;
- absence from a source from an affirmative negative fact; and
- source completeness from downstream eligibility or score quality.

## Goals

1. Give every provider parser one reusable, testable ownership boundary.
2. Preserve `PlayerId` and `PlayerIdentity` as canonical core concepts.
3. Keep raw-byte acquisition, caching, snapshot sealing, and activation in
   `icelines-fetch` and FLETCH.
4. Emit neutral player, organization, participation, assignment, contract,
   transaction, availability, and performance facts.
5. Keep unresolved provider identities out of canonical player facts until an
   explicit review application succeeds.
6. Make source coverage and exclusions machine-readable before scoring.
7. Preserve existing public Rust paths and serialized artifacts during an
   incremental migration.
8. Support exact offline replay from sealed source bytes and manifests.
9. Improve the all-32 prospect census without coupling the source crate to the
   prospect ranking policy.

## Non-goals

- `icelines-sources` does not become another HTTP client.
- It does not own FLETCH plans, cache TTLs, snapshot activation, SQLite, or the
  filesystem.
- It does not decide whether a player is a prospect, rookie, fantasy add,
  lineup fit, trade target, or simulation breakout.
- It does not rank provider trust with an opaque numeric score.
- It does not turn camp attendance into draft rights or a contract.
- It does not silently resolve names to NHL IDs.
- It does not require a simultaneous rewrite of every `icelines-fetch` module.
- It does not create one crate per league or one adapter per NHL team.

## Workspace topology

The target edges follow the actual workspace direction; the arrows below mean
"depends on":

```text
icelines-query   -> icelines-core
icelines-sources -> icelines-core
icelines-fetch   -> icelines-core, icelines-query, icelines-sources
icelines-site    -> icelines-core, icelines-fetch
icelines-web     -> icelines-core, icelines-query, icelines-fetch
icelines-cli     -> the applicable core/query/fetch/site/web libraries
```

The existing fetch-to-query edge remains permitted for feature-domain
composition during this migration. Nothing may introduce query-to-fetch or
sources-to-fetch. `design/ARCHITECTURE.md` must be corrected to this actual DAG
in the crate-scaffold slice, before parser extraction.

Dependency rules:

- `icelines-core` must not depend on `icelines-sources` or `icelines-fetch`.
- `icelines-sources` may depend on `icelines-core`, serialization, and
  deterministic parsing libraries.
- `icelines-sources` must not depend on `reqwest`, Tokio runtime features,
  FLETCH, `rusqlite`, Axum, Ratatui, or a renderer crate.
- `icelines-fetch` depends on `icelines-sources` and remains the acquisition
  facade used by commands and applications.
- UI crates should not parse provider payloads or reproduce reconciliation.
- A future `icelines-prospects` crate may consume core facts and own prospect
  policy, but its creation is independent of this source boundary.

## Ownership boundary

| Concern | Owner |
|---|---|
| `PlayerId`, `PlayerIdentity`, team and season identity | `icelines-core` |
| Source-neutral evidence and resolved fact envelopes | `icelines-core` |
| Raw provider DTO and required-field validation | `icelines-sources` |
| Provider-local identity and canonical identity proposal | `icelines-sources` |
| Explicit review decisions and their source evidence | existing review workflow, progressively moved behind `icelines-sources` services |
| URL expansion and request execution | `icelines-fetch` / FLETCH |
| Retry, pacing, stale behavior, locks, cache manifests | `icelines-fetch` / FLETCH |
| Snapshot seal, active pointer, installed bundle | `icelines-fetch` |
| Prospect eligibility, graduation, scoring, top ten | prospect domain builders, currently core/fetch and eligible for a later `icelines-prospects` extraction |
| CLI, TUI, Web, cards | shared ViewModels and renderers; never provider parsing |

## Canonical fact contracts

Source facts are append-only observations. A correction supersedes or retracts
a prior assertion through a new reviewed assertion; it does not rewrite a
sealed artifact. Fact domain time is never inferred from evidence capture time.

The source-neutral contracts belong in `icelines-core` because consumers must
not depend on provider adapters:

```rust
pub struct SourceEvidence {
    source_id: SourceId,
    source_url: SourceUrl,
    provider: ProviderId,
    captured_at: DateTime<Utc>,
    content_sha256: ContentHash,
    adapter_version: AdapterVersion,
}

pub struct FactAssertion<T> {
    fact_id: FactId,
    subject: FactSubject,
    occurred_at: EffectiveTime,
    fact: T,
    evidence: NonEmpty<SourceEvidence>,
    supersedes: Vec<FactId>,
    retracts: Vec<FactId>,
}

pub struct ProviderIdentityProposal {
    proposal_id: ProposalId,
    locator: ProviderPersonLocator,
    displayed_name: String,
    birth_date: Option<NaiveDate>,
    proposed_player_id: Option<PlayerId>,
    evidence: NonEmpty<SourceEvidence>,
}

pub struct StagedPlayerAssertion {
    assertion_id: StagedAssertionId,
    semantic_key: String,
    proposal_id: ProposalId,
    occurred_at: EffectiveTime,
    authority: FactAuthority,
    fact: SourceFact,
    evidence: NonEmpty<SourceEvidence>,
}

pub struct IdentityReviewDecision {
    decision_id: DecisionId,
    proposal_id: ProposalId,
    action: IdentityReviewAction,
    canonical_player_id: Option<PlayerId>,
    reviewer: String,
    reviewed_at: DateTime<Utc>,
    rationale: String,
    evidence: Vec<SourceEvidence>,
}
```

`FactSubject` is a typed reference (`Player`, `Organization`, `Team`, `Game`, or
`League`) so later schedule, shift, and game adapters are not distorted into
player facts. Player assertions accept only a validated, non-zero canonical
`PlayerId`; private fields, constructors, and custom deserialization enforce
that invariant. IDs, hashes, URLs, adapter versions, and evidence collections
use validated types rather than unchecked public strings.

`ProviderPersonLocator` supports either a stable provider player ID or a
source-scoped row locator plus displayed identity attributes. Proposals are
immutable and review decisions are separate append-only records. A proposal
can enter a review queue but cannot enter scoring, roster membership, or
canonical facts. A `StagedPlayerAssertion` preserves the hockey fact waiting
on that review so sealed-package replay cannot remember the person while
losing the event. Neither record can establish organization control until a
compatible decision is applied. Existing AHL
review documents lower through an explicit compatibility adapter.

### Events, relationships, and participation are separate

The current prospect population compatibility schema has one `relationship`
field. It remains readable, but the next general fact model must not keep
conflating control, assignment, and attendance.

```rust
pub enum PlayerOrganizationEvent {
    Drafted { by: OrganizationId, year: u16, round: u8, overall: u16 },
    ContractSigned { with: OrganizationId, contract_kind: ContractKind },
    RightsTransferred { from: OrganizationId, to: OrganizationId },
    RightsExpired { organization: OrganizationId },
    Assigned { by: OrganizationId, to: ClubRef },
    Rostered { at: ClubRef },
    // Discovery/club context only; an affiliate roster is not NHL control.
    AffiliateRostered { affiliate: OrganizationId, at: ClubRef },
    Recalled { by: OrganizationId, from: ClubRef, to: ClubRef },
    Loaned { by: OrganizationId, to: ClubRef },
    Released { by: OrganizationId },
}

pub struct PlayerParticipationFact {
    pub organization: OrganizationId,
    pub season: Season,
    pub kind: ParticipationKind,
    pub authority: ParticipationAuthority,
}

pub enum ParticipationAuthority {
    ControlledPlayer,
    FreeAgentInvite,
    Tryout,
    Unknown,
}
```

A participation row with `Unknown` authority is still useful to camp
simulation. It cannot independently establish organization control. A dated
state resolver may infer current control only from compatible events and the
applicable, versioned policy. Source adapters emit observations; they do not
hide that inference inside parsing.

Organization-state output is independently keyed by `(player_id,
organization, as_of, resolver_policy_version)`. Rights state is explicitly
`Supported`, `Expired`, `Transferred`, `Unknown`, or `Conflicted`, with reason
codes and required-input disclosures. A draft event proves historical
selection only. It never proves indefinite current control, and only
`Supported` reaches the controlled stage.

The fact ledger is separate from `StatsRepository`. Reviewed identity-stable
corroboration may update `PlayerIdentity` through existing merge rules;
performance enters `SeasonStats` only through the existing `(player_id,
season, season_type)` invariants; organization state remains in the separately
keyed resolver output.

## Source families

| Family | Normalized facts | Current IceLines state | Priority |
|---|---|---|---|
| NHL player landing | identity corroboration, birth date, career totals, awards, contract hints | draft/career/organization/award and contract-hint parsing implemented in `icelines-sources`; endpoint acquisition, local stores, and award ViewModel construction remain in `icelines-fetch` | compatibility facade retained |
| NHL rosters and stats REST | roster participation, bio, season totals, TOI | provider DTO schema and serde validation implemented in `icelines-sources`; endpoint acquisition and snapshot loading remain in `icelines-fetch` | migrate endpoint parsers incrementally |
| Reviewed contract CSV | season monetary values, expiry hints, source URL, source check time | deterministic byte parser implemented in `icelines-sources`; file loading and selected-bios identity join remain in `icelines-fetch` | compatibility facade retained |
| CapWages opt-in contract API | contract values, expiry hints, provider identity | response DTOs, season/name normalization, and contract projection implemented in `icelines-sources`; credentials, paging, concurrency, and acquisition remain in `icelines-fetch` | licensed opt-in source; absence remains unknown, never zero |
| NHL player search and landing REST | canonical player-ID proposal, birth-date corroboration, draft coordinates, career evidence | deterministic search/landing identity parsers and source adapter implemented in `icelines-sources`; acquisition, cache replay, candidate eligibility, and review finalization remain in `icelines-fetch` | expand beyond current rosters only through measured identity gaps; parsed proposals never self-approve |
| Contract-control ledger | canonical player-to-organization control facts with exhaustive terminal coverage | provider-neutral `contract_control_ledger.v1` adapter and audit seam implemented | connect an authorized all-32 provider; never infer from roster membership |
| Camp-participation ledger | canonical development/rookie/training/tournament attendance with exhaustive terminal coverage | provider-neutral `camp_participation_ledger.v1` adapter and audit seam implemented | connect reviewed all-32 publications; attendance never implies control |
| NHL standings, schedule, score, boxscore, play-by-play, shifts | standings rows/core projection, scheduled-game and playoff-bracket interpretation, boxscore stats, event-level scoring projections, and official shift intervals now implemented in `icelines-sources` | parser/DTO families migrated with fetch compatibility facades; shift-overlap feature aggregation remains outside the source boundary | preserve capability gates and feature ownership |
| NHL draft results | selection event and initial organization | no complete reusable ledger | prospect-census priority |
| NHL transactions and club releases | rights movement, signing, recall, assignment, release | ESPN page parsing, drift accounting, date/team normalization, classification projection, and stable IDs implemented in `icelines-sources`; HTTP/cache/snapshot paths remain in `icelines-fetch`; authored official evidence remains separate | expand official authority without treating ESPN as legal-control proof |
| NHL club development/rookie/training publications | participation, position, prior club, acquired/invite labels | small authored overlay only | highest population priority |
| AHL roster/statistics/affiliations/transactions | provider identity, assignment, development performance, dated ADD/DEL evidence | HockeyTech JSONP/catalog/roster/skater/goalie DTO parsing and validation, canonical identity proposal normalization/merge, roster adapter, and transaction DTO/page parsing implemented in `icelines-sources`; acquisition, FLETCH, crosswalk review, and projection composition remain in `icelines-fetch` | preserve explicit review semantics and provider-local IDs |
| CHL/NCAA/USHL/European official sources | current assignment and fresh development performance | career history often available after identity; no complete current-assignment layer | follow population census |
| Yahoo fantasy import | local eligibility and fantasy membership only | eligibility byte parser and normalized rows implemented in `icelines-sources`; file loading remains in `icelines-fetch` | non-prospect reuse proof; remain isolated from hockey identity authority |
| MoneyPuck | advanced performance facts | player, team-game, and goalie-game CSV contracts/parsing implemented in `icelines-sources`; trailing model features and endpoint/acquisition composition remain in `icelines-fetch` | compatibility facades retained; feature ownership stays outside source parsing |
| Media/podcast/analyst evidence | attributed context and traits | evaluation/context lanes | never override official identity/control facts |

Official club publications require one semantic adapter family, not 32
team-specific domain implementations. Provider layout variants may have
separate parsers behind the same output contract and fixture suite.

The contract-control import is defined by
[`../schemas/contract_control_ledger.v1.schema.json`](../schemas/contract_control_ledger.v1.schema.json).
Schema validation is necessary but not sufficient: the adapter additionally
requires unique team coverage, unique canonical player IDs, exact per-team
count reconciliation, rows belonging only to covered teams, and no effective
time after capture.

The canonical camp import is defined by
[`../schemas/camp_participation_ledger.v1.schema.json`](../schemas/camp_participation_ledger.v1.schema.json).
It has the same exact-scope and count-reconciliation requirements, while its
facts retain `FactAuthority::Attendance` regardless of the publication's
attendee classification.

Finalized cross-provider identity decisions use
[`../schemas/identity_review_ledger.v1.schema.json`](../schemas/identity_review_ledger.v1.schema.json).
The ledger is proposal-family neutral: draft, camp, transaction, contract, AHL,
and future league proposals share the same decision contract. Package sealing
binds every decision to an acquired proposal and rejects duplicate or stale
review application.
Every decision evidence item carries its own source ID, URL, provider, capture
time, content SHA-256, and adapter version; a registry hash is never mislabeled
as the hash of an external source document.

The paired `identity_review_workboard.v1` is a UI-neutral read model derived
only from a validated sealed package. It lists unresolved proposals with their
staged hockey context and evidence, and excludes already decided rows. It is
not a mutable registry, does not perform fuzzy matching, and does not assign
confidence; clients render or use it to acquire evidence for a separate
finalized identity-review ledger.
Its serialized contract is frozen in
[`../schemas/identity_review_workboard.v1.schema.json`](../schemas/identity_review_workboard.v1.schema.json).
Draft contexts carry structured organization/year/round/overall coordinates in
addition to their display label and retain the full hashed proposal evidence,
not only display URLs. `icelines-fetch` consumes those coordinates
through `official-identity-candidates`, deduplicates exact-name search
cachelines, acquires every candidate landing through FLETCH, and emits the
UI-neutral `official_identity_candidate_board.v1` evidence artifact. Search and
landing evidence each retain the capture timestamp and SHA-256 of their own raw
provider bytes. The artifact preserves the base package cutoff separately from
its evidence cutoff: live evidence is available only to a subsequent package
seal, while `--evidence-cutoff` enforces an explicit replay horizon. `--offline`
reads verified search and landing cachelines through one manifest snapshot and
classifies every unavailable object without contacting a provider. A row is
eligible only when every exact-name candidate has a
valid landing, the landing identity agrees with the exact search name and any
proposal birth date, and exactly one landing matches all four draft coordinates.
Surname expansion, fuzzy similarity, and name-only results never grant
eligibility, and the board itself never mutates identity authority. Its schema
is [`../schemas/official_identity_candidate_board.v1.schema.json`](../schemas/official_identity_candidate_board.v1.schema.json).
`official-identity-review-ledger` is the explicit authority step: it requires a
reviewer, review timestamp, and absolute registry URL, includes both the exact
draft proposal evidence plus the exact search and landing evidence for every
eligible row, and emits the same generic
`identity_review_ledger.v1` accepted by the source-package audit. Non-eligible
rows are never converted.

## Provider adapter contract

Every adapter exposes a deterministic parse operation over supplied bytes:

```rust
pub trait SourceAdapter {
    type Output;

    fn descriptor(&self) -> SourceDescriptor;
    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError>;
}
```

`SourceInput` contains bytes plus acquisition evidence supplied by
`icelines-fetch`. It contains no network handle. `SourceDescriptor` declares:

- stable source and adapter IDs;
- provider and payload family;
- supported schema/layout versions;
- required identity keys;
- additive-field policy;
- freshness class;
- historical availability;
- whether absence has semantic meaning; and
- output fact families.

All adapters return a common `AdapterError` envelope naming source, adapter,
input fingerprint, error category, and record/package disposition. Existing
structured NHL and ESPN DTOs retain their load-bearing
`serde(deny_unknown_fields)` behavior during extraction. A different adapter
family may tolerate additive fields only through an explicit architecture
amendment, versioned drift policy, frozen mutation fixture, and promotion gate.

| Condition | Disposition |
|---|---|
| unsupported envelope or layout | fatal for the source object |
| missing/type-changed identity or control field | fatal or quarantined, never defaulted |
| malformed non-authority record | quarantined with a typed exclusion |
| explicitly approved additive drift | recorded informational drift |

A quarantined record makes the affected requested scope incomplete. HTML
publication adapters validate structural anchors and terminal pagination in
addition to required record fields.

## Acquisition and storage boundary

`icelines-fetch` remains responsible for:

1. expanding a domain request into URLs;
2. asking FLETCH for reusable bytes;
3. recording request and cache evidence;
4. passing immutable bytes and capture metadata to an adapter;
5. sealing raw and normalized artifacts;
6. updating active pointers only after validation; and
7. returning stale data or a typed failure according to existing policy.

`icelines-sources` never reads the active snapshot implicitly. Replays provide
the exact captured bytes or a sealed package reference.

Source packages are snapshot-owned derived artifacts, not a new fallback tier.
They live at `sources/<package_id>/package.json` under the producing sealed
snapshot; raw-input references and integrity hashes live in the versioned
snapshot manifest. The snapshot activates atomically only after every required
raw input and normalized package validates. `LoadOutcome.missing` reports
missing required source families distinctly from authoritative-empty results.
An older binary either reads only its known manifest version without mutating
the package or returns the existing typed unsupported-version failure; it never
silently activates or rewrites an unknown package. Queries never perform live
acquisition as fallback; fetch/install paths remain the only network writers.

## Source package

The shared output envelope is `icelines_source_package.v1`:

```text
schema
package_id
evaluation_season
effective_cutoff
knowledge_cutoff
adapter_registry_version
reconciliation_policy_version
review_registry_fingerprint
run_manifest          request scope, catalog version, expanded objects/outcomes
inputs[]              URL, capture/freshness state, hash, adapter ID/version
fact_assertions[]
identity_proposals[]
identity_review_decisions[]
conflicts[]
coverage
disclosures[]
```

The run manifest records expected and expanded object IDs, per-object
success/failure/not-applicable state, terminal pagination evidence, capture
metadata, and package completeness. Only a complete successful source set can
assert an authoritative empty result. Partial packages remain auditable but
cannot pass strict publication.

Freshness is computed by `icelines-fetch` against the injected package cutoff,
never the adapter wall clock, and serializes freshness class, captured time,
evaluated-at, status (`fresh`, `stale`, `static`, or `unknown`), and policy
version.

The package is UI-neutral and deterministic. Semantically unordered inputs,
facts, and evidence are canonically sorted by stable typed keys before hashing;
order is retained only where the domain declares it meaningful. The fingerprint
covers canonical serialized bytes, input identities and hashes, adapter and
policy versions, both cutoffs, review registry, and evaluation scope. A source
package does not contain product scores.

`coverage` is versioned and reports expected, acquired, parsed, quarantined,
resolved, conflicted, and excluded counts by source family, organization,
player class, and position group. Count invariants reconcile it to manifest,
fact, conflict, and exclusion rows. `disclosures` uses stable codes, including
`partial_population`, `missing_source_family`, `stale_source`,
`unresolved_identity`, `conflicting_control`, `participation_only`,
`historical_cutoff`, `rights_policy`, and `unsupported_league`.

## Reconciliation

Reconciliation is evidence-based and typed:

- compatible assertions merge under a canonical semantic fact key and retain
  every evidence item;
- assertion identity and evidence identity remain separate;
- incompatible canonical identity, organization, or effective-date facts
  produce a conflict row;
- a later capture does not automatically supersede an earlier effective fact;
- absence from camp, a roster, or a search result is not a release event;
- typed authority is defined per fact family: draft, attendance, contract,
  legal control, assignment, and contextual evidence never substitute for one
  another;
- conflicting official assertions remain conflicts unless a versioned resolver
  names the governing fact-family authority; no global trust score is invented;
- every inferred current state names the resolver policy version and all input
  facts used.

Replay uses two clocks. `knowledge_cutoff` constrains capture and review
decisions; `effective_cutoff` constrains domain events. A documented
`reconstructed_identity` mode may apply a later identity review to an earlier
observation, but records the later review-registry fingerprint and cannot import
later performance or organization facts. Default `as_known_then` mode requires
reviews to satisfy the knowledge cutoff.

## Prospect census consumer

The first major consumer is a provider-neutral organization prospect census.
It consumes resolved source facts and produces this coverage funnel for every
season-canonical NHL organization:

```text
discovered
  -> canonical identity reviewed
  -> organizational control supported
  -> prospect policy eligible
  -> career evidence usable
  -> development study built
  -> ranked
```

Counts and typed exclusions are emitted at every transition. A top-ten
shortfall therefore identifies whether the missing authority is population,
identity, control, eligibility, career evidence, or study construction. The
census does not manufacture a tenth player and does not weaken the ranking
gate.

`ranking_depth_complete` and `population_authority_status` are independent.
Ten eligible studies prove depth only; they do not prove that a better player
was discovered. Population authority requires a dated, enumerated acquisition
matrix for every organization and applicable source family: draft ledger,
contract/rights ledger, current assignments, and published camp sources. Every
expected object must be acquired, validated, not-applicable with a reason, or a
typed blocker. Every discovered identity/control conflict must be resolved or
excluded. Rankings and organization scores are withheld unless population
authority passes.

The initial population union is:

1. multi-year official NHL draft selections, retained as historical selection
   events until a versioned rights resolver independently establishes current
   control;
2. current NHL/AHL contract and assignment facts;
3. official development, rookie, and training-camp publications with explicit
   acquired/invite classification;
4. reviewed rights transfers and releases; and
5. applicable official current-assignment facts from amateur and European
   leagues, with unsupported leagues disclosed rather than silently omitted.

Goalie population and usable goalie evidence are named coverage dimensions,
not optional cleanup. Team and league output reports skater/goalie discovery,
identity, control, evidence, and exclusion counts separately. Program-health or
positional-balance ranks remain withheld when required goalie authority fails.

Consumers receive a UI-neutral `ProspectCensusView`, not the raw source package.
It includes evaluation organization/season, both cutoffs, authority and
freshness state, funnel and positional counts, typed exclusions/disclosures,
policy versions, and remediation guidance. Failed, quarantined, and
incomplete-pagination source objects are retained as typed per-team authority
gaps. A league summary groups those gaps by source family and state and counts
affected organizations, allowing every renderer to distinguish a missing
all-32 provider family from isolated team failures without parsing disclosure
text. The first required surface is
`icecast prospect-census --source-package PATH [--json]`. TUI/Web/card
renderers are explicitly deferred until their
`surface-parity.md` rows and row-identity/partial-state tests are added; no
surface reimplements census logic.

`prospect_census_readiness_board.v1` is the compact, UI-neutral publication
projection of that census. It retains the 32-team funnel totals, independent
population/depth/publication gates, typed authority-gap and player-loss
summaries, remediation, the sealed package fingerprint, and a fingerprint of
the complete source census. It intentionally omits player-level loss rows so a
renderer can load the league readiness state without loading the full audit
ledger. `icecast prospect-census-readiness --input PATH [--json]` builds this
projection only after team, league, loss, and authority-gap totals reconcile.
It does not replace the census or grant publication authority. The retained
board is projected without re-scoring through CLI, a two-page TUI league view,
and Web HTML/JSON routes. Web team focus is a row filter over the same sealed
artifact; every surface preserves its fingerprint and withheld publication
state.

`prospect_authority_closure_board.v1` turns those typed gaps into an operator
workboard without crossing the acquisition boundary. Each team/family cell
retains its failed, quarantined, or incomplete-pagination state; names the
population or organizational-control gate it blocks; and, where registered,
names the exact provider-neutral artifact schema and existing ingestion option.
The builder validates the source readiness fingerprint and reconciles family,
team, and league cell counts. Closing a recipe requires a new source-package
and census replay; editing the closure board cannot change readiness.

The retained August 5 replay demonstrates that lifecycle. An official
2026-27 AHL snapshot supplied exact affiliate coverage with explicit empty
preseason rosters. Replay raised acquired manifest objects from 128 to 160 and
removed all 32 AHL source gaps, while controlled-player counts correctly stayed
at zero. The remaining 64 cells are the 32-team camp and contract families.

## Compatibility and migration

Migration is facade-first and build-green:

- Existing `icelines-fetch` public paths remain available through temporary
  `pub use icelines_sources::...` re-exports.
- Existing serialized schemas remain readable.
- Moving a parser without changing semantics does not change its artifact
  schema or method version.
- A semantic change requires a new schema/method or an explicit compatibility
  bridge; extraction alone is not permission to reinterpret old data.
- No phase may leave CLI, TUI, Web, tests, or packages uncompilable.
- Deprecated re-exports receive a documented removal milestone only after all
  workspace consumers migrate.

`prospect_population_overlay.v1` remains supported. A future v2 lowering maps
its compatibility relationship into separate event/participation facts while
retaining the original source and classification disclosure.

## Failure and offline behavior

- Missing source bytes: typed acquisition failure in `icelines-fetch`.
- Unsupported payload layout: typed adapter error with source ID and input
  fingerprint.
- Missing required semantic field: apply the common disposition matrix; never
  default identity or control, and never call quarantined scope complete.
- Additive upstream field: fail for preserved strict DTOs; otherwise apply only
  an explicitly promoted adapter-family drift policy.
- Ambiguous identity: review proposal, never canonical fact.
- Conflicting organization evidence: conflict row and withheld current state.
- Stale but valid source: retain with freshness state; never relabel as current.
- Offline replay: parse sealed bytes without network, filesystem discovery, or
  wall-clock dependence inside `icelines-sources`.

## Test contract

### L0 — adapter and fact invariants

- valid provider fixture to normalized facts;
- missing/type-changed required fields;
- additive-field drift behavior;
- non-zero canonical player IDs;
- Unicode names and accents;
- event/participation separation;
- duplicate and conflict semantics;
- deterministic ordering and fingerprints;
- no wall-clock or network dependence.

### L1 — acquisition and package integration

- `icelines-fetch` acquires mock bytes and calls the adapter;
- cache hit, stale fallback, and offline replay;
- snapshot sealing and refusal before validation;
- identity-review application and unresolved proposal retention;
- source package round-trip and compatibility fixtures.
- sealed all-32 reference package rebuilt offline to the same fingerprint;
- run-manifest empty, missing, partial-pagination, and quarantined distinctions.

### L2 — consumer behavior

- all-32 prospect census coverage funnel;
- per-team authority gaps reconcile exactly into league source-family/state
  counts and actionable remediation;
- compact readiness output reconciles to the complete census, preserves its
  fingerprint, and refuses partial league envelopes;
- authority closure recipes reconcile to readiness gaps, preserve their source
  fingerprint, and never promote a recipe into acquired evidence;
- strict publication independently refuses incomplete population authority and
  incomplete ranking depth;
- camp-only invite remains usable by camp simulation but not prospect ranking;
- CLI output projects from `ProspectCensusView`; later promoted surfaces must
  receive that same view;
- historical replay enforces effective and knowledge cutoffs independently.

### Independent census validation

A frozen, human-reviewed corpus spans teams, skaters, goalies, CHL, NCAA,
Europe, AHL, traded rights, unsigned picks, invitees, and expired rights. It
reports candidate recall, false-control classifications, unresolved-identity
rate, and team rank changes. Promotion requires zero false control claims.
Recall limitations remain published rather than converted into an unsupported
completeness percentage. A separate live canary detects current provider drift
but never replaces deterministic fixtures or the sealed reference package.

### Structural gates

- dependency-cycle and forbidden-dependency checks;
- no `reqwest`, FLETCH, SQLite, Axum, Ratatui, or CLI dependency in
  `icelines-sources`;
- no provider DTO imports in core scoring or renderer modules;
- public re-export compatibility tests during migration;
- workspace format, Clippy, test, audit, packaging, and offline smoke gates.

## Acceptance criteria

The architecture is implemented when:

1. `icelines-sources` is a workspace crate with enforced dependency rules.
2. At least one existing production source family is migrated through a
   compatibility re-export with semantic equality and an explicitly named
   canonical artifact serializer for any byte-level golden.
3. Draft, club-publication, and contract/transaction adapters can assemble a
   dated all-32 candidate census without team-specific policy branches.
4. Identity proposals, review decisions, and resolved facts remain separate.
5. The census reports every transition and exclusion in the coverage funnel.
6. Strict prospect publication succeeds only when all season-canonical
   organizations pass both population authority and requested ranking depth;
   incomplete states publish no numeric program score or ordinal rank.
7. The same source facts are consumable by at least one non-prospect feature,
   proving the crate is not a disguised prospect subsystem.
8. Existing sealed artifacts and documented commands remain readable or fail
   through an explicit version boundary.
9. The independent census corpus records zero false control claims and discloses
   remaining recall and unresolved-identity limitations.

## Deferred decisions

- Whether prospect policy later moves from core/fetch into a separate
  `icelines-prospects` crate.
- Which official amateur and European league adapters are promoted after the
  census proves which current-assignment gaps remain.
- Whether provider DTO crates should later split by independent release cadence.

These decisions do not block the `icelines-sources` boundary or the all-32
population census.
