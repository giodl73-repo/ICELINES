# WP-009 Pulse 01 - Major analytics cache specification baseline

## Scope

Accept a DCR/specification baseline for a shared major analytics cache before
building future hockey decision-support surfaces.

## Evidence

- Added coach/analyst mission and CONOPS coverage for cache-backed decision
  surfaces.
- Added `CHG-072`, `CON-010`, `REQ-CACHE-001` through `REQ-CACHE-004`,
  `IF-CACHE-001`, `VAL-011`, `INT-009`, `WP-009`, `ADR-VT-006`, and `CR-033`.
- Defined the target cache evidence envelope: version, scope, source window,
  provenance, freshness/staleness, quality/completeness, warnings, invalidation
  keys, methodology, disclosure, and consumer-contract version.
- Constrained cache reads to local/snapshot state and kept live fetches out of
  query-time/read paths.
- Constrained downstream dashboard, scout, player-card, line, goalie, practice,
  postgame, and agent surfaces to consume prepared evidence rather than
  recomputing source-state, confidence, or methodology meaning locally.
- Preserved non-claims: no autonomous coaching authority, prediction accuracy,
  betting value, injury certainty, line-chemistry causality, or complete-world
  truth.
- Docs gates:
  - `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
  - `git -C C:\src\TRACKER\repos\applied-systems\icelines diff --check`

## Validation disposition

| Scenario | Result | Notes |
|---|---|---|
| VAL-011 | target_spec_pending | Specification baseline accepted; cache schema/source-state/invalidation/consumer fixtures remain future implementation work. |

## Required next evidence

- Schema fixture for compatible and incompatible cache records.
- Source-state fixtures for complete, stale, partial, missing, unsupported, and
  invalidated cache inputs.
- No-live read-path proof.
- Consumer-envelope demo for one coach/scout/report/card-style surface.

## Decision

`WP-009` is opened as `target_spec_pending`; its documentation/specification gate
passes.

The major analytics cache is the next product foundation, but ICELINES must not
claim cache-backed screens, reports, cards, line views, goalie views,
practice/postgame reports, or agent decisions until future implementation
evidence passes.
