# Phase Canadiens Source - Goalie xGA source gate

Status: Closed

## Intent

Advance the goalie GSAx work without promoting unsupported metrics by making
the missing goalie xGA source explicit on the goalie leaderboard surfaces.

## Scope

- Add Web/API `goalie_xga_source` metadata for `/goalies` and
  `/api/v1/goalies`.
- Name the blocked metric family: goalie xGA, goalie xGA/60, GSAx, and GSAx/60.
- Record required evidence before promotion: pinned schema fixture, goalie
  identity join fixture, freshness/source-state contract, and missing-source
  non-claim copy.
- Preserve QS% and SA/60 as workload/quality metrics, not goalie xGA
  substitutes.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router goalies`
- `git diff --check`
