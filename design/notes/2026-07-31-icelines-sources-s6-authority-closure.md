# IceLines Sources S6 — Authority Closure Progress

**Date:** 2026-07-31
**Status:** In progress

## Measured starting gap

The real sealed 2026-27 census stops 807 canonical NHL-roster identities at
`unsupported_control` and 246 draft/trade candidates at
`unresolved_identity`. The current catalog has no contract/control source, so
the zero controlled count is correct. It is not repaired with inferred rights,
affiliate membership, or fabricated pipeline rows.

## First provider-level seam

- `ProspectSourceAuditArtifacts` accepts an optional raw
  `ahl_roster_stats.v1` snapshot acquired by IceLines' existing official AHL
  HockeyTech client.
- `run_prospect_source_audit_with_artifacts` stores the exact bytes in the
  content-addressed package store and parses them through
  `AhlRosterStatsV1Adapter`; provider IDs never become NHL player IDs.
- Manifest objects for `ahl_current_assignment` become acquired only when an
  official AHL team row explicitly names an in-scope NHL affiliate. An
  affiliated roster with zero published players remains an authoritative empty
  acquisition. Missing affiliations remain visible failures.
- The adapter contributes identity proposals and staged AHL roster facts.
  Without a separately reviewed identity decision they remain unresolved at
  replay, by design.
- Repeated finalized `ahl_identity_review_decisions.v1` batches can be admitted
  with one explicit review-registry evidence URL. Each batch is parsed through
  `AhlIdentityReviewV1Adapter`, its exact bytes are content-addressed, and the
  sorted canonical decisions produce the package's review-registry
  fingerprint.
- The canonical fact model distinguishes `affiliate_rostered` from an NHL
  assignment or control event. Once reviewed, this observation establishes an
  organization discovery and AHL club assignment, but the census correctly
  stops it at `unsupported_control` until rights or contract evidence exists.
- `icelines fetch prospect-sources --ahl-roster-snapshot PATH` exposes the seam
  without coupling core or `icelines-sources` to files, HTTP, or CLI types.

## Historical draft fan-in and canonical landing joins

- Catalog variants let multiple physical provider objects prove one logical
  organization/source-family cell. The 2026-27 catalog now requests every NHL
  draft ledger from 2018 through 2026. One failed member fails the cell; one
  successful year can never mask another failed year.
- Archived official draft payloads legitimately omit
  `broadcastStartTimeUTC`. The adapter now preserves provider timestamps when
  present and uses a year-bound representative time with `unknown` precision
  when absent. A live run exposed and verified this drift fix.
- `nhl_player_landing` is an independently enumerated family. With
  `--include-roster-player-landings`, the existing FLETCH cacheline acquires the
  exact official landing bytes for all canonical current-roster identities.
- A landing `draftDetails` row creates an identity decision only when year,
  round, overall pick, and drafting organization exactly and uniquely match an
  official staged draft row. No name similarity participates. The canonical
  sorted decisions bind the review-registry fingerprint.
- Explicit replay cutoffs remain fixed. Live runs close effective and knowledge
  cutoffs after acquisition, preventing newly captured FLETCH evidence from
  being misclassified as post-cutoff data.

The real live package now contains 224 logical source objects, with 128
acquired: all 32 historical draft cells, current NHL assignment cells, player
landing cells, and transaction cells. It created 308 deterministic draft
identity decisions and sealed as fingerprint
`966436b4dea1207e3cbef6c165dabffba053a35c3b917418656c6364c83cef12`.
The corresponding census has 2,477 discoveries, 807 canonical identities,
1,670 unresolved identities, and 807 `unsupported_control` losses. NYR has 81
discoveries/26 canonical; SEA has 67/22. The remaining 96 missing cells are the
camp, contract, and AHL families for all 32 organizations.

## Contract-control provider boundary

- `ContractControlLedgerV1Adapter` parses the provider-neutral
  `contract_control_ledger.v1` envelope into canonical `ContractSigned` facts.
- Each requested organization must appear exactly once with terminal coverage,
  including explicit zero-record teams. Declared counts must exactly reconcile
  with unique canonical player rows; otherwise the source fails atomically.
