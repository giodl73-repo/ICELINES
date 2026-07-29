# The Window — Organization Health Implementation Plan

**Date:** 2026-07-27 (consolidated 2026-07-28)
**Status:** Active — foundation, evaluation surfaces, historical origins,
within-season movement/personnel attribution, and scenario sensitivity
implemented; production coverage and a future untouched holdout remain open
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
9. Every composite answers one named decision. Descriptive organization health,
   forecast competitive success, and competitive-window timing may share
   profiles, but they do not share an unlabeled master score.

## Implementation baseline (2026-07-27)

This plan is incremental. The following baseline is already present and must be
preserved while the remaining work is completed:

| Workstream | State | Proven capability | Remaining gate |
|---|---|---|---|
| W0 | complete | 37-profile machine-readable inventory: 17 ready, 13 evaluation, 4 context-only, 3 blocked | Reclassify only through the promotion protocol below. |
| W1-W2 | complete | Versioned observations/manifests/boards, deterministic fingerprints, validation, normalization, aggregation, confidence/coverage, rank gates | Cross-platform fingerprint matrix in W9. |
| W3-W4 | evaluation-complete | `balanced.v1`, 17 typed source adapters, a sealed portable source-package contract, one-pass all-32 cache lineup assembly, core-derived fatigue and frozen-strength profiles from sealed forecasts, official NHL TOI-backed special-teams depth, core-composed NHL/AHL organization lineups from reviewed affiliate projections, all-32 partial evaluation board, classifications, focused cards, and a fail-closed production-rank gate. The July 29 real package completes 14/16 required profiles. | Complete reviewed all-league AHL identity/assignment facts for organization and recall depth; then pass `--require-ranked`. |
| W5 | complete | Comparable movement/history contracts, refusal tests, immutable bridge/rebase, a real three-checkpoint 2024-25 IceCast history, both earlier-scenario and later-counterfactual attribution bases, and a real Jan. 31 -> Feb. 28 paired rolling-replay personnel artifact with 219 dated events | Preserve raw profile effects when percentile normalization yields zero aggregate movement; keep the paired estimate explicitly non-causal and uncalibrated. |
| W6 | complete | Sealed comparison and typed authorities; real 32-team 2026-27 multi-source baseline/scenario boards; paired isolated NYR event effects; combined NYR/SEA 1,000-trial distribution; direct/cohort/unchanged attribution; fail-closed and partial-pane regression fixtures | Recalibrate scenario assumptions as stronger future evidence arrives without rewriting the sealed artifacts. |
| W7 | evaluation-complete | Leakage gate, per-origin frozen baselines, rolling origins, pane ablations, organization stability, between-origin uncertainty, sealed claim status, frozen training/validation/retrospective-holdout roles, and four real point-in-time observed-history origins | Trial-noise propagation and future untouched-holdout evidence; the current retrospective holdout is explicitly inconclusive. |
| W8 | complete | CLI, two-page TUI all-32 board/focused cards, Web/API, JSON, UI-neutral card, durable Markdown report, desktop/tablet/mobile live review, semantic checks, keyboard/reduced-motion/narrow review, and exact board/card plus semantic renderer golden parity across CLI/TUI/Web | Keep the shared golden test mandatory as surfaces evolve. |
| W9 | complete | Authoring/compatibility/cache documentation, compatibility/extension fixtures, canonical replay and three-OS fingerprint gates, strict lint, schema/golden checks, offline smoke, dependency audit, performance/browser/accessibility evidence, and verified Linux x86_64, Windows x86_64, macOS ARM64, and macOS x86_64 packages; PR #23 run 30351304523 passed 26/26 checks | Keep these gates mandatory; production source completeness and a genuinely future untouched holdout remain W3-W4/W7 product gates rather than release-evidence claims. |

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
| Deprecate, supersede, demote, or retire a Line | Registry lifecycle amendment, replacement/hold rationale, affected-Frame list | New registry revision; sealed observations and boards remain immutable; official Frames migrate only through a new manifest | HART, KEEL, TAPE, PACE, BENCH, EDGE; saved-board replay, replacement/hold behavior, comparison audit |

Profile promotion is one-way only when evidence supports it:

