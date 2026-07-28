# AHL preseason rollover

**Status:** Implemented (planning foundation)

The preseason rollover reconciles a completed prior-season official affiliate
roster with a current NHL training-camp pool. It answers whether IceLines has
enough reviewed, assignable candidates to author a sourced
`preseason_projection` pool. It does not create an official roster or silently
fill an affiliate lineup.

## Inputs

- prior `ahl_roster_stats.v1` snapshot;
- snapshot-bound `ahl_identity_crosswalk.v1`, including pending rows;
- current `training_camp_simulation` input;
- matching `training_camp_forecast.v1`; and
- target season, explicit NHL/AHL team binding, as-of date, absolute sources,
  and optional sourced decisions for prior players retained, departed, or
  assigned to another league.

Organization-status decisions are authored through a separate
`ahl_preseason_organization_review.v1` gate. Its draft is bound to the exact
historical snapshot, current camp, and SHA-256 fingerprint of the reviewed
identity crosswalk. Any later identity approval or remap makes the draft stale.

The prior crosswalk must exactly cover the official roster. Reviewed canonical
NHL identities are unique. Camp input and forecast player sets must match.
Prior-player decisions require reviewed identities, absolute evidence URLs, and
notes.

The generated draft is deliberately non-applicable. A reviewer must first
finish every identity decision, then mark each reviewed prior-only player as
`retained`, `departed`, or `other_league`, attach source URLs and a note, set
`draft: false`, and add their name plus an RFC3339 timestamp. Current-camp
players cannot receive redundant organization-status decisions.

The explicit NHL team binding is required because some historical AHL catalog
rows do not carry affiliation metadata. When a prior snapshot does include an
affiliate, it must agree with the configured NHL team; a missing historical
label is not treated as contradictory evidence.

## Reconciliation

Reviewed prior identities and camp candidates reconcile only by canonical NHL
player ID. Name similarity never merges identities. Each output row preserves
its origins, camp make/cut probabilities, modal NHL-roster status, waiver
status, exact primary and multi-position eligibility, projected score, and
blockers. A sealed league camp forecast missing its primary-position
eligibility is rejected as position-incomplete instead of being reconstructed
from a coarse forward/defense/goalie group.

Camp players outside the modal NHL roster count toward the projected affiliate
pool only when waiver-exempt. Non-exempt players remain waiver-gated. Prior-only
players require a sourced retained/departed/other-league decision. Pending
crosswalk rows remain identity-review blockers.

## Readiness

`projection_ready` requires at least 12 projectable forwards, six defensemen,
and two goalies, with no unresolved prior identities or prior-player
organization-status reviews. This is candidate-pool readiness only. The next
adapter must still provide professional-game totals, development-rule facts,
contracts, injuries, final assignment rights, and player projections.

`ahl_preseason_league_facts_workboard.v1` is the NHL-ID-keyed composition
boundary for that next adapter. It joins a complete league rollover to a
matching professional-game ledger, retains every team/player row, and names
missing identity review, organization status, waiver clearance, exact
position, projected score, prospect status, recall readiness, professional
games, final development-rule qualification, and assignment authority. It is
a workboard, not an assignment model: even a fully measured candidate remains
blocked until explicit assignment, prospect, and recall authorities exist.
Goalies do not receive the dressed-skater development-rule blocker.

The workboard carries a canonical SHA-256 fingerprint. A generated
`ahl_preseason_league_facts_overlay.v1` draft exactly lists every canonical
candidate but contains no facts and cannot be applied. A finalized overlay
requires reviewer, RFC3339 timestamp, absolute evidence URLs, notes, and the
exact source fingerprint. Each optional field clears only its corresponding
blocker. Explicit `assigned_to_affiliate: false` changes the row to
`not_assigned` and removes it from the candidate pool without relabeling it as
departed or assigned to another league. Conflicting sealed facts fail closed.
The application retains both source and result fingerprints.

`ahl_preseason_league_projection_inputs.v1` lowers the reviewed application
into the existing `AhlAffiliateProjectionInput` contract. Lowering requires a
final professional-game policy, a matching threshold, and explicit dated AHL
dressed-roster rule authority. A team is emitted only when it has no identity
or player-fact blockers and the canonical affiliate builder can dress 12F/6D/2G
while satisfying the development minimum. Every other organization remains a
named failure with its blocker counts or optimizer reason; partial league
success never shrinks the requested cohort silently. Team rollover sources and
per-player review evidence survive in the preseason pool authority.

## Surface

```powershell
icelines icecast affiliate-status-draft `
  --prior-snapshot prior-ahl.json `
  --crosswalk reviewed-identities.json `
  --camp camp.json `
  --nhl-team NYR --ahl-team "Hartford Wolf Pack" `
  --out status-review-draft.json

