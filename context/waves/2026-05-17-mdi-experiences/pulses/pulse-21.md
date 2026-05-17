# Pulse 21: Serve leader deep links

## Goal

Make leader previews jump directly to player cards instead of only opening the
generic Leaders page.

## Changes

- Added player-card hrefs to dashboard leader summary rows.
- Updated empty favorites/watchlist leader fallback cards to use row hrefs.
- Added a focused unit guard for player-card leader hrefs.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web handlers::dashboard::tests::l0_dashboard_leader_rows_link_to_player_cards`
- `git diff --check`

## Status

Done.