```text
blocked -> context-only -> evaluation -> ready
```

Each promotion records source authority, cohort coverage, point-in-time safety,
method version, calibration claim, limitations, and verification evidence.
Demotion is always allowed when authority or freshness regresses and must not
silently retain the previous rank eligibility.

Readiness and lifecycle are orthogonal. Readiness says what evidence supports a
method now; lifecycle says whether new manifests should select that method.
The lifecycle states are `active`, `deprecated`, and `retired`. A deprecated
method remains readable and may remain in a pinned historical/custom Frame, but
new official Frames require its declared replacement or an explicit hold. A
retired method remains replayable only for sealed artifacts and cannot be added
to a newly authored Frame. Supersession is an explicit edge between immutable
method versions, never an alias or in-place rewrite.

The lifecycle fields are a planned registry amendment. They must not be added
to the current `organization_window_registry.v1` wire contract until the schema,
reader compatibility, registry validator, author guide, and replay fixtures land
together.

Every pull request that alters Window semantics includes a change note naming:

1. the lane above;
2. affected profile, manifest, schema, and source fingerprints;
3. compatibility behavior for saved artifacts;
4. newly valid and invalid comparisons; and
5. VTRACE evidence added or intentionally still open.

## Consolidated delivery tracks

The remaining work advances through three independent promotion tracks. A
track may improve without implying that either of the others is complete.

| Track | Current state | Promotion target | Hard gate |
|---|---|---|---|
| **Source completeness** | 14/16 required `balanced.v1` profiles complete; 0/32 teams rank eligible | Reproducible all-league package with every required profile value | `window-source-audit` reports 16/16 complete and `window-build --require-ranked` succeeds without proxies |
| **Predictive evidence** | Four frozen historical origins; retrospective holdout inconclusive | Claims calibrated by target and horizon | A genuinely later untouched holdout, leakage pass, baseline comparison, and claim-specific acceptance rule |
| **Extension maturity** | Typed registry, manifests, bridges, scenarios, shared renderers, and authoring contract implemented | Safe addition or alteration of profiles, panes, Frames, sources, and schemas | Compatibility fixtures, method/version decision, role review, VTRACE mapping, and surface parity for each promoted extension |

These tracks define the product labels:

- **evaluation**: a valid board may be partial and rank-withheld;
- **production-ranked**: source completeness passed for the selected Frame;
- **calibrated**: a named prediction claim passed its separate untouched-holdout
  gate; and
- **custom**: a valid user Frame, always labeled by manifest fingerprint and
  never conflated with an official Frame.

They also keep three composite claims distinct:

| Composite | Decision answered | Required evidence | Output language |
|---|---|---|---|
| **Organization health** | How strong, deep, sustainable, and resilient is the organization at this checkpoint? | Complete selected Frame, source/coverage gates, explainable pane contributions | Score, pane profile, rank when eligible |
| **Competitive success forecast** | How likely is a named result over a named horizon? | A target-specific predictive model, probability calibration, and untouched holdout | Probability/distribution with calibration status |
| **Window timing** | Is the organization contending, rising, plateauing, retooling, or rebuilding, and for which horizon? | Current health plus separately fingerprinted horizon Frames and classification method | Classification with drivers, horizon, and uncertainty |

No renderer or marketing copy may call a health percentile a Cup probability,
or infer window timing from one overall score without the named classification
method.

## Current execution tranche — source completion

Execute this tranche in dependency order. Each stage seals a useful artifact
and remains independently reversible.

### S1 — Cache-native prospect composition — implemented 2026-07-28

Turn the already proven camp -> career context -> career discovery -> prospect
program chain into one fetch-owned source assembly option for
`window-source-package`.

- Select camp candidates marked `prospect` **or** `rookie_eligible`, then apply
  exact dated age and NHL-workload gates.
- Read the configured career-history cache, with an explicit path override for
  reproducible/offline runs; do not depend on repository-relative data files.
- Reuse the existing typed builders for context, discovery, goalie studies, and
  the 32-team prospect board. The CLI orchestrates but does not score.
- Conflict with an explicitly supplied prospect-program artifact rather than
  silently choosing one authority.
- Seal intermediate/source fingerprints and disclose excluded identities.

