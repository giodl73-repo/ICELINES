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
status, and blockers.

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

## Surface

```powershell
icelines icecast affiliate-status-draft `
  --prior-snapshot prior-ahl.json `
  --crosswalk reviewed-identities.json `
  --camp camp.json `
  --nhl-team NYR --ahl-team "Hartford Wolf Pack" `
  --out status-review-draft.json

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
```

The apply command emits a sourced `AhlPreseasonRolloverConfig`; it does not
produce a roster. The UI-neutral review and rollover documents are
authoritative. CLI text is an inspection renderer; TUI and web review queues
remain planned.
