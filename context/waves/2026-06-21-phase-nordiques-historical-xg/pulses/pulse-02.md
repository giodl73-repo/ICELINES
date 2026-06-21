# Phase Nordiques Pulse 02 - Multi-Season Fetch

## Result

Passed. `fetch money-puck` now accepts `--seasons N`, defaulting to `1`, and
walks backward from the selected season.

## Evidence

- `icelines-cli/src/cli.rs`
- `icelines-cli/src/commands/fetch.rs`
- `l0_moneypuck_season_window_counts_back_from_latest`
- `l0_moneypuck_season_window_rejects_non_consecutive_years`
- `l2_cmd_fetch_moneypuck_seasons_dry_run_lists_historical_urls`