**Acceptance:** the automatic path produces 32 programs and the same three
complete prospect profiles as the explicit artifact path, including FLA's
rookie-only candidate; focused, full fetch, parser, order-invariance, offline,
and package-audit tests pass.

Implementation evidence: the real July 28 automatic and explicit paths each
produce 32 programs from 162 studies (94 ranked, 68 graduated). All 96
organization/profile comparisons across prospect pool, development, and
readiness have identical raw values, normalized scores, ranks, and sample
sizes. The cache-native full package retains the honest 13/16 required-profile
audit and 0/32 rank eligibility. Artifact fingerprints differ intentionally
when the explicit path carries additional repository-overlay citations.

### S2 — Reviewed NHL/AHL organization composition

Create and review season-aware NHL/AHL affiliation, player-identity, and
assignment facts, then build all 32 affiliate projections through the existing
organization-lineup primitive.

- Keep provider-local IDs outside canonical core identity until reviewed.
- Represent shared, changed, missing, and historical affiliations explicitly.
- Enforce AHL veteran-development rules in AHL roster/line construction without
  treating those rules as NHL prospect quality.
- Preserve source capture time and the Window cutoff on every join.
- Do not reuse current affiliations for historical replay without dated
  authority.

**Acceptance:** `development.organization_depth` and
`development.recall_depth` have eligible values for all season-canonical teams;
ambiguous identities remain typed review failures; affiliation and assignment
fixtures cover relocation, shared affiliates, trades, loans, and missing clubs.

**Identity checkpoint — 2026-07-28:** official 2025-26 AHL roster evidence
contains 1,425 team appearances across 32 clubs. The exact, sourced-alias, and
ordinary conflict lanes reviewed 1,410 appearances; the league rejection lane
closed the remaining 15 mapping appearances without deleting their AHL player
facts. The resulting envelope has 0 pending rows, 100.00% resolved coverage,
and 98.95% canonical NHL-identity coverage. Identity review is therefore
complete for this snapshot. Current-season affiliation, assignment, projected
role/score, readiness, waiver, and professional-game facts remain a separate
authority gate; a camp cut is not treated as an AHL assignment.

The rollover contract now also separates optional `prior_ahl_team` from the
target `ahl_team`. This removes the same-club assumption for affiliation
changes such as NYI/Hamilton while retaining backward compatibility and an
explicit disclosure of both authorities.

**Rollover checkpoint — 2026-07-28:** the forecast-native league adapter and
separate 2025-26/2026-27 affiliation catalogs build 32/32 team rollover plans
with no schema, cohort, affiliation, or forecast failures. Zero teams are yet
projection-ready: 1,174 prior-only appearances require sourced organization-
status review, 15 appearances retain mapping rejection, and 144 camp
candidates remain waiver-gated. Aggregate position shortages are 357F/171D/59G.
These are now typed team/player work queues. They are not filled by assuming a
camp cut or prior roster appearance proves a 2026-27 assignment.

The league organization-status draft now materializes all 1,425 appearances as
32 fingerprint-bound child reviews with 1,174 required decisions, 15 identity
blockers, and zero failed teams. Forecast-native review and application are
artifact-identical to the explicit camp-input path. Finalized application is
atomic across the league; mapping rejection stays a readiness blocker without
discarding valid status decisions for other players.

**Professional-game checkpoint — 2026-07-28:** a new typed policy/ledger pair
and `fetch career --league-crosswalk` adapter turn the reviewed identity cohort
into reproducible career evidence. The real run fetched 1,323/1,323 unique
canonical histories with zero skips. An intentionally partial policy completes
585 totals and preserves 738 players as league-treatment work, led by 539 ECHL
and 87 SHL appearances. The adapter reports only the 260-game threshold test;
age qualification, European youth-season exemptions, and assignment authority
remain independent gates.

The provisional policy exercises those independent gates without promoting
them: all 1,323 totals resolve, 1,010 fall within the raw threshold, 313 exceed
it, and 8,780 games across 181 players are retained as European youth-exempt.
Because the policy authority is not `final`, all 1,323 final qualification
fields remain withheld pending the published 2026-27 rule book.

