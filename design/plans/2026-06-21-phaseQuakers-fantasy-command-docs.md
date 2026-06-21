# Phase Quakers - Fantasy command docs truth gate

> Phase Quakers aligns dashboard fantasy mutation command docs with the
> read-only GET command contract.

**Created:** 2026-06-21
**Status:** Closed - Phase Quakers complete

---

## Frame

Dashboard command parsing rejects fantasy roster-shape mutation and CSV import
because dashboard command execution is GET/navigation oriented. The parser copy
names the read-only GET boundary and points to CLI mutation flows. `COMMANDS.md`
still used generic "deferred" wording, so `/docs` needed a small truth fence.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Quakers Goal 1 - Command reference copy** | Command docs should name the GET boundary. | Fantasy roster-shape set and import examples say not GET-backed and point to CLI commands. |
| 2 | **Quakers Goal 2 - Docs route fence** | Embedded docs should catch stale recovery text. | `/docs` test asserts fantasy mutation recovery copy. |
| 3 | **Quakers Goal 3 - Closeout** | The cleanup needs phase state. | Wave packet and indexes record the closed docs truth gate. |

---

## Non-goals

- Do not add browser fantasy CSV import.
- Do not add browser roster-shape preset mutation.
- Do not make dashboard command text mutate fantasy state through GET.

---

## Closeout

Phase Quakers is closed. The command reference now describes fantasy
roster-shape set and fantasy import examples as not GET-backed, with recovery to
the canonical CLI mutation commands.

---

## Validation Expectations

- `cargo fmt --check`
- `cargo test -p icelines-web --test l1_router docs`
- `git diff --check`
