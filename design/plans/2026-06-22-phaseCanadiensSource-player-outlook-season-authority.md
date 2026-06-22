# Phase Canadiens Source - Player outlook season-stat authority

Status: Closed

## Intent

Make player scoring outlook source authority explicit for the season-total stats
that drive player pace rows, while keeping schedule authority separate.

## Scope

- Add `meta.season_stat_authority` to player outlook JSON.
- Render the season-stat authority label on player outlook HTML pages.
- Keep team outlook scoped to schedule authority only.
- Cover player outlook JSON and HTML authority in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router outlook`
- `git diff --check`