Core no longer forces all consumers to repeat the old count-only
classification bug. Affiliate inputs and views preserve a separate final
`development_rule_qualified` fact, which overrides threshold fallback when a
reviewed policy supplies it. The league facts workboard requires that final
field before a skater can become facts-ready for preseason projection.

The official-snapshot facts bridge is implemented: a final ledger can now be
applied to one team's provider-keyed projection facts without altering score,
position, prospect, recall, assignment, or waiver authority, and the existing
affiliate-input builder consumes the fingerprint-bound envelope. Preseason
camp-only players use the NHL-ID-keyed league workboard described below.

**League facts-workboard checkpoint — 2026-07-28:** that composer is now
implemented as `ahl_preseason_league_facts_workboard.v1`. The camp forecast
seal preserves primary plus multi-position eligibility end to end and rejects
older position-incomplete forecasts. The real rebuild retains exact positions
for 933/933 camp players, including 26 multi-position players, and composes
32/32 affiliate workboards from the reviewed rollover and provisional ledger.

The board identifies 1,371 viable candidates and zero facts-ready candidates.
All 1,371 still require explicit assignment, prospect-status, and recall-
readiness authority; 1,174 need organization-status and projected-score facts;
144 need waiver clearance; the initial run had 255 prior-only exact-position
gaps; 52 lack professional-game totals; and 1,202 skaters await final rule
qualification.
Those are independent, queryable queues, not inferred cuts or assignments. The
next S2 slice is a sourced overlay/application contract that reduces these
blockers and then feeds the existing affiliate and organization-lineup
primitives.

**Facts application checkpoint — 2026-07-28:** the sourced overlay/application
contract is implemented. Draft generation exactly covers canonical candidates;
final application requires reviewer, timestamp, per-row evidence and notes,
and the exact workboard fingerprint. Optional facts clear only their owned
blocker, sealed-value conflicts fail, and `not_assigned` is a distinct outcome.
A real all-32 command-path smoke changed one synthetic validation row from
candidate to not-assigned, reduced the league candidate/assignment/waiver
queues by exactly one, preserved all other blockers, and produced distinct
source, overlay, and result fingerprints. The smoke is not roster evidence.

The remaining S2 acquisition work is now explicit: finalize organization-
status review, publish/finalize the season rule policy, and author sourced
assignment/prospect/recall/waiver/score/position facts. The next code bridge
will lower a facts-ready application into the existing per-team
`AhlAffiliateProjectionInput` contract and refuse incomplete team pools.

**Projection-input lowering checkpoint — 2026-07-28:** that bridge is now
implemented as `ahl_preseason_league_projection_inputs.v1`. It revalidates the
facts application, requires final matching rule authority, retains rollover
and review provenance, and calls the existing affiliate optimizer before a team
is emitted. A complete 12F/6D/2G fixture builds; removing one goalie produces a
named team failure; the real provisional all-32 artifact is refused at the
authority boundary. No provisional team input was published.

S2 now has all composition primitives needed for real output. Its remaining
work is source acquisition/review, followed by building affiliate projections
and the existing organization-lineup documents from the resulting complete
inputs.

**Development-rule authority audit — 2026-07-28:** the official AHL FAQ and
Scott Howson's 2026 CBA announcement confirm that 2026-27 retains twelve of
eighteen skaters at 260 or fewer NHL/AHL/European-Elite regular-season games
and removes the former 261-320 slot restriction. The public AHL rules page
still publishes only the 2025-26 book. That book contains the under-25 and
European-youth exceptions, but no target-season source yet confirms those two
inherited notes.

The professional-game policy, ledger, and facts-application contracts are now
explicit v2 documents. Base-rule, age-clause, and youth-exemption authorities
carry independent effective seasons; final build/application requires all
three to equal the target season. The real 2025-26-backed policy therefore
remains provisional by construction. This is a calendar-gated source gap, not
permission to promote 1,202 pending skater classifications.

**Official-position cache checkpoint — 2026-07-28:** the same NHL landing
payload already used for career totals now persists its official primary
position beside birth date. All 1,323 canonical histories refreshed with zero
skips and 1,323 official positions. The ledger/workboard fallback filled only
generic prior-AHL positions, never overwrote camp eligibility, and reduced the
exact-position queue from 255 to zero across all 32 teams. The remaining 1,174
projected-score gaps require a separately versioned value model or authored
projection authority; position metadata is not used as a scoring proxy.