- Provider identity, capture time, coverage URL, row evidence URLs, contract
  kind, and effective time survive into the sealed package. The audit marks a
  contract manifest cell acquired only after the complete ledger validates.
- `--contract-control-ledger PATH` exposes this seam without embedding a
  licensed provider in core logic. A fixture proves that contract evidence
  advances a canonical candidate to `controlled_relationship`, while a reviewed
  AHL roster observation remains `unsupported_control`.
- No current all-32 provider ledger has been supplied in this workspace. The
  NHL landing endpoint does not publish contract terms, and the environment has
  no configured CapWages credential, so the live census remains honestly
  unclosed rather than fabricating control.

## Camp-participation provider boundary

- `CampParticipationLedgerV1Adapter` admits canonical development-camp,
  rookie-camp, training-camp, and prospect-tournament rows with explicit
  provider, capture, occurrence, and row evidence.
- The ledger requires exact terminal coverage for the audit scope, including
  explicit zero-row teams, and reconciles every team count atomically.
- `--camp-participation-ledger PATH` marks camp manifest cells acquired only
  after validation. The normalized result is attendance, never a legal-control
  or assignment fact—even when the source classifies the attendee as a
  controlled player.
- This complements the raw NHL article-layout adapters: providers may produce
  the canonical ledger after their own deterministic identity resolution, while
  raw name-only publications still use staged proposals plus explicit review.

## Generic identity-review boundary

- `IdentityReviewLedgerV1Adapter` replaces provider-specific review sprawl with
  one finalized `identity_review_ledger.v1` contract for every proposal family.
- Repeatable CLI inputs retain the registry provider, URL, reviewer, review
  time, rationale, exact bytes, hash, and row evidence in the sealed package.
- Decision and proposal IDs must be unique within a ledger. Package validation
  rejects unknown proposals and duplicate cross-ledger decisions.
- The integration fixture resolves a staged draft row to a canonical NHL ID but
  correctly leaves it at `unsupported_control`; identity review cannot create
  rights authority.

`identity_review_workboard.v1` is the read-only UI-neutral queue paired with
that write contract. It excludes already decided proposals and retains package
fingerprint/cutoffs, proposal ID, displayed identity, provider/evidence URLs,
and typed staged context for draft, participation, assignment, affiliate
roster, recall, loan, transfer, expiry, release, and compatibility rows. The
CLI renders the same core-built document as JSON or concise text; a lightweight
fetch example supports large-package validation without the monolithic CLI
link cost.

The real landing-enriched package produces 1,708 unresolved workboard rows:
1,685 historical draft proposals and 23 rights-transfer proposals. Draft rows
span 2018 through 2026 (167, 167, 155, 180, 178, 193, 208, 216, and 221 rows
respectively), and none carries a proposed canonical player ID. This measured
queue makes official exact-name search plus landing-coordinate corroboration
the next acquisition slice; it does not justify speculative league adapters.

That acquisition slice is now reusable in `icelines-fetch` rather than hidden
inside an affiliate-oriented CLI workflow. Draft workboard rows expose typed
coordinates, official search requests are deduplicated by normalized exact-name
query, and candidate player landings are acquired through the shared FLETCH
cache. `official_identity_candidate_board.v1` classifies every evaluated draft
proposal as exact, ambiguous, no exact name, landing missing, coordinate
mismatch, or provider failure. Each search and landing evidence object hashes
its own raw cacheline bytes. Only one exact organization/year/round/overall
match across a complete landing set is marked eligible; the artifact creates
no `IdentityReviewDecision` by itself. Frozen tests cover exact unique matches,
duplicate-coordinate ambiguity, mismatch, missing landings, and a near-name
result that remains ineligible.

The first all-league stress run also exposed an acquisition checkpoint issue:
one slow provider retry could hold a 200-query cohort open and prevent its
completed cachelines from reaching the shared manifest. Official identity
search now checkpoints in 25-query cohorts. Existing verified cachelines remain
reusable, and an interrupted run loses at most the current small cohort rather
than repeating a large block.
The identity search path also uses a bounded 10-second/three-attempt provider
policy instead of the generic 30-second/five-attempt cacheline default; failed
queries remain explicit provider failures and can be retried later without
pinning a league run for minutes per request.

