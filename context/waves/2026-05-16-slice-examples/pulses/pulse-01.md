# Pulse 01: Simple player row selectors

## Goal

Prove SLICE can filter simple ICELINES-shaped rows without replacing ICELINES'
hockey query layer.

## Changes

- Add a dev-only `slice-core` dependency to `icelines-query`.
- Add selector tests for prepared player bio/stat rows.
- Document that ICELINES keeps stat IDs, aliases, windows, career aggregation,
  leaderboards, ranking, similarity, percentiles, and data requirements.

## Validation

- `cargo test -p icelines-query --test slice_simple_selector`
- `git diff --check`

## Status

Done.