**AHL player-value checkpoint — 2026-07-28:** the separately versioned model
is now implemented as `ahl_player_value_policy.v1` with method
`ahl_prior_performance_bayesian_rate.v1`. Core owns deterministic skater and
goalie estimation; fetch owns official-snapshot aggregation, reviewed identity
joining, stable source fingerprints, and narrow workboard application. The
method uses position-specific points/game priors for skaters and a shot-based
save-percentage prior for goalies. It is explicitly evaluation-only, not an
NHL equivalency or calibrated forecast.

The real 2025-26 all-league run scored 1,221 canonical players: 689 forwards,
398 defensemen, and 134 goalies. Applying the ledger filled 1,076 of 1,174
missing preseason scores, reducing that blocker queue by 91.7% to 98. Of the
remaining rows, 97 have no prior AHL statistical observation and one has a
position-group conflict; all remain blocked. The application cleared no other
fact class and retained a new sealed result-workboard fingerprint. S2 remains
open on those 98 scores plus sourced status, assignment, prospect, recall,
waiver, and final-rule authority.

**Operational prospect-status checkpoint — 2026-07-28:** a separate core
policy now defines the reserve-system population by exact cutoff age and
observed NHL regular-season workload. It is explicitly not NHL rookie,
contract, waiver, assignment, or scouting status. Either graduation axis can
establish `false`; `true` requires both facts. Fetch composes this player-global
classification from the official career cache and applies only the
`prospect_status` blocker to the chained league workboard.

The real run classified all 1,282 canonical candidates: 594 eligible prospects
and 688 graduates (466 age-only, 13 workload-only, and 209 on both axes). It
applied those classifications to all 1,371 organization appearances and
reduced the prospect-status queue from 1,371 to zero. Eighty-one canonical
players appear in more than one organization because rollover assignment is
still unresolved; their status is reused without choosing an organization.
S2 remains open on organization status, assignment, 98 insufficient-evidence
recall-readiness rows, waiver clearance, 98 score gaps, and final-rule
authority.

**Recall-readiness checkpoint — 2026-07-28:** core now owns
`ahl_recall_readiness_policy.v1` and method
`weighted_value_experience_camp.v1`. The evaluation index weights
within-position value, observed NHL regular-season workload, and camp proximity
at 0.50/0.30/0.20, requires 0.70 evidence coverage, and reports confidence
separately. It is not a calibrated recall or NHL-success probability.

The fetch ledger composes the chained workboard, official career cache, and
sealed all-32 camp forecast. Prior-AHL value takes precedence over camp value;
camp proximity is omitted whenever camp already supplied the value signal, so
one modeled source is not double-counted. The real run estimated 1,185 of 1,282
canonical candidates and applied readiness to 1,273 of 1,371 appearances. The
remaining 97 canonical players cover 98 appearances and stay blocked for
insufficient value coverage. No assignment, waiver, organization-status,
prospect, score, game, or final-rule fact was changed.

This slice also fixed a pre-existing unit bug in the Window adapter: missing
0..1 readiness can no longer fall back to a 0..100 projected player score.
Portable ledger sealing now canonicalizes through the supported JSON wire
representation; the real saved-ledger replay exposed and closed the prior
in-memory-only float fingerprint mismatch.

**Cross-league value checkpoint — 2026-07-28:** core now owns
`ahl_cross_league_value_policy.v1` and method
`career_paired_ahl_translation.v1`. This is not a universal NHLe table. It
learns frozen same-season or next-season source-to-AHL career pairs by position
group: workload-weighted multiplicative points-per-game translations for
skaters and additive save-percentage deltas for goalies. Pair count, unique
players, aggregate and per-pair workload, recency, source sample, and RMSE fit
all gate or discount the estimate. The sealed evaluation ledger binds the
exact workboard, career source, policy, calibration diagnostics, and player
evidence; its application fills only a missing score before prospect and
recall-readiness evaluation.

