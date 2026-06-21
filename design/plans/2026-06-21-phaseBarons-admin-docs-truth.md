# Phase Barons - Admin docs truth gate

> Phase Barons aligns the embedded command reference and `/docs` route with the
> Phase Seals admin install/remove safety contract.

**Created:** 2026-06-21
**Status:** Closed - Phase Barons complete

---

## Frame

Phase Seals mounted bounded admin install/remove routes. One active command
reference block still described web install/remove as deferred. Phase Barons
removes that stale claim and adds route coverage so the embedded web docs keep
the local-only confirmation contract visible.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Barons Goal 1 - Reference truth** | `COMMANDS.md` is embedded by `/docs` and must not contradict route behavior. | Admin operations copy names bundled install and scoped remove. |
| 2 | **Barons Goal 2 - Route fence** | Docs drift should fail a focused route test. | `/docs` test asserts the install/remove confirmation contract and rejects stale deferred wording. |
| 3 | **Barons Goal 3 - Closeout** | The truth gate needs a phase record. | Wave packet and indexes record the closed cleanup. |

---

## Non-goals

- Do not change admin runtime behavior.
- Do not broaden install beyond embedded bundled seasons.
- Do not broaden remove beyond `~/.icelines/seasons/<season>`.
- Do not add persistent report-toggle writes.

---

## Closeout

Phase Barons is closed. `COMMANDS.md` and `/docs` now describe web admin install
as embedded bundled-season only with exact `INSTALL <season>` confirmation, and
web admin remove as scoped installed-season deletion with exact
`REMOVE <season>` confirmation.

---

## Validation Expectations

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router docs`
- `git diff --check`
