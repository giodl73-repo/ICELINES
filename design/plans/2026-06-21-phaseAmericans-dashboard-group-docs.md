# Phase Americans - Dashboard group docs truth gate

> Phase Americans aligns the embedded command reference with the dashboard group
> command recovery copy from Phase Maroons.

**Created:** 2026-06-21
**Status:** Closed - Phase Americans complete

---

## Frame

Phase Maroons updated dashboard command parser copy so unsupported group
mutations are described as not GET-backed and point to `/favorites` POST forms.
`COMMANDS.md` still used older "deferred in command bar" wording. Because
`/docs` embeds `COMMANDS.md`, the route needed a docs-truth fence.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Americans Goal 1 - Command reference copy** | Command docs should use the same recovery language as the parser. | `group create Prospects` says not GET-backed and points to `/favorites` POST forms or `icelines group`. |
| 2 | **Americans Goal 2 - Docs route fence** | The embedded `/docs` route should fail if stale wording returns. | Route test asserts the group recovery copy and rejects "deferred in command bar". |
| 3 | **Americans Goal 3 - Closeout** | The cleanup needs phase state. | Wave packet and indexes record the closed docs truth gate. |

---

## Non-goals

- Do not make dashboard command group mutations GET-backed.
- Do not change `/favorites` POST behavior.
- Do not add new command grammar.

---

## Closeout

Phase Americans is closed. The command reference now matches Maroons: dashboard
group mutation commands are not GET-backed, and users should use `/favorites`
POST forms or `icelines group`.

---

## Validation Expectations

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router docs`
- `git diff --check`