The real all-32 run supported 14 league/position calibrations and estimated 78
of 97 unique missing-value candidates, applying 79 of 98 organization
appearances. Rebuilding the dependent readiness chain increased coverage from
1,185 to 1,263 of 1,282 canonical candidates and from 1,273 to 1,352 of 1,371
appearances. Score and readiness queues fell from 98 appearances to 19. Those
19 remain explicit insufficient-evidence rows; no organization, assignment,
waiver, game-count, or development-rule authority was cleared.

**Organization-status authority checkpoint — 2026-07-28:** the official NHL
landing cache now preserves dated current-team observations per player.
`ahl_organization_status_ledger.v1` compares only positive current-team facts
with the sealed 32-team review cohort: equality resolves retained, another
canonical team resolves departed, and missing/stale/non-cohort facts refuse.
Its fingerprint-bound application prefills the review but never finalizes it.

The all-32 run read 1,282/1,282 verified candidate landings acquired July
25-26 with zero skips and resolved 549 of 1,174 organization decisions (425
retained, 124 departed).
All 625 unresolved rows have an official landing document but no current NHL
team. They remain manual contract/league-status research; no camp-absence or
unsigned-player inference was made. S2 remains open on those 625 decisions,
assignment, waiver clearance, 19 score/readiness gaps, and final-rule
authority.

**Official AHL transaction checkpoint — 2026-07-28:** source acquisition now
captures the complete paginated league `ADD`/`DEL` stream, provider team
catalog, exact page totals, feed URLs, and verified per-page acquisition times
as `ahl_transaction_snapshot.v1`. The first sequential implementation exposed
an expensive repeated-manifest path; acquisition now batches all pages with one
bounded cache reconciliation, matching the established roster-fetch pattern.

The completed 2025-26 replay seals 4,011 events across 21 pages and 32 teams:
2,259 additions and 1,752 deletions. The official 2026-27 season catalog is
already present with 32 teams but currently returns zero transaction rows.
Therefore no target-season assignment blocker is cleared yet. A separate
cutoff-aware state ledger now interprets explicit event sequences; old-season
events and target-season absence still cannot be promoted into current
assignments.

**AHL transaction-state checkpoint — 2026-07-28:**
`ahl_transaction_state_ledger.v1` evaluates only each player's latest event
date through an explicit cutoff. Single-destination ADD sets resolve assigned;
DEL-only sets resolve removed; unknown events, multiple ADD destinations, and
same-team ADD/DEL sets remain ambiguous because the feed has no trusted
intraday order. Reviewed identity and target affiliation are separate bound
inputs, and the ledger validates its counts and all four fingerprints against
tampering or rebinding.

The 2025-26 historical replay resolves 1,161 player states from 4,011 events:
695 assigned, 403 removed, and 63 ambiguous. Reviewed identity covers 1,149;
12 remain provider-only. The current 2026-27 feed contains zero events, so the
target ledger correctly contains zero states and clears no blockers. The new
fingerprinted application writes only unambiguous canonical state, preserves
row-level method/cutoff/source provenance, and refuses existing conflicts. Its
real target run applied zero true and zero false facts and retained all 1,371
assignment blockers. S2 now needs actual target-season source events; waiver
clearance, 19 score/readiness gaps, 625 organization decisions, and final rule authority
remain separate work.

**AHL waiver-clearance checkpoint — 2026-07-28:** the completed ESPN envelope
was audited before introducing authority. It contains 109 waiver placements
and 10 claims but no explicit clearance rows; the AHL transaction feed contains
no waiver descriptions. Therefore source silence cannot clear a player.

`ahl_waiver_clearance_review.v1` now provides a fingerprint-bound exact queue,
sourced partial finalization, and narrow application. Only dated target-season
cleared/claimed results with absolute evidence URLs and reviewer authority can
write. Cleared removes only the waiver blocker; claimed records false and stays
blocked without changing organization or assignment. The real July queue has
144 required decisions, zero resolved, and 144 pending (PHI and UTA lead with
nine each), fingerprint
`sha256:0f87409017ee96e884cd4ef908c7159222fc0f9ac1b2fe2f6280bbcfd73b6eef`.
This is the correct pre-camp state. S2 remains open on actual camp-time waiver
results, 625 organization decisions, 19 score/readiness gaps, and final-rule
authority.

