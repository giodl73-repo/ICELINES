# Card System Authority Freeze — 2026-07-21

**Date**: 2026-07-21
**Status**: Recorded Wave 0 baseline
**Plan**: [`../plans/2026-07-21-ui-neutral-card-system.md`](../plans/2026-07-21-ui-neutral-card-system.md)
**Specification**: [`../specs/ui-neutral-card-system.md`](../specs/ui-neutral-card-system.md)

## Decision

The first team-card fixtures use the current official 2026-27 roster snapshot
with completed 2025-26 player evidence and the current IceCast forecast and
development-calibration artifacts. Rangers and Kraken cards, their
side-by-side comparison, fantasy decisions, and simulation views must join
through the mainline IceLines season/team/player/game identities. No renderer
or card-only database may reconcile these streams.

This is an input and gap freeze, not approval of today's raw player scores or
line assignments for display.

## Roster authority

| Field | Frozen value |
|---|---|
| Season | `20262027` |
| Snapshot | `20262027-2026-07-21-rosters` |
| Created | `2026-07-21T17:05:11Z` |
| Evidence at | `2026-07-21T17:05:11.530251600+00:00` |
| Evidence source | `official_nhl_api_live` |
| State | sealed |
| Official capture SHA-256 | `3dfe200746ccccefece68dd6e1405c3c7b2c3f31ff427337c496f362fd2390d8` |

| Team | F | D | G | Total | Official headshots | Roster SHA-256 |
|---|---:|---:|---:|---:|---:|---|
| NYR | 14 | 9 | 3 | 26 | 26 | `b3bacf1f6cf450833d3dc5f3936135be5bbc82b580dede55727537d2a8fab1cc` |
| SEA | 12 | 7 | 2 | 21 | 21 | `336ea7ea6fd4207ee4bf313bf7fa2c9c5a61f535747518f886906ffcdf82f456` |

Tye Kartye resolves to NYR as player `8481789` in this snapshot. The headshot
records use official NHL asset URLs and have complete coverage for both frozen
team rosters. Card assets still require typed missing/stale/fallback behavior
because later snapshots may not be complete.

## Forecast and scenario fixtures

| Artifact | Role | SHA-256 |
|---|---|---|
| `examples/icecast-alp-development-variance.json` | NYR scenario events | `8a8c769dfedfb847a510507261a0184ad008d85312d2a34464fa9269334178f9` |
| `examples/icecast-brv-development-variance.json` | SEA scenario events | `8211c6f2a7b3273b45c5cd311f07d928a73c60a653d2cf881da6790235f09c7f` |
| NYR calibrated multilens report | NYR forecast evidence | `652878f85a7e755db07e0a84cfb3932301aa99482b76805c22656a51587d6ceb` |
| SEA calibrated multilens report | SEA forecast evidence | `3561efcb35b7810016ce3fb4df36497c1e48a2997951e917b6c16e0d62c639a4` |
| Development calibration v2 report | player-event priors | `2b3c115c0cd3905a9a1cdc0f33471189d34194c8cb2b5991293fe1996d31d814` |
| Team-ceiling authority report | current roster/player lens probe | `e3d7037bdb4afdda19fc5d7e8afc5fb320fe7c1228f69975710bede1b76204ca` |

The development calibration contains 11,156 player transitions across 89
cohorts. Its current global rates are 16.58% breakout and 25.40% downturn,
with median changes of +3.24 and -3.19 respectively. These priors remain model
inputs; they are not card labels unless a builder supplies the correct cohort,
provenance, and interpretation.

### Frozen headline outputs

| Team | Baseline points | P10 / P50 / P90 | Playoffs | Cup | Forced all-five ceiling points | Ceiling playoffs | Ceiling Cup |
|---|---:|---|---:|---:|---:|---:|---:|
| NYR | 98.93 | 88 / 99 / 110 | 65.25% | 4.93% | 103.77 | 82.52% | 9.99% |
| SEA | 89.35 | 78 / 89 / 101 | 40.75% | 1.36% | 93.84 | 60.58% | 2.63% |

The all-five values are conditional ceiling simulations, not baseline
forecasts. They must never be presented as the sum of standings-point deltas.

