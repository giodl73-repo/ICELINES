# Phase Canadiens Source - Outlook schedule authority

Status: Closed

## Intent

Add explicit schedule authority metadata to scoring outlook surfaces while
avoiding any claim that schedule data covers play-by-play scoring events,
expected goals, or betting forecasts.

## Scope

- Add `meta.schedule_authority` to player and team scoring outlook JSON.
- Render the same schedule authority label in the shared scoring outlook HTML
  banner.
- Cover remaining games, projected-finish context, team goals for/against, and
  recent form as schedule-backed fields.
- Cover missing schedule authority in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router outlook`
- `git diff --check`
