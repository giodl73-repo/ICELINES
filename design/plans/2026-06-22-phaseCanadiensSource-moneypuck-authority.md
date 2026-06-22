# Phase Canadiens Source - MoneyPuck authority

Status: Closed

## Intent

Make the existing skater MoneyPuck snapshot contract visible on leaders
surfaces without promoting adjacent unsupported advanced metrics.

## Scope

- Add Web/API `moneypuck_source` metadata for `/leaders` and
  `/api/v1/leaders`.
- Name covered optional snapshot metrics: individual xG, ixG/60, on-ice xGF,
  on-ice xGA, xGF%, CF%, and FF%.
- Name blocked adjacent claims: goalie xGA/GSAx, goalie high-danger SV%,
  skater high-danger chance %, zone entries, and deployment recommendations.
- Preserve the current missing-snapshot behavior: unavailable MoneyPuck values
  remain absent, not zero.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router leaders`
- `git diff --check`