The same run caught a cutoff-lifecycle defect: evidence acquired from a sealed
package's workboard is necessarily newer than that base package. Candidate
boards now retain both boundaries. Live acquisition derives an evidence cutoff
from the admitted FLETCH captures for use by a subsequent package seal;
`--evidence-cutoff` instead enforces deterministic replay. Neither path mutates
or backdates the base package.

Resumption also exposed a high-cardinality cache defect: the search helper
re-read the complete shared FLETCH manifest for every 25-query checkpoint,
including checkpoints whose requests were already cached. It now performs one
verified batch read, returns those cachelines immediately, and sends only
missing dataset IDs through bounded network checkpoints. A regression fixture
uses more than one checkpoint of cached requests and proves that none remains
pending for network acquisition.

Eligible candidate rows now have an explicit, provider-neutral promotion path.
`official-identity-review-ledger` requires reviewer identity, review time, and
an absolute registry URL, then emits `identity_review_ledger.v1` with the
original draft ledger, raw search, and landing evidence. The generic adapter
round-trips the generated
document in tests. Ambiguous, missing, mismatch, and provider-failure rows stay
on the candidate board and are never converted.

The first cache-only measured board at the August 1 evidence horizon evaluates
all 1,685 draft proposals: 686 are exact-coordinate eligible, 15 have no exact
name result, 642 have an exact-name candidate but still need a landing
cacheline, and 342 search cachelines remain unavailable. The 1,345 observed
candidate appearances are 1,345 unique NHL IDs. NYR is 22/51 eligible with 17
landing gaps, 10 search gaps, and 2 no-exact-name rows; SEA is 22/45 with 19,
3, and 1 respectively. Eligibility by draft year is 80 (2018), 109 (2019),
104 (2020), 112 (2021), 105 (2022), 99 (2023), 55 (2024), 20 (2025), and 2
(2026), which makes the expected recency gap visible rather than overstating
newly drafted-player coverage. Cole Beaudoin resolves exactly to NHL ID
8484786; Alberts Smits and Liam Greentree remain search-cache gaps in this
partial pass. Gabe Perreault is absent from this unresolved queue because the
base package already decided him through its roster/landing path.

That partial measurement is now superseded by the completed acquisition and
offline replay. The final board evaluates the same 1,685 draft proposals: 1,661
are exact-coordinate eligible and 24 have no exact official name result. There
are zero landing-missing, coordinate-mismatch, ambiguous, or provider-failure
rows. NYR is 49/51 exact with two no-exact rows; SEA is 43/45 exact with two
no-exact rows. Exact eligibility by draft year is 165 (2018), 165 (2019), 155
(2020), 180 (2021), 176 (2022), 192 (2023), 204 (2024), 207 (2025), and 217
(2026). Alberts Smits resolves exactly to NHL ID 8485957, Liam Greentree to
8484802, and Cole Beaudoin to 8484786. The live and offline artifacts are
byte-identical at 3,317,516 bytes with SHA-256
`cd5cac25d7ed90d316f03dd15db7084b9c61a6c99d36ca6d28b7fd05b5156af0`,
proving cache-only reconstruction at the final evidence horizon.

An August 1 follow-up removed an accidental exact-display-name dependency from
draft discovery without relaxing promotion. Exact normalized names are still
preferred. An empty full-name result now admits same-surname discoveries, and
only those empty rows receive a second surname query. Every candidate must
still have a verified official landing, the landing identity must agree with
the search identity, and exactly one landing must match draft year, round,
overall pick, and organization. Surname agreement by itself never establishes
identity. Offline runs use a fallback cacheline only when it is already sealed;
an absent fallback never turns the prior result into a provider failure.

The completed live replay raises exact-coordinate eligibility from 1,661 to
1,683 of 1,685 proposals. It resolves 22 alternate-name cases, including Artem
Gonchar (NYR, NHL ID 8485573) and Hawke Huff (SEA, NHL ID 8486280), and reaches
51/51 NYR and 45/45 SEA draft rows. The two remaining rows stay explicit:
Cameron Lund is a coordinate mismatch because the provider's surname result
set contains only unrelated players, and Nikita Susuyev has no provider result.
The evidence board is 3,372,684 bytes with SHA-256
`9ec610dbee9da55b8b2784ac2440169e5b46ed2aaa6859a48d3d96d94e353453`.
It remains a candidate board, not a finalized review ledger.

