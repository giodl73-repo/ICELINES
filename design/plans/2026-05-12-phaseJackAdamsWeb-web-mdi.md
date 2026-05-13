# Phase Jack Adams Web - browser dashboard and command surface

**Date**: 2026-05-12
**Status**: Draft
**Trophy lineage**: Jack Adams concept bridge. The original Jack Adams phase made
the TUI feel like a coordinated bench: MDI dashboard, stable side panes,
workspace swapping, and a command bar. This phase brings that product model to
the browser while respecting the web surface's own constraints.
**Depends on**: Ted Lindsay route parity, Campbell ViewModels, Prince of Wales
visual tokens, Selke fantasy surfaces.

---

## Why

The web surface is route-complete and increasingly coherent, but it still feels
like a set of pages. The TUI Jack Adams dashboard solved a bigger product
problem: give the user a hockey front door where live context, saved interests,
navigation, and analysis all coexist.

The web should carry the same concepts:

- a persistent scores ribbon;
- Favorites/Watchlist as durable side context;
- a central workspace that can host any product surface;
- a command palette that understands the same verbs as the TUI command bar;
- responsive collapse rules that preserve the workspace on small screens;
- shared ViewModels, not browser-only scoring/filter logic.

This is not a SPA rewrite for its own sake. It is a browser shell around the
existing route and ViewModel contracts.

---

## Product Shape

### Desktop

```text
┌──────────────────────────────────────────────────────────────┐
│ Scores ribbon: live/final/scheduled games + active context    │
├──────────────┬───────────────────────────────┬───────────────┤
│ Favorites    │ Workspace                     │ Schedule      │
│ Watchlist    │ leaders/player/team/poach/... │ Today/week    │
│ quick picks  │                               │ upcoming      │
├──────────────┴───────────────────────────────┴───────────────┤
│ Command palette / status / shortcuts                          │
└──────────────────────────────────────────────────────────────┘
```

### Mobile

Mobile is not a squeezed three-column dashboard. It becomes:

- scores ribbon at top;
- workspace as the main page;
- bottom command/search affordance;
- Favorites, Schedule, and Watchlist as drawer tabs;
- command palette as a full-screen overlay.

---

## Locked Decisions

| Decision | Choice |
|---|---|
| Architecture | Progressive enhancement over server-rendered routes. Keep normal URLs working without JavaScript. |
| Workspace identity | URLs remain canonical (`/leaders`, `/player/:id`, `/poach`, etc.). The dashboard shell can load them as panels, but deep links render the same content. |
| Panel mechanics | Server-rendered fragments are the default panel unit: full routes render full pages, and `?partial=workspace` or explicit fragment siblings can render only workspace-safe markup. JSON is for APIs and parity checks, not the primary browser rendering path. |
| Dashboard URL state | Canonical workspace lives in `/dashboard?workspace=<url-encoded-internal-path>`. Side-pane visibility is local/session state. Command input/history is never persisted in the URL. |
| Command grammar | Reuse the TUI command vocabulary where possible: `stats`, `goalies`, `poach`, `gaps`, `simulate`, `team EDM`, `player Bedard`, `box EDM@BOS`, `favorites`, `/fav add`, `/hide schedule`, `/show favorites`. |
| Command execution | Deterministic parser first. Optional AI interpretation remains off by default and must validate into the same command schema. |
| Command contract ownership | Prefer extracting the deterministic command grammar into a shared parser used by TUI and web adapters. If extraction is too expensive in the first slice, add parity fixtures so web and TUI examples cannot drift silently. |
| Side panes | Favorites/Watchlist and Schedule are context panes, not independent app modes. They can be hidden, collapsed, or opened as drawers. |
| Data source | Panels consume existing HTML fragments or JSON/ViewModel endpoints. No duplicated scoring/projection logic in JavaScript. |
| No-JS baseline | Every product route remains usable as a full server-rendered page. |
| Mutation boundary | Web commands that mutate state must submit through existing POST handlers or shared mutation intents. No GET mutations, no external navigation, and no browser-only mutation logic. |
| Styling | Use Prince semantic tokens and existing route layout classes; no separate one-off dashboard palette. |

---

## Sub-Phase Order

```text
JAW.1  Shell contract and route inventory
JAW.2  Server-rendered dashboard shell
JAW.3  Command palette parser and routing
JAW.4  Panel loading, workspace history, and side panes
JAW.5  Fantasy/poach scenario actions in the shell
JAW.6  Responsive/mobile drawers and accessibility
JAW.7  Tests, docs, closeout
JAW.8  Optional AI command interpretation
```

---

## JAW.1 - Shell Contract

Define the web dashboard shell contract before implementation.

