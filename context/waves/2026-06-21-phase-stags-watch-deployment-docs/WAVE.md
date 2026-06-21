# Phase Stags Watch Deployment Docs

## Scope

Align `COMMANDS.md` and `/docs` with dashboard watch deployment recovery copy.

## Entry Posture

- Dashboard parser rejects `watch deployment` as unsupported team/deployment
  editing.
- Parser recovery points to CLI deployment preview or `/watchlist` player rules.
- Command docs only named CLI preview.

## Goals

1. Update command reference copy for watch deployment recovery.
2. Add `/docs` route coverage for the recovery text.
3. Preserve no-GET-mutation and player-rule-only web boundaries.
4. Close the phase with validation evidence recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Command reference and docs route update | passed; watch deployment docs now name `/watchlist` recovery |
| 02 | Close Phase Stags | passed; final docs truth claims recorded |

## Validation Posture

- `cargo test -p icelines-web --test l1_router docs`
- `cargo fmt --check`
- `git diff --check`

## Closeout

Phase Stags is closed. `/docs` now names CLI deployment preview and `/watchlist`
player-rule recovery for unsupported dashboard watch deployment edits.
