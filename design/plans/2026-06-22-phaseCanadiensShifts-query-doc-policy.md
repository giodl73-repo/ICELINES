# Phase Canadiens Shifts - Query doc policy

Status: Closed

## Intent

Align query and dashboard docs with the current historical shift policy. Current
ICELINES has scoring-event strength splits from play-by-play, but true on-ice
5v5/PP/PK rates and shift-backed deployment joins remain parked until a verified
shift source, bundle, fetch, fixture, and join policy ships.

## Scope

- Update `design/specs/query-engine.md` to stop calling `ShiftProfile`
  infrastructure source authority for on-ice strength rates.
- Update `design/specs/dashboard-engine.md` to mark `QuerySource::Shifts` as
  parked behind the future shift policy.
- Update the query guide source to label situational breakdown as parked.

## Validation

- `git diff --check`
