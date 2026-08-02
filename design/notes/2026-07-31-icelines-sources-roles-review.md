# IceLines Sources — Roles Review

**Date:** 2026-07-31
**Reviewed:** [`../specs/icelines-sources.md`](../specs/icelines-sources.md) and
[`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Role source:** `.roles/ROLE.md` plus HART, KEEL, TAPE, FORGE, PACE, BENCH,
EDGE, WIRE, SCOUT, GLASS, CREST, and broadcast

## Method

Three independent passes reviewed the initial draft:

1. HART/KEEL/FORGE — canonical shape, actual crate DAG, persistence, and Rust
   invariants;
2. TAPE/WIRE/EDGE — authority, temporal truth, identity review, drift,
   completeness, and failure behavior; and
3. BENCH/PACE/SCOUT/GLASS/CREST/broadcast — validation, hockey population
   recall, goalie coverage, disclosures, and consumer delivery.

Conflicts used the repository tiebreak order. Every blocker and major finding
was resolved in the revised documents; minor findings were either incorporated
or explicitly scoped.

## Findings and disposition

| Roles | Severity | Finding | Revision |
|---|---|---|---|
| HART/TAPE | BLOCKER | Player facts lacked organization and domain-effective time, so control and replay were undefined. | Replaced the player-only envelope with typed `FactSubject`/`FactAssertion`, explicit event organizations/from/to, effective time, and separately keyed organization state. |
| KEEL | BLOCKER | The proposed diagram contradicted the real fetch-to-query dependency and could imply a cycle. | Replaced it with the actual explicit edge list and made the architecture-DAG correction part of S1. |
| TAPE/SCOUT | BLOCKER | A draft selection plus no later evidence was incorrectly capable of implying current rights. | Draft now proves historical selection only; a versioned resolver emits Supported/Expired/Transferred/Unknown/Conflicted, and only Supported reaches control. |
| FORGE/WIRE | BLOCKER | Additive-field tolerance contradicted the active strict DTO rule. | Existing NHL/ESPN DTOs preserve `deny_unknown_fields`; tolerance requires a separate architecture amendment, drift policy, fixtures, and promotion gate. |
| PACE/BENCH/SCOUT | BLOCKER | Ten eligible players did not prove that the population was complete. | Separated `population_authority_status` from `ranking_depth_complete`; scores/ranks require both and an enumerated acquisition matrix. |
| TAPE/WIRE/EDGE | MAJOR | Proposal review state was mutable and could not preserve original proposal/decision evidence. | Split immutable proposals from append-only decisions, linked by IDs, and allowed source-scoped locators for name-only publications. |
| HART/FORGE | MAJOR | Non-zero player IDs and evidence validity were asserted but not type-enforced. | Required validated newtypes, private construction/custom deserialization, and non-empty evidence tests. |
| KEEL | MAJOR | Source-package persistence, sealing, activation, fallback, and older-binary behavior were absent. | Defined packages as snapshot-owned derived artifacts with manifest hashes, atomic activation, typed missing state, and no query-time live fallback. |
| TAPE/PACE/BENCH | MAJOR | One `as_of` could not represent fact-effective time versus knowledge available at replay time. | Added effective and knowledge cutoffs, review-registry fingerprint, default `as_known_then`, and bounded reconstructed-identity mode. |
| TAPE/WIRE/BENCH | MAJOR | Successful inputs alone could not distinguish authoritative empty from failed discovery or pagination. | Added a sealed request/run manifest, expected/expanded object outcomes, terminal pagination, completeness state, and strict-publication refusal. |
| TAPE | MAJOR | Correction, retraction, deduplication, and evidence union were not representable. | Added immutable fact IDs, supersedes/retracts, semantic fact keys, separate evidence identity, conflicts, and reconciliation policy version. |
| WIRE/EDGE | MAJOR | Freshness and partial-record disposition were promised but unspecified. | Added deterministic serialized freshness plus a common fatal/quarantine/informational matrix; quarantine makes scope incomplete. |
| TAPE | MAJOR | A player-only package could not later hold team/game/league source facts. | Generalized the assertion subject to Player/Organization/Team/Game/League. |
| HART/KEEL | MAJOR | The ledger relationship to `StatsRepository` was ambiguous. | Kept the evidence ledger separate and documented one-way reviewed identity and season-stat promotion boundaries. |
| FORGE | MAJOR | Adapter APIs permitted invalid strings and had no common failure taxonomy. | Required validated source/adapter/hash/url types and a common categorized `AdapterError` envelope. |
| FORGE | MAJOR | Fingerprint order rules contradicted each other. | Canonically sort unordered collections, preserve only explicit domain order, and hash canonical serialized bytes. |
| KEEL/FORGE | MAJOR | S1 and S2 both claimed the fact contracts, breaking the intended migration boundary. | S1 now preserves the existing player-landing type; S2 introduces facts and packages separately. |
| BENCH/PACE/SCOUT | MAJOR | Internal fixtures could not measure undiscovered-player recall. | Added a stratified human-reviewed validation corpus, zero-false-control gate, recall/rank-change reporting, sealed offline reference, and separate live canary. |
| SCOUT/PACE | MAJOR | Goalie evidence was late and optional despite affecting program composition. | Made skater/goalie population and evidence first-class funnel dimensions with score withholding. |
| GLASS/BENCH | MAJOR | The raw package was incorrectly described as the shared renderer document. | Added `ProspectCensusView`; CLI is first, and other surfaces are deferred until explicit parity rows/tests. |
| GLASS/broadcast/SCOUT | MAJOR | Untyped disclosures could vanish downstream. | Added stable disclosure codes and mandatory status/cutoff/freshness fields in the census view. |
| BENCH/GLASS | MAJOR | Documentation truth was postponed to release cleanup. | Made documentation parity a per-source/per-consumer promotion rule; S8 now consolidates and publishes. |

## Minor findings incorporated

- The package uses `evaluation_season`; each fact retains its own effective
  time rather than inheriting package season.
- Byte equality is required only for a named canonical serializer; extraction
  otherwise freezes semantic equality.
- S1 names the dependency inspection mechanism and repository CI target/feature
  matrix rather than saying only “supported targets.”
- Coverage is a versioned schema with reconciliation invariants, not an
  unstructured object.
- Authority is fact-family-specific; “official” is not a universal priority.
- Partial teams expose distinct population/depth/score/rank-withheld states.

## Lens disposition

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, and broadcast
all produced material findings. CREST produced no additional finding because
the reviewed work defines data and delivery contracts rather than a visual
composition.

## Result

The revised plan is ready to begin at S0. It does not authorize implementation,
commit, push, merge, release, or TRACKER submodule-pointer updates by itself.
