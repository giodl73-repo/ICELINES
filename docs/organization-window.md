# The Window

The Window is IceLines' organization-health composition layer. It compares all
teams in one frozen cohort without hiding the difference between score,
confidence, coverage, and rank eligibility. It is descriptive until a sealed
calibration artifact supports a narrower predictive claim.

## Contracts

- `organization_profile_observation.v1` is one versioned signal for one team,
  season, cutoff, and horizon.
- `organization_window_manifest.v1` is the Frame: dimensions, weights, family
  caps, required profiles, cohort, normalization, and classification method.
- `organization_window_board.v1` is the complete sealed league result.
- Movement, history, scenario-impact, bridge/rebase, and calibration documents
  compare sealed boards without rewriting them.
- `card_document.v1` is the renderer-neutral focused-team projection.

The reviewed registry contains 32 candidate profiles: 17 ready for typed
adapters, 8 evaluation, 4 context-only, and 3 blocked. The blocked shift,
injury, and confirmed-cap signals receive no proxy score.

## Build and inspect

`icecast window-build` accepts any available sealed source documents. Repeated
`--team-lineup`, `--organization-lineup`, and `--schedule-rest` arguments are
allowed. Missing sources remain explicit missing observations.

```text
icelines icecast window-build \
  --season 20262027 \
  --as-of 2026-07-27 \
  --generated-at 2026-07-27T20:00:00-07:00 \
  --team-season-forecast season.json \
  --team-lineup nyr-lineup.json \
  --prospect-program prospects.json \
  --out window.json

icelines icecast window --input window.json
icelines icecast window --input window.json --team NYR
icelines icecast window --input window.json --markdown --out window-report.md
icelines icecast window --input window.json --team NYR --markdown --out nyr-window-report.md
icelines icecast window-card --input window.json --team NYR --out nyr-window-card.json
```

Markdown reports retain season, cutoff, Frame, manifest/board fingerprints,
league coverage, rank status, confidence, coverage, pane state, focused-team
profile evidence, blockers, and disclosures. `--json` and `--markdown` are
mutually exclusive; both project the same sealed board.

Comparable checkpoints use `window-movement` and `window-history`. Unlike
manifests are rejected unless the user supplies a sealed, reviewed bridge:

```text
icelines icecast window-movement \
  --earlier october.json \
  --later january.json \
  --out movement.json

icelines icecast window-rebase \
  --input october.json \
  --target-manifest balanced-v2.json \
  --bridge balanced-v1-to-v2-bridge.json \
  --out october-rebased.json

icelines icecast window-movement \
  --earlier october.json \
  --later january-v2.json \
  --bridge balanced-v1-to-v2-bridge.json \
  --out bridged-movement.json
```

`organization_window_bridge.v1` maps every target profile to exactly one
source profile and records a finite affine raw-value transform, rationale, and
evidence fingerprints. Rebase transforms raw observations and reruns the
canonical cohort normalizer and hierarchical scorer. It never patches an
overall score. Bridged movement separates observed-input change from the
method/manifest change and exposes any residual revaluation.

Typed scenarios add one or more upstream authority documents:

```text
icelines icecast window-scenario \
  --baseline baseline.json \
  --scenario deadline-addition.json \
  --scenario-id deadline-addition \
  --authority trade-authority.json \
  --out window-impact.json
```

IceLines can derive authorities from `team_season_forecast.v1` events (trade,
injury/return, goalie, and player-form/development),
`training_camp_league_forecast.v1`, and `line_combination_forecast.v1`.
Scenario attribution records direct raw-input changes, evidence changes,
league-cohort normalization effects, and unchanged profiles. Any changed
profile without a matching sealed authority fails closed. Combined scenarios
repeat `--authority`; interaction effects remain in the combined board rather
than being forced to equal isolated effects.

Rolling-origin calibration consumes one JSON origin document per frozen
checkpoint. Each origin embeds its sealed board, exact outcome cohort, complete
profile leakage audit, and a simple baseline value frozen before outcomes:

```text
icelines icecast window-calibrate \
  --target next-season-organization-value \
  --origin 2023-origin.json \
  --origin 2024-origin.json \
  --origin 2025-origin.json \
  --minimum-origins 3 \
  --out rolling-calibration.json
```

Freeze the role of every origin before inspecting its outcome, then evaluate
development evidence separately from the newest completed-season checkpoint:

```bash
icelines icecast window-evaluate \
  --target next-season-organization-value \
  --origin 2022-train.json \
  --origin 2023-train.json \
  --origin 2024-validation.json \
  --origin 2025-retrospective-holdout.json \
  --minimum-training-origins 2 \
  --out split-evaluation.json
```

Each input is a `WindowCalibrationEvaluationOriginInput`: `role` plus the
existing frozen calibration `origin`. The output headline is determined only
by the retrospective holdout. It is historical generalization evidence, not an
untouched future-season result.

`organization_window_rolling_calibration.v1` reports pooled and per-origin
error/rank correlation, pane metrics, leave-one-pane-out ablations,
organization stability, and a between-origin MAE confidence interval. It
refuses mixed Frames, duplicate origins, invalid board fingerprints,
incomplete dimensions/outcomes/audits, and invalid baselines. A leakage failure
blocks the claim. Trial noise remains explicitly `not_provided` until an
upstream board carries trial-level uncertainty; it is not conflated with
between-season variation.

