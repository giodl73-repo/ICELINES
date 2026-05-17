# Pulse 19: Serve score chip deep links

## Goal

Make score previews jump to the specific game when possible instead of always
falling back to the generic Scores page.

## Changes

- Carried `game_id` from the core scores view into web `ScoreRow`.
- Added optional dashboard summary hrefs so score chips can target
  `/game/{game_id}` and slate chips can target the active scores date.
- Linked score-table score/state cells to the matching game page.
- Added focused assertions for score summary hrefs.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web --test l1_static l1_static_css_contains_prince_route_layout_classes`
- `cargo test -p icelines-web handlers::dashboard::tests::l0_dashboard_scores_summary_counts_game_states`
- `git diff --check`

## Status

Done.
