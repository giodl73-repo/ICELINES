# Pulse 22: Serve row-specific preview links

## Goal

Let center dashboard preview rows use the specific hrefs added by recent
leader, score, and schedule deep-link pulses.

## Changes

- Updated the center workspace preview table to prefer `row.href`.
- Kept the active workspace URL as a fallback for rows without a specific href.
- Added a route assertion to keep the fallback link behavior covered.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router dashboard`
- `git diff --check`

## Status

Done.
