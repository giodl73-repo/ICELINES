# Phase Cougars Watch Editor Boundary Docs

## Scope

Clarify `COMMANDS.md` and `/docs` watch-rule editor boundary copy.

## Entry Posture

- Web/TUI support player-rule create and toggles; web also supports persisted
  player-rule delete.
- CLI supports `watch deployment ... --save` preview/save.
- The shared web/TUI mutation intent does not yet validate arbitrary
  team/deployment dimensions.

## Goals

1. Tighten command reference copy for watch-rule editor boundaries.
2. Add `/docs` route coverage for the boundary text.
3. Preserve runtime behavior unchanged.
4. Close the phase with validation evidence recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Command reference and docs route update | passed; watch editor boundary copy clarified |
| 02 | Close Phase Cougars | passed; final docs truth claims recorded |

## Validation Posture

- `cargo test -p icelines-web --test l1_router docs`
- `cargo fmt --check`
- `git diff --check`

## Closeout

Phase Cougars is closed. `/docs` now preserves the watch editor boundary:
web/TUI player-rule editing stays narrow, while team/deployment preview/save
remains CLI-backed until the shared mutation intent grows validated fields.
