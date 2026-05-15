---
wave: compose-the-bench
date_open: 2026-05-15
status: closed
source: user selected next wave after Call the Changes closeout
---

# Compose the Bench

## Mission

Turn the shared workbench catalog into a composable pane system. IceLines should
let users choose not only the center workspace, but also the surrounding context:
left/right pane models, active dimensions/fields, top-ribbon scope, and bound
experiences that can be swapped as coherent layouts across TUI and web.

## Award Fit

This continues the Jack Adams workbench arc. A coach does not only call which
line jumps over the boards; they compose the bench around the moment: matchups,
specialists, scouts, and live context. This wave makes that composition explicit
and shared instead of letting each surface hardcode Favorites-left and
Schedule-right forever.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Pane composition contract | Define typed bindings between center workspaces, side pane models, active fields, top-ribbon scope, and status/action zones. | Add new hockey math or surface-local scoring logic. |
| Field/dimension summary | Treat shared workbench fields as the language for filters, dimensions, summaries, and source-state affordances. | Build a full query-builder rewrite. |
| Bound experiences | Promote tabs/presets into named compositions that can swap workspace + panes together. | Recreate legacy tab cycling as the primary navigation. |
| TUI pane controls | Let default MDI users move between zones and change pane composition without command syntax. | Remove `--classic` or `--standalone`. |
| Web pane controls | Let `/dashboard` expose pane composition server-side with safe URL/read state. | Convert the dashboard to a SPA or make pane changes mutate user data. |
| Docs and release gates | Document pane composition and close with proof/build gates. | Hide command bars; commands remain shortcuts. |

## Composition Model

| Layer | Purpose | Examples |
|---|---|---|
| Center workspace | Primary task/document. | Leaders, Goalies, Team, Player, Scores, Schedule, Poach, Fantasy. |
| Left pane binding | User-owned navigation and pinned context. | Favorites navigator, Watchlist queue, Groups navigator, Saved queries. |
| Right pane binding | Active-workspace secondary context. | Schedule inspector, Player inspector, Team inspector, Source/data state. |
| Active field set | Shared dimensions the current composition exposes. | team, player, game, date, position, category, availability, sort, source-state. |
| Top ribbon scope | Glanceable live/source state. | scores, active date, season/type, sync/source warnings. |
| Bottom status/actions | Command/status feedback and explicit existing actions. | command parse result, favorite/watch mutation result, cache-load result. |
| Bound experience | A named preset over all of the above. | Tonight bench, Team room, Player lab, Fantasy room, Admin room. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Pane composition inventory and contract | complete | `PANE-COMPOSITION-INVENTORY.md`; `plans/pulse-01.md` |
| 02 - Shared pane binding contract | complete | `icelines-core/src/workbench.rs`; `icelines-core/src/lib.rs`; `icelines-cli/src/tui/workbench.rs`; `icelines-web/src/workbench.rs`; `plans/pulse-02.md` |
| 03 - TUI pane composition controls | complete | `icelines-cli/src/tui/mdi.rs`; `icelines-cli/src/tui/app.rs`; `icelines-cli/src/tui/screens/mod.rs`; `plans/pulse-03.md` |
| 04 - Web pane composition controls | complete | `icelines-web/src/handlers/dashboard.rs`; `icelines-web/templates/dashboard.html`; `icelines-web/static/style.css`; `icelines-web/static/dashboard.js`; `plans/pulse-04.md` |
| 05 - Docs, regression gates, and closeout | complete | `README.md`; `COMMANDS.md`; `design/specs/surface-parity.md`; `plans/pulse-05.md` |

## Role Notes

- **keel**: pane composition must be one shared system. TUI and web can lower
  differently, but the identity of a pane model, field, and bound experience
  must come from `icelines-core::workbench`.
- **glass**: users need to understand what is center work versus context at a
  glance. Pane controls must not overload the screen or bury the center task.
- **forge**: prefer small typed IDs, arrays, and adapter functions over stringly
  route-local state. Keep `icelines-core` pure.
- **wire**: pane and experience selection is navigation/read state only. Any
  favorite/watch/admin mutation remains POST-backed through existing intents.
- **bench**: every pane binding rule needs tests: catalog completeness, surface
  support, focus traversal, no-JS web rendering, URL allowlisting, and docs
  parity.

## Current Result

Compose the Bench is closed. IceLines now has a shared pane-composition contract
with typed pane bindings, field sets, pane models, and bound experiences in
`icelines-core::workbench`; TUI users can apply bound experiences and cycle
focused side-pane bindings; web dashboard users can use allowlisted
`left`/`right`/`experience` URL state and no-JS pane selector chips. README,
COMMANDS, and the surface-parity matrix document the system and preserve the
GET-read versus POST-mutation boundary.

## Next

Choose the next wave from the phase inventory.
