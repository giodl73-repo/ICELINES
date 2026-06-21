# Phase Stags - Watch deployment docs truth gate

> Phase Stags aligns the embedded command reference with dashboard watch
> deployment recovery copy.

**Created:** 2026-06-21
**Status:** Closed - Phase Stags complete

---

## Frame

Dashboard watch deployment/team editing remains intentionally unsupported because
the shared watch-rule mutation intent only validates player-rule dimensions. The
parser recovery copy points users to CLI deployment preview or `/watchlist` for
player rules. `COMMANDS.md` only named CLI preview, so `/docs` needed a small
truth fence.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Stags Goal 1 - Command reference copy** | Command docs should match parser recovery. | `watch deployment TOR` points to CLI preview or `/watchlist` player rules. |
| 2 | **Stags Goal 2 - Docs route fence** | Embedded docs should catch stale recovery text. | `/docs` test asserts `watch deployment TOR`, CLI preview, and `/watchlist`. |
| 3 | **Stags Goal 3 - Closeout** | The cleanup needs phase state. | Wave packet and indexes record the closed docs truth gate. |

---

## Non-goals

- Do not add arbitrary team/deployment watch-rule editing.
- Do not change player watch-rule create/toggle/delete behavior.
- Do not make dashboard command text mutate through GET.

---

## Closeout

Phase Stags is closed. The command reference now matches dashboard parser
recovery: `watch deployment TOR` remains deferred for dashboard mutation and
points to CLI preview or `/watchlist` player rules.

---

## Validation Expectations

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router docs`
- `git diff --check`