### S3 — Goalie dependency authority decision

The July 29 live roster replay exposed two different gaps: UTA lists a second
goalie with no 2025-26 NHL sample, while BOS lists only Jeremy Swayman. Those
are not the same missing-data problem and must not share a shortcut.

The value half is now implemented. `career_paired_ahl_to_nhl_goalie.v1` fits a
shot-weighted additive save-percentage delta from same/next-season AHL/NHL
career pairs, measures RMSE and sample confidence, discounts candidate
workload by calibration confidence, shrinks to an explicit NHL prior, and only
fills a missing NHL goalie-quality score. A sealed
`nhl_goalie_translation_ledger.v1` retains the cohort, policy, candidate
estimate, unavailable rows, disclosures, and fingerprint. The lineup marks the
score `estimated` and carries the method in a warning; observed NHL values win.

The real all-32 replay estimates Jaxson Stauber at 46.3 on the NHL lineup scale
from 13 confidence-adjusted games of evidence. UTA's goalie dependency is
complete, and `nhl.goalie_quality` is 32/32.

The dated camp composition is now implemented as a second independent
authority. A confirmed-pool modal camp branch may fill only an empty goalie
slot; it cannot replace an existing assigned goalie. The selected goalie still
requires a separate paired-career value. Boston therefore adds Michael
DiPietro as a scenario backup at 65.4 from 16 confidence-adjusted games while
Swayman's confirmed score remains untouched. The full-package audit now has
`resilience.goalie_dependency` at 32/32 and 14/16 required profiles complete.

`window-source-refresh-lineups` validates an existing package fingerprint,
refreshes all cache lineups, optionally replaces the camp authority, composes
goalie assignments, clears only the derived package fingerprint, and reseals
the complete document. Legacy package replay also exposed and fixed an
additive-field fingerprint bug: absent optional/empty camp and affiliate fields
remain omitted during serialization, preserving old v1 package identity.

**Acceptance:** met. The paired value and dated assignment methods remain
separate, both goalie profiles are 32/32, the old full package validates before
refresh, and the refreshed package reseals and audits at 14/16 required
profiles without changing existing goalie assignments.

### S4 — Production-rank gate and evidence package

Rebuild from a clean configured cache with no network, audit the package, run
`--require-ranked`, and publish the exact source/board fingerprints with the
release evidence.

**Acceptance:** 16/16 required profiles complete, 32/32 teams rank eligible,
all shared parity/canonical replay/package checks remain green, and no blocked
source claim was promoted by proxy.

## Next predictive tranche — future holdout

This tranche is calendar-gated, not coding-gated. Before outcomes become known,
freeze the next origin, manifest, sources, targets, baselines, exclusions, and
acceptance thresholds. After the target period closes, score it once and retain
the result whether favorable or not. Trial-level uncertainty propagation may
land earlier, but it cannot substitute for the untouched holdout.

## Extension backlog lanes

New capability should enter the smallest applicable lane instead of expanding
`balanced.v1` by default:

1. **New Lines:** cap, injury concentration, shift chemistry, management
   behavior, player-development variants, and prospect conversion.
2. **New Frames:** `win_now`, `sustainable`, and `rebuild`, each with one primary
   horizon and independently reviewed family caps.
3. **New authorities:** verified cap, supported shifts, injury/availability,
   and dated qualitative research promotion methods.
4. **New consumers:** fantasy draft/morning/trade views and simulation cards
   consume sealed Window profiles or boards; they do not fork scoring logic.
5. **New seasons:** the season catalog, affiliations, cutoffs, schedule, and
   holdout roles are data/configuration inputs rather than hardcoded 2026-27
   branches.

An extension is not added to an official Frame merely because it can be
computed. It must improve a named decision, clear source and leakage review,
fit within a signal-family budget, and survive an ablation or sensitivity
test.

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

After the current production-source tranche, add the registry-lifecycle
amendment described above as one atomic compatibility slice. Do not overload
`WindowProfileReadiness`: lifecycle, evidence readiness, and observation status
are three different axes.

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