Deliverables:

- `WebDashboardView` or equivalent shell ViewModel carrying:
  - active season/type label;
  - scores ribbon rows;
  - favorites rows;
  - watchlist alerts/rules summary;
  - schedule summary rows;
  - workspace route metadata;
  - source warnings.
- A route capability table mapping command verbs to existing routes and
  endpoints.
- A URL/state invariant note:
  - `/dashboard?workspace=/poach?availability=imported-available` identifies
    the workspace;
  - side-pane hide/show lives in local/session state;
  - command input/history is never serialized into the URL.
- A panel contract for every workspace route:
  - full-page URL;
  - server-rendered HTML fragment URL or `?partial=workspace` behavior;
  - JSON endpoint;
  - command verbs that can open it;
  - required query params.
- A mutation-safety table mapping web commands to existing POST routes or
  mutation intents.

Acceptance:

- No route loses its full-page rendering.
- `surface-parity.md` names which web routes are dashboard-panel-ready.
- Tests fence the shell ViewModel's active context and source-state fields.
- Tests prove dashboard URL state keeps workspace canonical and does not encode
  side-pane or command-input state.

---

## JAW.2 - Server-Rendered Dashboard Shell

Status: Implemented in the initial Jack Adams Web slices. `/dashboard` now
renders a no-JS shell with scores, Favorites/Watchlist, workspace, schedule,
command form, allowlisted workspace URL state, and a shared
`?partial=workspace` fragment.

Add a new first-class dashboard route, probably `/dashboard`, then decide later
whether `/` should redirect or render the shell by default.

Deliverables:

- `/dashboard` HTML shell with:
  - scores ribbon;
  - Favorites/Watchlist pane;
  - central workspace defaulting to leaders/home preview;
  - Schedule pane;
  - command/status bar.
- Keep `/` as the current low-risk home until the shell is proven.
- Use existing templates/partials where practical.

Acceptance:

- `/dashboard` works without JavaScript.
- With JavaScript disabled, `/dashboard` still renders useful scores,
  favorites/watchlist, workspace, schedule, and normal links.
- Shell renders active context, empty states, and source warnings.
- Route inventory test includes `/dashboard`.
- HTML tests assert all landmark regions exist:
  - `header`/scores;
  - `aside` favorites/watchlist;
  - `main` workspace;
  - `aside` schedule;
  - command palette trigger.

---

## JAW.3 - Command Palette

Status: Partially implemented. The web has a deterministic command parser,
`COMMANDS.md` parity examples, and `POST /dashboard/command`. Read commands
redirect to allowlisted dashboard workspace URLs; favorite/watch mutations
delegate to existing POST handlers or mutation intents. Remaining work is the
full palette overlay, keyboard opener/history, and richer visible error/status
rendering.

Bring the TUI command bar idea to web as a command palette.

Deliverables:

- Shared command grammar module for web-compatible command parsing. Prefer
  extracting the TUI parser into a reusable crate/module if practical; otherwise
  mirror the grammar with tests proving parity.
- Palette opens with `/`, `Ctrl+K`, or a visible command button.
- Commands resolve to safe internal routes or form actions:
  - reads navigate/load workspace;
  - mutations use existing POST endpoints and CSRF/same-origin-safe forms if
    added later;
  - invalid commands render clear errors.
- Command history for the session.

Acceptance:

- Parser tests cover every supported verb.
- Web and TUI command examples in `COMMANDS.md` stay aligned.
- No command can navigate to an external URL.
- No mutation bypasses existing POST handlers or mutation intent contracts.
- Command grammar is either shared with the TUI parser or fenced by parity
  fixtures derived from the documented command examples.

---

## JAW.4 - Workspace Panels and Side Panes

Status: In progress. The first progressive enhancement is shipped:
dashboard workspace links and command redirects fetch `?partial=workspace`,
replace the central panel, and update browser history. Favorites/Schedule panes
have local toggle state that does not alter canonical URLs. The first
product-aware workspace summaries are also shipping: `/dashboard?workspace=/team/EDM/season`
projects a `TeamSeasonView` summary, and `/dashboard?workspace=/leaders` plus
`/dashboard?workspace=/goalies` project top rows from `HomeView`. Team depth
workspaces now summarize roster count, first line, first pair, goalies, and
extras from `TeamDepthView`, all while preserving canonical full route links.
Remaining work is panel-specific fragments/parity for more product routes and
richer side-pane row actions.

Make the shell feel like MDI without losing web semantics.

Deliverables:

- Workspace panel loader:
  - server-rendered initial panel;
  - progressive enhancement with `fetch`/HTMX-style fragment replacement;
  - browser history updates on workspace swaps.
