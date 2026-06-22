# Phase Canadiens Source - Tonight Intel authority

Status: Closed

## Intent

Bring Tonight Intel onto the same scoring source authority contract as game,
team, and player scoring reports.

## Scope

- Add `meta.source_authority` to `/api/v1/tonight/intel`.
- Render the scoring source authority label in the Tonight Intel HTML banner.
- Reuse the cached official NHL play-by-play authority helper.
- Cover loaded JSON and missing-cache HTML paths in router tests.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router tonight_intel`
- `git diff --check`