JSON retains the full cohort. A team filter affects text projection only. Web
registers saved Frames by stable ID:

- `/window/balanced.v1/20262027`
- `/api/v1/window/balanced.v1/20262027`
- `/icecast/20262027/NYR/window`
- `/api/v1/cards/organization-window/20262027/NYR`

The TUI command `window-board` (or `window`) opens the sealed 32-team board in
two 16-team pages; `p` changes halves. `window-card NYR` or `window-card SEA`
opens focused detail; `t` toggles those teams and `p` changes between The
Window and The Insider.

## Authoring a profile

1. Produce a sealed core ViewModel or point-in-time authority record. Do not
   read files or the network from `icelines-core`.
2. Add one descriptor to
   `design/data/organization-window-profile-inventory.v1.json`. Its
   `(key, method_version)` pair is immutable and unique. Declare direction,
   raw unit, signal family, minimum cohort, evidence type, calibration target,
   and promotion gaps.
3. Implement a typed adapter returning `OrganizationProfileInput`. Reuse the
   upstream value; do not recreate its formula.
4. Add the profile to a manifest only after dependency and coverage review.
   Correlated profiles share a signal family and must stay within its cap.
5. Test schema/context errors, duplicates, non-finite data, missing evidence,
   input order, cohort minimum, and fingerprint stability.
6. Add a leakage audit and historical target before changing a claim from
   descriptive to calibrated.

Adding a registered profile does not require changing the normalization,
aggregation, comparison, card, CLI, TUI, or Web renderers.

## Custom Frames

Users may copy a manifest, change its ID/label, and alter positive weights,
coverage gates, required flags, or declared family caps. Dimension weights and
each dimension's profile weights must separately total 1. Unknown methods,
duplicate profiles, dependency cycles, invalid caps, incomplete cohorts, and
non-finite values fail before scoring.

Every canonical manifest receives a SHA-256 fingerprint. Different weights
therefore create a different comparison authority. Web serves only registered
saved Frames; local custom manifests remain a CLI/API concern.

## Compatibility and migration

- Schemas are additive within a version; a breaking wire change gets a new
  schema version.
- Hockey formula changes get a new profile method version.
- Official Frame changes get a new manifest version and fingerprint.
- Unbridged movement and scenario comparisons require the exact same manifest,
  organization catalog, season context, and profile methods.
- Cross-version subtraction is rejected. An intentional upgrade needs a
  separately reviewed, fingerprinted bridge/rebase artifact with complete
  one-to-one mappings. Missing mappings, invalid transforms, tampering, and
  mismatched source/target manifests fail closed.
- Old boards remain readable through their schema version. Unsupported versions
  fail explicitly instead of being silently reinterpreted.

## Official Frame changelog

### `balanced.v1` — evaluation

- Five dimensions: NHL strength, deployment, pipeline, development system, and
  resilience.
- Seventeen adapter-ready profiles with hierarchical weights and signal-family
  caps.
- Prospect conversion is optional for score composition, but uneven cohort
  availability withholds league ranks.
- Classification is descriptive, not Cup probability.
- The bundled July 27 evaluation board uses real 32-team prospect-program
  evidence and the available Rangers/Kraken lineup artifacts. It is deliberately
  partial and unranked.

## Cache, performance, and release policy

Boards and cards are immutable once sealed. Cache keys are board/manifest
fingerprints. Web uses fingerprint ETags with a short revalidation interval.
Core calculation performs no I/O and is deterministic under input reordering.

Every loaded board crosses a canonical validation boundary before it can be
rendered, compared, rebased, or calibrated. Validation checks the checksum and
then rebuilds normalized scores, aggregates, classifications, drivers,
blockers, and ranks from the stored raw observations. Rehashing hand-edited
output values therefore cannot turn them into a trusted artifact.

### Performance baseline

The 2026-07-27 Windows x64 release baseline uses the 746,508-byte bundled
all-32 evaluation board and the optimized `icelines 0.26.0` binary. Five
consecutive offline focused-team runs of:

```powershell
icelines --no-live icecast window `
  --input examples/organization-window-board-evaluation-2026-27.json `
  --team NYR
```

measured 4,111.6 ms for the cold process/OS/antivirus start and 45.1, 44.7,
44.2, and 47.9 ms warm (44.9 ms warm median). These are reproducible reference
measurements, not a universal latency guarantee. Web and TUI retain the
validated embedded board in a process-local `OnceLock`; CLI remains one-shot.
No cache entry can bypass canonical replay validation, and fingerprints—not
filesystem timestamps—remain the cache identity.

Run the live narrow/desktop browser and semantic review with:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/window-browser-review.ps1
```

The gate starts IceLines with `--no-live`, checks skip navigation, main focus,
table captions/headers, rank-gate copy, focused-team isolation, and both card
pages, then captures nonblank all-team/focused/card images at desktop, tablet,
and 390-pixel mobile widths. Browser background networking and sync are
disabled for the capture process.

Release gates validate inventory counts, dependency graphs, schemas,
fingerprints, the complete cohort, tied/inverse/target-range normalization,
missingness, history/scenario compatibility, leakage status, surface parity,
80-column and narrow-browser output, clippy, formatting, tests, audit, package,
and no-network smoke behavior. Release notes label every Frame calibrated,
inconclusive, evaluation, or blocked.
