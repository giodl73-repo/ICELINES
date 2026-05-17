# Pulse 20: Serve schedule deep links

## Goal

Make right-pane schedule preview rows jump to the specific game when possible,
matching the score-preview deep-link behavior.

## Changes

- Carried `game_id` from the core schedule view into web `ScheduleRow`.
- Added hrefs to schedule summary rows: `/schedule` for the aggregate row and
  `/game/{game_id}` for game rows.
- Updated right-pane schedule preview cards to use row hrefs when present.
- Added focused assertions for schedule summary hrefs.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `cargo test -p icelines-web handlers::dashboard::tests::l0_dashboard_schedule_query_and_summary_preserve_team`
- `git diff --check`

## Status

Done.
