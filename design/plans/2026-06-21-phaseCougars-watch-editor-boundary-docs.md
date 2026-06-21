# Phase Cougars - Watch editor boundary docs truth gate

> Phase Cougars clarifies the watch-rule editor boundary in the embedded command
> reference.

**Created:** 2026-06-21
**Status:** Closed - Phase Cougars complete

---

## Frame

The web/TUI watch-rule editors intentionally support player-rule create,
enable/disable, and web delete. Team/deployment editing remains a CLI
preview/save path through `icelines watch deployment ... --save` until the shared
mutation intent carries validated team/deployment fields. Phase Cougars tightens
the command reference and `/docs` coverage for that boundary.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Cougars Goal 1 - Boundary copy** | The docs should not imply web/TUI deployment editing. | Watch editor copy names player-rule-only web/TUI support and CLI deployment preview/save. |
| 2 | **Cougars Goal 2 - Docs route fence** | Embedded docs should preserve the boundary. | `/docs` test asserts player-rule support, CLI deployment save, and shared-intent limitation copy. |
| 3 | **Cougars Goal 3 - Closeout** | The cleanup needs phase state. | Wave packet and indexes record the closed docs truth gate. |

---

## Non-goals

- Do not change watch-rule runtime behavior.
- Do not add web/TUI team/deployment editing.
- Do not remove the CLI deployment preview/save path.

---

## Closeout

Phase Cougars is closed. The command reference now clearly separates web/TUI
player-rule editing from CLI team/deployment preview/save behavior.

---

## Validation Expectations

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router docs`
- `git diff --check`