- Side-pane controls:
  - hide/show Favorites;
  - hide/show Schedule;
  - compact Watchlist alerts;
  - selected Favorite opens player/team in workspace.
- Panel-ready routes start with:
  - leaders;
  - player;
  - team/depth;
  - goalies;
  - scores/game detail;
  - schedule;
  - fantasy gaps/simulation;
  - poach;
  - transactions.

Acceptance:

- Direct links and shell-loaded panels render equivalent data.
- Browser back/forward restores workspace route.
- Side-pane hide/show state is local and does not alter canonical URLs.
- Panel fragment tests compare visible row identity against the full route or
  JSON ViewModel endpoint.
- Tests cover panel parity for at least leaders, player, team, fantasy, poach.

---

## JAW.5 - Fantasy and Poach Actions

The dashboard becomes a player-poacher cockpit.

Deliverables:

- Commands:
  - `poach availability=imported-available`;
  - `gaps categories=goals,shots`;
  - `simulate add="Player" drop="Player" weeks=4`;
  - `/fav add "Player"`;
  - `watch "Player"`;
- Workspace cards expose add/drop/drop-only scenario actions using the existing
  `FantasySimulationView` and mutation/result contracts.
- Errors stay visible in the command/status bar and in the workspace.

Acceptance:

- Scenario resolution remains canonical-name based.
- Invalid drops render the same error contract as full-page routes.
- No fantasy scoring/projection logic is introduced in JS.
- All mutations are POST-backed and resolve through existing intent/result
  ViewModels.

---

## JAW.6 - Responsive, Accessibility, and Visual Polish

Deliverables:

- Desktop, tablet, and mobile layouts with explicit breakpoints:
  - wide: scores + two side panes + workspace;
  - medium: schedule collapses first;
  - narrow: both side panes become drawers;
  - mobile: command palette full-screen.
- Keyboard accessibility:
  - command button focusable;
  - palette traps focus while open;
  - Escape closes overlays;
  - landmarks and labels are explicit.
- Visual pass against Prince tokens and CREST/aesthetic role.

Acceptance:

- Playwright/screenshot checks for wide, medium, mobile.
- No text overlap in header, panes, command palette, or fantasy cards.
- No color-only status meaning.

---

## JAW.7 - Tests, Docs, Closeout

Tests:

- Rust route tests for `/dashboard` and panel endpoints.
- Template tests for shell regions and active context.
- Static asset tests for dashboard JS/CSS if added.
- Parser parity tests against TUI command examples.
- No-JS tests for `/dashboard` useful fallback content.
- Mutation-safety tests proving command actions do not use GET and do not emit
  external URLs.
- Playwright/smoke screenshots for responsive states.

Docs:

- `COMMANDS.md` web dashboard section.
- `README.md` quickstart mentions `/dashboard`.
- `surface-parity.md` adds dashboard shell/panel readiness columns or notes.
- `phase-jack-adams-overview.md` gets an addendum pointing to this web bridge
  instead of implying the original TUI phase should own web.

Closeout:

- Decide whether `/` becomes the dashboard shell.
- Record remaining deferred items explicitly.
- Commit, tag if this is a release phase.

---

## JAW.8 - Optional AI Command Interpretation

This mirrors Jack Adams AI fallback, but should ship last and remain config-gated.

Rules:

- deterministic parser always runs first;
- LLM output must validate into the command schema;
- no arbitrary JS, shell, or URL execution;
- API keys stay in environment variables;
- failures show the deterministic parse error, not hallucinated suggestions.

Acceptance:

- Mock-provider tests only in CI.
- No external LLM calls in automated gates.

---

## Risks and Guardrails

| Risk | Guardrail |
|---|---|
| SPA rewrite gravity | Keep full-page routes canonical and usable. |
| JS duplicates hockey logic | JS only routes, loads fragments, and submits existing intents. |
| Command parser drifts from TUI | Shared parser or parity fixtures from `COMMANDS.md`. |
| Shell makes pages slower | Initial shell uses server-rendered summaries; panel loads are incremental. |
| Mobile becomes cramped | Side panes become drawers; workspace remains primary. |
| Mutation safety gets blurry | Existing mutation intent ViewModels remain the only mutation boundary. |

---

## First Implementation Slice

1. Add `/dashboard` shell route with server-rendered scores/favorites/schedule
   summaries and a workspace placeholder.
2. Add tests for shell landmarks, active context, and route inventory.
3. Add the command-palette grammar spec and parser parity fixtures, but do not
   wire mutations yet.
4. Make leaders or home preview the first real workspace panel.

This gives us visible product movement without committing to a full SPA or
breaking existing web routes.
