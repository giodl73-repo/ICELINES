# Phase Maroons - Dashboard group copy truth gate

> Phase Maroons aligns dashboard command error copy with the current Favorites
> group editor contract.

**Created:** 2026-06-21
**Status:** Closed - Phase Maroons complete

---

## Frame

Whalers added POST-backed `/favorites` group editor forms and JSON routes while
keeping dashboard command navigation GET-only. The dashboard command parser still
sent group mutation recovery copy to only TUI/CLI paths. Phase Maroons keeps the
GET mutation rejection intact but points users to the current web POST forms.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Maroons Goal 1 - Recovery copy** | Unsupported dashboard group mutations should name the supported web editor. | Error copy says group edits are not GET-backed and points to `/favorites` POST forms or `icelines group`. |
| 2 | **Maroons Goal 2 - Parser fence** | Copy changes should preserve command safety. | Parser test still rejects `group create Prospects` and checks the new recovery target. |
| 3 | **Maroons Goal 3 - Closeout** | The route-copy gate needs traceable phase state. | Wave packet and indexes record the closed cleanup. |

---

## Non-goals

- Do not make dashboard command text mutate groups through GET.
- Do not change `/favorites` group editor behavior.
- Do not add new group mutation commands.

---

## Closeout

Phase Maroons is closed. Dashboard `group create/delete/rename/member` commands
remain rejected as unsupported GET-backed mutations, and the recovery text now
points to `/favorites` POST forms or `icelines group`.

---

## Validation Expectations

- `cargo fmt --check`
- `cargo test -p icelines-web dashboard_command_group`
- `git diff --check`