## Mainline data-stream map

| Domain | Existing authority | Required shared join | Wave 0 state |
|---|---|---|---|
| Roster and assets | sealed official roster snapshot | season + stable team/player ID + evidence cutoff | Ready |
| Completed player evidence | 2025-26 stats and team-ceiling lenses | player ID + stats season + sample/coverage | Partial |
| Projected lineup | roster/depth paths | roster snapshot + role + evidence state | Missing cross-season builder |
| Season simulation | IceCast league run and focused reports | calendar/model/roster/scenario fingerprints | Partial; promote focused outputs |
| Fantasy scoring | league scoring and roster-shape views | player ID + scoring scheme + eligibility cutoff | Partial; retain scheme identity |
| Fantasy schedule fit | weekly schedule/equivalence views | game ID + calendar fingerprint + timezone/week | Planned |
| Injuries | injury/IR decision views | player ID + observed/effective times + source | Planned mainline event join |
| Transactions | waiver, pickup, and trade views/events | player/team IDs + effective time + roster snapshot | Planned mainline event join |
| Side-by-side | no canonical wrapper yet | compatible document fingerprints + aligned metric keys | Planned in core |

Fantasy value and simulation impact are derived measures attached to shared
identities. They do not overwrite the player score and do not become a second
roster, schedule, injury, or transaction truth.

## Bug and gap ledger

| ID | Finding | Severity | Owner | Required gate |
|---|---|---|---|---|
| CARD-001 | `icelines team NYR` and `SEA` fail because 2026-27 is not bundled and the command cannot select a 2026-27 roster with 2025-26 stats. | Blocking | core/CLI data resolution | cross-season roster/stats fixture passes for both teams |
| CARD-002 | Raw team-ceiling lens values are not comparable: fantasy values can exceed 300 while other lenses use roughly 0-100 scales. | Blocking | core scoring | versioned position-aware 0-100 score normalization and bounds tests |
| CARD-003 | Three-game goalie Dylan Garand receives 98.66 across all lenses and ranks above Igor Shesterkin. | Blocking | core goalie scoring | low-sample shrinkage/coverage fixture prevents unsupported elite rank |
| CARD-004 | Team-ceiling output lacks a reliable team-level coverage field for the card probe. | High | core ViewModel | typed coverage and missing-evidence serialization test |
| CARD-005 | Exact 4F/3D/2G projected assignments are not authoritative in the cross-season path. | Blocking | core lineup builder | legal-shape, no-duplicate, goalie-slot, and extras tests |
| CARD-006 | Highlighted player effects are combined scenario inputs, not isolated paired team impacts. | Blocking | IceCast/core | one-event paired-run reconciliation tests |
| CARD-007 | Fantasy and simulation outputs can be consumed as reports without one explicit shared evidence-graph contract. | High | core architecture | identity/provenance join tests across roster, calendar, fantasy, and simulation fixtures |
| CARD-008 | Renderers have no canonical side-by-side compatibility and delta contract. | High | core card builder | NYR/SEA comparison golden plus incompatible-cutoff refusal |

The incorrect `--season` probe used during discovery was operator error; the
supported command uses `--roster-season` and `--stats-season` and is not logged
as a product defect.

## Display decisions frozen for Wave 1

- Page 1 uses a new versioned IceLines player score on a bounded 0-100 scale;
  it does not display a raw lens average or the current team ensemble score.
- Skaters and goalies may use different components, but their displayed scores
  require explicit calibration and coverage policy.
- Line and pair projection belongs in core and carries actual, reported,
  estimated, or scenario authority.
- Headshots come from authoritative asset records; missing assets receive a
  deterministic non-likeness fallback.
- Page 2 uses isolated paired simulations for named player deltas.
- Side-by-side deltas are computed in a core comparison builder using compatible
  documents. Renderers only arrange the result.
- Historical replay binds every join to its historical evidence cutoff and may
  not use information learned later in the season.

## Wave 0 exit assessment

The exact showcase inputs and known gaps are now recorded, and no renderer must
invent a value. Wave 0 is complete as a discovery freeze. CARD-001 through
CARD-008 are implementation gates for the appropriate later waves, not reasons
to conceal missing information in presentation code.