A final cross-proposal audit compared the 23 trade proposals with both the
package's 308 prior identity decisions and the 1,683 exact-coordinate candidate
rows. Ten trade names have one unique identity already proven elsewhere (Sean
Farrell, Kalle Vaisanen, Devon Levi, Shakir Mukhamadullin, Zachary Sharp, Sasha
Pastujov, Mavrik Bourque, Cole Beaudoin, Sean Durzi, and Dennis Hildeby). They
remain proposals because copying an identity onto a different source row is a
review decision under `identity_review_ledger.v1`; IceLines must not silently
convert same-name evidence into approval. The other 13 require independent
identity evidence or review.

The August 1 closure audit found no finalized identity-review,
contract-control, or camp-participation ledger in the repository or target
artifacts, and `CAPWAGES_API_KEY` is not present. Current camp publications are
also a future-season input rather than a complete all-32 authority at this
cutoff. Consequently the remaining S6 work requires an authorized reviewer,
licensed/provider contract data, and later all-32 camp publications. This is an
external authority boundary, not an implementation or test failure.

## Validation

```text
cargo test -p icelines-fetch --lib prospect_source_audit::tests::audit_seals_an_honest_incomplete_matrix_with_parsed_facts -- --test-threads=1
cargo clippy -p icelines-fetch --all-targets -- -D warnings
cargo check -p icelines-fetch -p icelines-cli
cargo test -p icelines-fetch --lib source_catalog::tests -- --test-threads=1
cargo test -p icelines-fetch --lib source_acquisition::tests -- --test-threads=1
cargo test -p icelines-sources
```

The full source-crate run exposed and fixed a stale AHL lowering path: reviewed
affiliate roster rows now preserve `AffiliateRostered { affiliate, at }`
instead of collapsing to generic club presence. This keeps the NHL affiliate
available for organization discovery while the rights resolver continues to
refuse it as control.

The targeted CLI parser test was attempted, but the oversized CLI test binary
did not finish building within the 244-second command limit and emitted no test
failure. Runtime/library compilation is clean; this timeout is retained as an
explicit validation limitation.

## Next measured closure

### August 5 AHL authority replay

The local official 2026-27 `ahl_roster_stats.v1` snapshot captured at
`2026-07-29T03:51:30Z` contains exactly 32 AHL teams, 32 unique NHL affiliate
codes, no missing affiliate, and zero published preseason roster players. Its
SHA-256 is
`b0c70318981fa2582607d34720c09ebe0b1207620e1ff0f101bc0c767cb18978`.
The source audit admitted each explicit empty affiliate row through the existing
adapter and raised the sealed manifest from 128 to 160 acquired objects. The
new package fingerprint is
`dcc7a347b6b17fe4723545cdc7886031b096b1c131fa3371e85d318d3183a142`.

The replayed census retains 2,477 discoveries, 807 canonical identities, and
zero controlled relationships. Its compact readiness fingerprint is
`591be971256cd1a7b11fa6f449045f07288166e36824d507078a1f931b09b5cc`.
All 32 `ahl_current_assignment` source gaps are closed; this proves acquisition
coverage only and does not assign a player or establish control. NYR and SEA
now each expose two remaining gaps. The closure board falls from 96 to 64 cells
with fingerprint
`00930ad770c7783545453f906a58c637da016dee48516575cd2a47356a3fa021`:
32 club camp and 32 contract publication cells.

1. Connect an authorized current all-32 provider to
   `contract_control_ledger.v1`; do not treat draft, affiliate, roster, or
   current-team observations as legal control.
2. Acquire or review all-32 camp ledgers through the new boundary, then resolve
   canonical camp identities without converting attendance into control.
3. Review the 1,683 exact-coordinate candidate rows, retain the two explicit
   exceptions, and convert only approved rows into an evidence-bound review
   ledger before applying transaction and expiry facts.
4. Re-run the census before choosing CHL, NCAA, European, or goalie-specific
   adapters; promote only the adapters named by remaining canonical losses.
