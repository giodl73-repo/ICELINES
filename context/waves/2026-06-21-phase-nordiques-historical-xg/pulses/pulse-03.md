# Phase Nordiques Pulse 03 - Season-Aware Reads

## Result

Passed. The MoneyPuck loader now reads any sealed snapshot for the requested
season, preventing active-season xG rows from leaking into historical queries.

## Evidence

- `icelines-fetch/src/stats_loader.rs`
- `l1_load_into_repo_reads_moneypuck_for_requested_historical_season`
- `l1_load_into_repo_does_not_reuse_active_moneypuck_for_missing_historical_season`
