# Phase Nordiques Historical xG Inventory

## Surfaces

| Surface | File | Result |
|---|---|---|
| CLI parse | `icelines-cli/src/cli.rs` | `fetch money-puck` accepts bounded `--seasons N`; default remains `1`. |
| CLI execution | `icelines-cli/src/commands/fetch.rs` | MoneyPuck fetch iterates from selected season backward and prints each URL in dry-run mode. |
| Loader | `icelines-fetch/src/stats_loader.rs` | Regular-season MoneyPuck reads prefer any sealed snapshot for the requested season. |
| Docs | `COMMANDS.md`, `docs/guides/04-data.md` | Historical MoneyPuck fetch examples and season-aware read behavior are documented. |

## Non-Claims

- No MoneyPuck playoff endpoint is claimed.
- No historical MoneyPuck data is bundled.
- No goalie GSAx or high-danger save percentage source is promoted.
- No Signals, analytics-cache, or leaderboard semantics change.

## Validation Plan

1. CLI helper unit tests for season-window generation and invalid season shape.
2. CLI system dry-run test for `--seasons` URL listing.
3. Fetch loader test proving requested-season MoneyPuck rows beat active-season rows.
