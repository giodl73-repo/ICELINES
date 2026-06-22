# Phase Canadiens Strength - Scoring strength-state labels

## Status

Closed - 2026-06-22

## Goal

Start the Canadiens strength-state push with a bounded read-surface promotion:
cached play-by-play scoring summaries should show human-readable skater-state
labels while preserving the raw NHL `situationCode`.

## Scope

- Normalize four-character NHL `situationCode` values into skater-state labels
  like `5v5 (1551)`, `4v5 (1451)`, and `5v4 (1541)`.
- Keep unknown or malformed situation codes as raw labels instead of guessing.
- Apply the label in the shared scoring ViewModel so Web HTML, Web JSON, and
  downstream CLI/export projections that consume the ViewModel share the same
  contract.
- Do not add new `StatId` keys, filters, or broad leaderboard semantics in this
  phase.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-core scoring::tests::l0_scoring
cargo test -p icelines-web --test l1_router rocket_game_scoring_json
git diff --check
```
