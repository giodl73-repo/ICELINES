---
wave: guard-the-operations
date_open: 2026-05-15
status: closed
source: post-Compose surface-parity partials and operational UX/admin gaps
---

# Guard the Operations

## Mission

Close the remaining explicit operational and product-UX partials after the MDI
workbench waves. IceLines now has a composable shell; this wave makes the
mutable support surfaces equally truthful: config/report toggles, admin data and
snapshot operations, watch-rule editing, favorites/groups management, and the
docs/parity ledger that tells users what is safe, persistent, deferred, or
handoff-only.

## Award Fit

This is a defensive wave. A good operations surface prevents own-goals: accidental
GET mutations, destructive admin actions without confirmation, stale runtime-only
config claims, and context panes that imply unsupported edits. The work is about
protecting users and preserving trust while still making the system easier to
operate.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Config/report persistence | Decide and implement the next safe step for web admin config/report toggles. | Add hidden global state or make runtime-only settings look durable. |
| Admin data operations | Resolve web data install/remove partials with safety contracts, confirmations, or explicit durable deferrals. | Trigger live/network downloads in tests or make destructive operations casual. |
| Watch-rule editing | Expand watch-rule editing where ViewModels and mutation intents already exist. | Rebuild the entire query/filter editor. |
| Favorites/groups | Close obvious management gaps in favorites/group UX without inventing new identity models. | Replace `GroupDb` or change persisted schema without a migration. |
| Surface truth | Keep README, COMMANDS, and `surface-parity.md` honest for each partial that moves. | Rewrite historical plans for style. |

## Operating Rules

- Pane/workspace selection remains GET/read navigation only.
- Favorites, watch rules, config, snapshots, and data operations remain
  POST-backed mutations.
- Web routes are one-shot request handlers; no hidden long-lived dashboard
  session state.
- Tests use bundled data, tempdirs, or fixtures. No live network tests.
- If an operation is too dangerous or underspecified, document the deferral
  instead of shipping a soft-confirmed mutation.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Operations parity inventory and pulse map | complete | `OPERATIONS-PARITY-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Persistent config/report toggle contract | complete | `icelines-web/src/handlers/admin.rs`; `icelines-web/src/config.rs`; `icelines-web/tests/l1_router.rs`; `plans/pulse-02.md` |
| 03 - Admin data operation safety | complete | `ADMIN-DATA-SAFETY.md`; `icelines-web/src/handlers/admin.rs`; `icelines-web/tests/l1_router.rs`; `plans/pulse-03.md` |
| 04 - Watch-rule editor parity | complete | `WATCH-RULE-EDITOR-SAFETY.md`; `icelines-cli/src/tui/command.rs`; `icelines-web/src/dashboard_command.rs`; `icelines-web/src/handlers/dashboard.rs`; `icelines-web/src/handlers/poach.rs`; `icelines-web/templates/watchlist.html`; `icelines-web/tests/l1_router.rs`; `plans/pulse-04.md` |
| 05 - Favorites/groups parity | complete | `FAVORITES-GROUPS-SAFETY.md`; `icelines-cli/src/tui/command.rs`; `icelines-web/src/handlers/favorites.rs`; `icelines-web/src/dashboard_command.rs`; `icelines-web/tests/l1_router.rs`; `plans/pulse-05.md` |
| 06 - Docs, regression gates, and closeout | complete | `README.md`; `COMMANDS.md`; `design/specs/surface-parity.md`; `plans/pulse-06.md` |

## Role Notes

- **keel**: operational state must stay on the correct surface and converge
  through shared ViewModels/mutation intents.
- **wire**: every mutation boundary must distinguish read state from write state
  and make dangerous/deferred operations explicit.
- **bench**: each moved partial needs a regression test at the right tier.
- **forge**: keep crate boundaries clean; `icelines-core` remains pure and web
  handlers should reuse existing intent/result types.
- **glass**: users must be able to tell whether a control changes state, opens a
  canonical route, or is intentionally unavailable.

## Current Result

Guard the Operations is closed. Config/report persistence, admin data-operation
safety, watch-rule editing, and favorites/groups management now have truthful
shared-surface behavior or durable deferral notes. Runtime-only web config,
deferred web report toggles, deferred data install/remove, deferred arbitrary
watch-rule dimensions, and deferred web group mutations are all documented
without implying unsupported persistence or GET-backed writes.

## Next

Choose the next wave from the phase inventory.
