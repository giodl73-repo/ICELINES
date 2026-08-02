# IceLines Sources S3 — Prospect Population Progress

**Date:** 2026-07-31
**Status:** Complete vertical slice; not a complete or publishable all-32 census

## Implemented in this slice

- `OfficialNhlDraftAdapter` parses `draftDetails` from captured official NHL
  player-landing bytes. It emits historical `Drafted` authority only. An
  undrafted landing response returns an authoritative empty result; draft
  history never claims current control.
- `OfficialNhlRosterAdapter` parses the frozen official NHL roster REST shape,
  validates forward/defense/goalie groups and duplicate IDs, and emits current
  `Assigned` observations. It never emits contract or legal-control authority.
- `NhlArticleNamedSectionsCampAdapter` parses the reviewed NHL.com JSON-LD
  article layout with named forward, defense, and goalie sections. Declared
  counts are structural anchors. Because the publication has displayed names
  rather than stable player IDs, output remains identity proposals plus staged
  participation with `Unknown` authority.
- `NhlArticleAcquiredTableCampAdapter` parses the reviewed five-column NHL.com
  camp table and preserves its relationship labels. `FA Invite` is an invite,
  `ELC` is controlled participation, and draft provenance remains `Unknown`
  because a historical selection cannot prove current rights. HTML entities
  are decoded before names enter identity review, and unsupported acquisition
  labels fail closed.
- `NhlArticlePtoCampListAdapter` parses the official league-wide NHL.com PTO
  list. Explicit professional tryouts—including goalie rows—lower to `Tryout`
  participation and never to contract or control authority.
- `NhlArticleContractSigningAdapter` parses the reviewed
  `<player> signs ...` NHL.com JSON-LD headline layout and publication date.
  It emits an identity proposal plus a staged `ContractSigned` observation;
  canonical lowering must wait for identity review. Unsupported headlines
  fail closed as a new layout rather than being guessed.
- `SourceDescriptor` now declares provider, supported layouts, required
  identity keys, additive-field policy, freshness class, historical
  availability, absence semantics, and output fact families.
- `OfficialNhlTradeTrackerAdapter` stages every acquired and returned player
  leg from the reviewed NHL.com trade-tracker layout. Draft-pick assets remain
  explicit exclusions rather than disappearing from the ledger.
- `NhlArticleContractTerminationAdapter` recognizes only completed contract
  termination language; announced future intent cannot become a release fact.
- `OfficialNhlDraftPicksAdapter` parses terminal official multi-year draft
  ledgers, includes goalie selections, and stages identity proposals because
  that endpoint does not publish canonical NHL player IDs.
- The reviewed AHL roster bridge converts provider-scoped roster identities
  and finalized review decisions into `Rostered` AHL assignment facts. The NHL
  affiliate remains context, not an inferred legal-control claim.
- Every unresolved adapter output now supplies a provider-neutral
  `StagedPlayerAssertion`. Sealed replay retains both the identity proposal and
  the exact hockey fact waiting for review.
- `icelines-fetch` now owns retried raw-byte acquisition, URL fan-out,
  content-addressed capture storage, fragment assembly, validated package
  storage, and guarded activation. A shared league URL is fetched once while
  each organization/family manifest outcome remains explicit. Incomplete
  audits can be stored but cannot become active.
- `ProspectSourceCatalog` is the versioned, data-driven acquisition plan. The
  checked 2026-27 catalog expands 32 organizations into 96 cataloged logical
  objects backed by 34 unique URLs, while the other 96 requested objects stay
  explicit as missing. Provider display-name aliases such as `Utah Mammoth`
  and unaccented `Montreal Canadiens` live in catalog data rather than parser
  or team-specific branches.
- `icelines fetch prospect-sources` validates that expansion with `--dry-run`
  and performs the sealed all-organization audit in live mode. Raw evidence is
  content-addressed, shared league URLs are parsed once and fanned out to the
  32 logical outcomes, and `--store` plus `--out` keep the run replayable.
- A live 2026-27 audit on 2026-07-31 sealed package fingerprint
  `7c16dacd5b2ea66160e0f84bc4f1d1b5e9fdfbdbb54110bd91a3c4a512a0a47f`:
  192 manifest objects, 96 acquired, 96 explicitly failed as uncataloged,
  34 evidence inputs, 807 canonical NHL assignment facts, 246 unresolved
  draft/trade identity proposals with matching staged assertions, 12 explicit
  non-player trade-asset exclusions, 192 coverage rows, and 96 disclosures.
  All 32 NHL roster, draft, and transaction logical objects acquired; camp,
  contract, and AHL assignment remain visibly missing. The incomplete package
  was stored and correctly refused activation.
- Live drift tests fixed two provider realities without weakening validation:
  a forfeited NHL draft slot has no player `firstName`, and current NHL.com
  trade-tracker JSON-LD concatenates Markdown rows. Forfeited slots are now
  recorded without inventing a player; concatenated transaction rows parse
  through a fixture-backed layout normalizer.
- `ProspectPopulationScope` expands caller-supplied season-canonical
  organizations across draft, camp, contract, transaction, NHL assignment,
  and AHL assignment source families. Its fixture proves 32 teams produce 192
  visible outcomes without a team-name branch.
- The first S4 boundary is also exercised: reviewed identity decisions lower
  staged camp and contract rows into canonical facts, while missing/rejected
  identities remain disclosed or excluded. Multiple decisions for one
  proposal fail closed pending an explicit supersession policy.

The frozen club-publication fixture is a minimal capture of the named-section
layout published by the Vegas Golden Knights on 2026-06-28. The frozen
contract fixture captures the common signing layout from NHL.com's Logan
Cooley signing article dated 2025-10-29. Fixtures retain only the structural
and factual text needed to test the adapter.

## Authority boundaries now tested

- draft selection is not current rights;
- NHL roster presence is assignment, not a contract;
- camp attendance is not control;
- an explicit ELC camp attendee is distinguishable from a free-agent invite;
- a camp table's draft label still does not establish current rights;
- named publication rows cannot become canonical facts before identity review;
- a missing source object is not an empty population;
- terminal acquired-zero is distinguishable from acquisition failure;
- goalie rows are first-class in both roster and camp adapters; and
- layout/count drift fails closed.

## S3 completion boundary

S3 proves a real, sealed all-32 vertical slice with no silent source gaps. It
does not claim full population authority: camp, contract, and AHL assignment
sources are intentionally uncataloged and disclosed. S4 reconciles identity
and current state; S5-S6 use the resulting coverage funnel to close only the
provider capabilities justified by measured exclusions.

## Validation

```text
cargo test -p icelines-sources
cargo clippy -p icelines-sources --all-targets -- -D warnings
cargo test -p icelines-fetch --lib prospect_source_audit::tests::audit_seals_an_honest_incomplete_matrix_with_parsed_facts -- --exact
cargo run -q -p icelines-cli -- fetch prospect-sources --catalog design/data/prospect-source-catalog-2026-27.v1.json --dry-run
```

The checkpoint count and strict-lint result are refreshed after each adapter
slice; the commands above are the authoritative verification rather than a
hand-maintained test total.