icelines icecast affiliate-status-show `
  --review status-review-draft.json

icelines icecast affiliate-status-apply `
  --prior-snapshot prior-ahl.json `
  --crosswalk reviewed-identities.json `
  --camp camp.json `
  --review status-review.json `
  --config rollover-base.json `
  --out rollover-config.json

icelines icecast affiliate-rollover `
  --prior-snapshot prior-ahl.json `
  --crosswalk prior-identities.json `
  --camp camp.json `
  --camp-forecast camp-forecast.json `
  --config rollover-config.json `
  --json --out rollover.json

icelines icecast affiliate-facts-board `
  --rollover league-rollover.json `
  --professional-games professional-games.json `
  --json --out affiliate-facts-board.json

icelines icecast affiliate-facts-draft `
  --workboard affiliate-facts-board.json `
  --out affiliate-facts-overlay-draft.json

icelines icecast affiliate-facts-apply `
  --workboard affiliate-facts-board.json `
  --overlay affiliate-facts-overlay-final.json `
  --json --out affiliate-facts-application.json

icelines icecast affiliate-inputs-league `
  --application affiliate-facts-application.json `
  --rule ahl-development-rule-final.json `
  --json --out affiliate-inputs-league.json
```

The apply command emits a sourced `AhlPreseasonRolloverConfig`; it does not
produce a roster. The UI-neutral review and rollover documents are
authoritative. `affiliate-status-show` is a read-only inspection renderer and
recomputes blocker counts from the rows, warning when declared counts are
stale. TUI and web review queues remain planned.

`ahl_organization_status_ledger.v1` may prefill that review from dated official
NHL landing current-team facts. Equality with the reviewed organization proves
`retained`; a different team in the sealed target NHL cohort proves
`departed`. Missing teams, stale facts, and teams outside the cohort remain
unresolved. The ledger cannot emit `other_league`, infer status from camp
absence, establish target NHL/AHL assignment, or finalize the review. Its
application is fingerprint-bound to the exact league draft and preserves the
human reviewer/timestamp gate.

The official assignment evidence path begins with
`ahl_transaction_snapshot.v1`. It captures the complete target-season AHL
`ADD`/`DEL` stream, provider team catalog, page totals, URLs, and verified cache
acquisition times. Provider identities remain outside canonical NHL identity
until joined through the reviewed crosswalk. The source snapshot itself does
not interpret a deletion as a destination, an addition as opening-night
assignment, or absence as any status; those semantics belong to a separately
versioned, cutoff-aware state ledger.

`ahl_transaction_state_ledger.v1` supplies that interpretation without
changing the workboard. It groups each provider player at a caller-selected
cutoff and evaluates only the latest calendar-date event set. One latest ADD
establishes assignment when there is no same-team ADD/DEL or multiple-ADD
conflict; an ADD paired with a DEL from another club establishes the explicit
destination. DEL-only latest sets establish removal from the observed AHL
transaction state. Unknown event kinds, multiple ADD destinations, and
same-team ADD/DEL sets remain typed ambiguity because the source does not
provide a trusted intraday order. Canonical player identity comes only from
the reviewed crosswalk, and organization comes only from the target-season
affiliation catalog. Source, identity, affiliation, cutoff, method, result,
and counts are fingerprint-bound and tamper-validated.

The completed 2025-26 replay reduces 4,011 events to 1,161 latest player
states: 695 assigned, 403 removed, and 63 ambiguous; 1,149 join to reviewed
NHL identities and 12 retain only provider identity. The 2026-27 snapshot has
zero source rows as of July 28, so its ledger contains zero players and clears
no assignment blockers. Absence remains a no-read. A separate narrow
application must prove that a ledger row corresponds to the exact target
workboard organization before it may clear `AssignmentAuthority`.

The real July 28 league run rebuilt the camp seal with 933/933 exact position
lists, including 26 multi-position players, then composed all 32 affiliates.
It exposes 1,371 viable candidates and zero facts-ready candidates. The
remaining queues include 1,371 assignment, prospect-status, and recall-
readiness authorities; 1,174 organization-status and projected-score facts;
144 waiver clearances; zero exact-position gaps after official NHL landing
position composition; 52 missing professional-game histories; and 1,202
skaters awaiting final development-rule
qualification. These counts describe missing authority and are not roster
predictions.

The July 28 run used verified official landing acquisitions from July 25-26
and covered all 1,282 canonical candidates.
The organization-status ledger resolved 549 of 1,174 prior-only appearances:
425 retained and 124 departed. The other 625 landing documents had no current
NHL team and remain explicit review work; none were inferred as departed or as
playing in another league.
