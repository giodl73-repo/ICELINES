# Phase Canadiens Source - Goalie danger source gate

Status: Closed

## Intent

Advance high-danger goalie context without promoting unsupported metrics by
making the missing goalie danger source explicit on goalie leaderboard surfaces.

## Scope

- Add Web/API `goalie_high_danger_source` metadata for `/goalies` and
  `/api/v1/goalies`.
- Name the blocked metric family: goalie high-danger shots against, high-danger
  saves, and high-danger save percentage.
- Record required evidence before promotion: pinned goalie danger schema
  fixture, goalie identity join fixture, freshness/source-state contract, and
  missing-source non-claim copy.
- Preserve raw SV%, SA/60, and skater on-ice xGA as non-substitutes for goalie
  high-danger context.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router goalies`
- `git diff --check`