The source handoff is now `organization_window_source_package.v1`: one sealed,
canonically ordered document containing the exact typed authorities consumed by
the 17 adapters. Loose CLI paths are normalized into this package before core
builds a board. `organization_window_source_coverage.v1` reports exact profile
and organization gaps rather than treating cohort presence as evidence
coverage. A production build uses `--require-ranked`, which refuses to write
while any organization remains withheld. The remaining W3 work is source
acquisition/assembly for complete all-32 lineup, organization, prospect,
conversion, camp, and schedule coverage; it is not another scoring contract.

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

The checked Jan. 31, Feb. 28, and Mar. 31, 2025 IceCast history satisfies the
real-checkpoint portion through
`build_forecast_history_organization_window_boards`. Its manifest contains only
`nhl.expected_points`, so it does not imply that pipeline, development,
deployment, or resilience were observed at those cutoffs.

The attribution implementation requires the earlier board, later board,
canonical movement, dated event set, scenario board, and typed scenario
authorities. It replays both observed movement and scenario impact before
populating `personnel_delta`, then assigns the unexplained remainder to
`residual_revaluation`. IceReplay event adapters classify the interval's
trades, waivers, recalls, assignments, injuries, activations, signings, and
releases without turning event presence into a numeric estimate. The remaining
paired-replay path keeps the game checkpoint fixed while omitting only personnel
evidence after the earlier cutoff.

The sealed Jan. 31 -> Feb. 28, 2025 run closes W5 with 219 dated events and 11
nonzero raw `nhl.expected_points` effects. No effect crossed an empirical
percentile boundary, so aggregate personnel score movement was zero. The
summary contract preserves those raw effects and discloses that the seeded
paired result is an estimate, not a causal or calibrated personnel claim.

### W6 — Scenario sensitivity

Connect existing IceCast, player development, trade, injury, training-camp,
and line-combination scenario artifacts through typed adapters. Report
isolated and combined profile/pane deltas and retain baseline/scenario
fingerprints.

Start with deterministic fixture scenarios, then seeded distributions. Add
monotonicity tests where the underlying scenario has an ordered expectation;
do not require monotonicity for genuinely interacting lineup changes.

Current evidence includes the versioned seeded input/output contracts,
deterministic full-cohort replay, input-order invariance, seed sensitivity,
fail-closed authority/profile scope, and a partial-pane panic regression. The
checked 2026-27 evidence set adds real 32-team season, lineup, and training-camp
sources; paired same-seed isolated NYR event effects; a typed deterministic
impact; and a combined 1,000-trial NYR-development/SEA-camp distribution. Every
numeric shock records a separate estimate-source fingerprint. The board remains
rank-withheld where required production profiles are missing, and the evidence
does not promote modeled assumptions into calibrated claims.

**W6 exit satisfied:** users can inspect what must go right or wrong, including
zero aggregate movement when a raw change does not cross a percentile boundary,
without rewriting observed history or filling missing panes with zero.

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
- profile lifecycle, supersession, demotion, and official-Frame migration
  guidance;
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

Current W3-W4 acquisition checkpoint (2026-07-29): the real all-league package
completes 14/16 required profiles after cache-aligned schedule, frozen team
strength, official NHL special-teams TOI, and the explicit 32-team prospect
program path. Rankings remain withheld at 0/32. The missing required methods
are organization depth (0/32) and recall depth (0/32). S1 automates prospect
composition without changing its hockey semantics; S2 owns the remaining
source decisions. W7 remains a
genuinely future untouched holdout.

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
| Retired methods disappear or silently alias to replacements | Orthogonal registry lifecycle, immutable sealed artifacts, explicit supersession edges, and new manifests for official-Frame migration. |
| Health rank is mistaken for outcome probability or window timing | Named composite purpose, target-specific forecast/calibration artifacts, and separately versioned classification methods. |
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
- Maintainers can deprecate, supersede, demote, or retire a profile without
  rewriting sealed artifacts or silently changing a saved Frame.
- Organization health, competitive-success forecasts, and window-timing
  classifications remain separately named, versioned, and validated claims.
- CLI, TUI, Web/API, JSON, and cards agree on every hockey value.
- Current boards contain all 32 organizations; historical boards contain the
  complete season-canonical league rather than fabricating 32.
- VTRACE, specs, plans, commands, surface parity, and release docs match the
  running build.
