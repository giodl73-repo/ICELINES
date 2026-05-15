# Pane Composition Inventory

## Purpose

Compose the Bench starts from the closed Call the Changes workbench. The shared
catalog now exists, the TUI has an activity rail, and `/dashboard` renders the
same catalog plus bound experience tabs. This inventory defines the next
contract: a user should be able to compose the center workspace with left/right
pane models, active fields, top-ribbon scope, and bottom status/actions without
each surface inventing its own pane language.

## Governing findings

| Role | Finding |
|---|---|
| keel | Pane identity must remain in `icelines-core::workbench`; TUI and web only lower shared metadata. |
| glass | The center workspace remains the main task. Pane controls must be visible and reversible, not another hidden command grammar. |
| forge | The implementation should extend typed static metadata and adapters, not add stringly pane names in renderers/templates. |
| wire | Pane/experience changes are read/navigation state. Favorite, watch, admin, cache, and data mutations stay POST-backed. |
| bench | Core table integrity, TUI focus/application behavior, and web no-JS/allowlist behavior all need explicit tests. |

## Current shared contract

`icelines-core/src/workbench.rs` already has the raw ingredients:

| Item | Current shape | Gap for this wave |
|---|---|---|
| Workbench entries | `WORKBENCH_CATALOG` maps workspace IDs to labels, aliases, default zone, document kind, candidate pane models, and fields. | Entries do not say which pane model should occupy left/right for a specific composition. |
| Fields | `WORKBENCH_FIELDS` names shared dimensions like team, player, game, date, category, availability, sort, source state, data kind, and mutation result. | Fields are displayed as affordances but are not yet grouped as reusable active field sets. |
| Pane models | `WORKBENCH_PANE_MODELS` names navigators, inspectors, filters, summaries, timelines, queues, source-state panes, action/status panes, and help panes. | Pane models have supported zones but no binding ID, priority, surface support, or concrete left/right selection rules. |
| Bound experiences | `WORKBENCH_EXPERIENCES` has center, optional left/right pane IDs, ribbon scope, and fields for Tonight bench, Scoring room, Team room, Fantasy room, and Admin room. | TUI ignores them; web renders them as tabs but only routes to the center workspace and keeps fixed side panes. |
| Lookups | `workbench_entry`, `workbench_field`, `workbench_pane_model`, and `workbench_experience` provide typed lookup seams. | Tests should lock referential integrity before more metadata is added. |

The existing model is close enough that Pulse 02 should evolve it instead of
creating a parallel `PaneComposition` universe. The missing piece is an explicit
binding layer that says "this left pane slot uses this pane model for these
surfaces and fields" and "this experience applies this center + panes + ribbon
as a single composition."

## Current TUI state

Default MDI currently lives in:

- `icelines-cli/src/tui/mdi.rs`
- `icelines-cli/src/tui/screens/mod.rs`
- `icelines-cli/src/tui/app.rs`
- `icelines-cli/src/tui/workbench.rs`

Current behavior:

| Area | Current behavior | Composition gap |
|---|---|---|
| Focus zones | `MdiFocus` covers activity rail, left pane, workspace, and right pane. `Tab` / `Shift+Tab` traverse visible zones. | Focus is ready for pane controls, but left/right focused zones do not yet have pane-selection behavior. |
| Catalog | Activity rail uses `WORKBENCH_CATALOG`; Up/Down selects, Enter opens no-argument TUI workspaces. | Catalog selection does not apply bound experiences or pane defaults. |
| Side pane IDs | `MdiLayout` stores `left_pane_model` and `right_pane_model`, defaulting to Favorites navigator and Schedule inspector. | IDs only change by code defaults; no picker/cycler and no validation against supported zones. |
| Side pane content | Left renderer always calls Favorites; right renderer always calls Schedule. Titles use shared pane labels. | Changing the ID today would change the title but not the concrete content. Pulse 03 must either constrain selectable IDs to implemented renderers or add safe stubs. |
| Top ribbon | Scores ribbon always renders as the top live strip. | Ribbon scope from `WorkbenchExperience` is not applied. Pulse 03 can start with labels/status only and avoid new data logic. |
| Bottom status | Command bar and flash status exist. | No bottom action/status pane binding yet; mutation results stay in existing command/status paths. |
| Compatibility | `--classic` keeps legacy tab cycling; `--standalone` locks one screen. | Pane controls must only apply when `app.mdi.is_some()`. |

### TUI-ready pane models

These are safe candidates for visible TUI composition because a matching screen
or renderer exists today:

| Pane model | Zone | Backing renderer/path | Pulse 03 action |
|---|---|---|---|
| Favorites navigator | left | `favorites::render` | Keep as primary left default and selectable. |
| Schedule inspector | right | `schedule::render` | Keep as primary right default and selectable. |
| Activity catalog | rail/overlay | `render_mdi_activity_catalog` | Already implemented; do not make it a side pane. |
| Docs/help | right/overlay | `Screen::Help` / docs overlay | Can be listed as future/right stub until a compact help renderer exists. |
| Data/source inspector | right/overlay | `Screen::Fetch` / admin overlay | Can be listed as future/right stub until compact source renderer exists. |

Pane models like Player inspector, Team inspector, Game inspector, Scoring
trend, Outlook summary, Records inspector, Career cohort, Poach filters, and
Fantasy simulation are valid shared metadata, but they need compact pane
renderers or safe textual stubs before becoming selectable TUI panes. This is a
Pulse 03 stop condition: do not expose a selectable TUI pane whose title changes
while content remains misleading.

## Current web dashboard state

The browser workbench currently lives in:

- `icelines-web/src/workbench.rs`
- `icelines-web/src/handlers/dashboard.rs`
- `icelines-web/src/templates.rs`
- `icelines-web/templates/dashboard.html`
- `icelines-web/templates/dashboard_workspace.html`
- `icelines-web/static/dashboard.js`
- `icelines-web/static/style.css`

Current behavior:

| Area | Current behavior | Composition gap |
|---|---|---|
| Catalog rail | Server renders grouped shared catalog entries through `dashboard_ready_workbenches`. | Good foundation; no change needed except ensuring pane query state preserves workspace links. |
| Experience tabs | Server renders `WORKBENCH_EXPERIENCES` as tabs. Each tab links to the center route and shows left + right labels in detail text. | Applying a tab does not change left/right panes; active state is inferred only from center workspace. |
| Left/right panes | Template always renders Favorites/Watchlist left and Schedule right; rows are backed by `FavoritesView`, `WatchlistView`, and schedule links. | Pane model rows are fixed to Favorites navigator and Schedule inspector. No server-side pane selector or URL state exists. |
| Workspace fragment | Fragment shows active pane models and fields from the active `WorkbenchEntry`. | Good no-JS affordance, but not the actual left/right composition. |
| JS | Progressive workspace replacement preserves workspace and local side-pane visibility. | JS has only show/hide local state; no pane/experience composition state. |
| Mutations | `/dashboard/command` delegates favorite/watch mutations to POST-backed existing handlers. | Must remain unchanged; pane selection must never target command or mutation endpoints. |

### Web-ready pane models

The web dashboard can safely render more pane choices than TUI if they are
display-only cards or route summaries, but Pulse 04 should still be conservative:

| Pane model | Zone | Backing data | Pulse 04 action |
|---|---|---|---|
| Favorites navigator | left | `FavoritesView` rows | Current default and selectable. |
| Watchlist queue | left/right | `WatchlistView` rows | Selectable once template can switch left pane body between favorites/watchlist emphasis. |
| Groups navigator | left | group/favorites routes | Selectable as links-only if no new data logic is added. |
| Schedule inspector | right | schedule links / `ScheduleView` summaries | Current default and selectable. |
| Data/source inspector | right | existing admin/data routes and source-state fields | Selectable as route-summary/help pane, no live fetch. |
| Docs/help | right | `/docs` route summary | Selectable as docs links/help text. |
| Poach filters | right | existing poach query params | Selectable only if it edits read-only query parameters. |
| Fantasy simulation | right/center | existing `/fantasy` query state | Selectable as summary/status, not as a mutation. |

Parameterized inspector panes for player/team/game can be shown only when the
workspace URL carries that entity. Otherwise the pane must render an empty state
like "Open a player workspace to inspect a player." Do not fake an entity or
derive data from the wrong route.

## ViewModel and field coverage

The shared fields in `WORKBENCH_FIELDS` are backed by existing ViewModels or
route summaries:

| Field family | Existing backing |
|---|---|
| Workspace/route | `WORKBENCH_CATALOG`, dashboard normalized routes, TUI `Screen`. |
| Player/team/game/entity | `PlayerCardView`, `TeamDepthView`, `TeamSeasonView`, `GameView`, favorites/watch rows. |
| Date/game state/opponent/home-away | `ScoresView`, `ScheduleView`, `GameView`. |
| Position/stat key/sort/report type | `LeadersView`, stat catalog, query/filter surfaces, reports. |
| League/category/availability | `CareerView`, `FantasyRosterGapView`, `PoachBoardView`, `PoachReportView`. |
| Source state/data kind/mutation result | admin/data views, scoring source-state summaries, existing `MutationResultView`. |

No new field is required for Pulse 02. If a later implementation needs a field
outside this list, stop and add it deliberately to `WorkbenchFieldId` with
backing evidence and tests rather than passing strings through UI state.

## Proposed binding taxonomy

Pulse 02 should add or refine typed metadata around these concepts:

| Concept | Purpose | Suggested shape |
|---|---|---|
| Pane slot | Distinguish left, right, overlay, top ribbon, bottom status. | Reuse `WorkbenchZone` where possible; add slot-specific helpers only if zone is too broad. |
| Pane binding | A selectable pane model in a slot with supported surfaces and optional fallback text. | Static struct with ID, pane model ID, slot, supported surfaces, fields, default priority. |
| Surface support | Avoid exposing a pane that has no renderer/route on a surface. | Enum/bitset for TUI and Web; core remains pure. |
| Field set | Reusable named list of active fields for a composition. | Could be embedded in experience first; extract only if duplication grows. |
| Experience application | Apply center workspace plus pane IDs plus ribbon/field scope as one preset. | Extend current `WorkbenchExperience`; adapters decide if center route/screen is available. |

Do not add persistence in this wave. Runtime selection can be in TUI `MdiLayout`
and web query/local state. Persisted workbench layouts are a separate future
wave because they require config/versioning decisions.

## Candidate bound experiences

| Experience | Center | Left | Right | Fields | Notes |
|---|---|---|---|---|---|
| Tonight bench | Scores | Favorites navigator | Schedule inspector | date, favorite group, game state, source state | Already in core; should become the default applied composition. |
| Scoring room | Stats | Saved queries | Stat/filter inspector | stat key, report type, team, position | TUI needs compact saved-query/filter pane before full selection. |
| Team room | Depth | Recent entities | Team inspector | team, position, opponent | Web can show route summaries first; TUI may start with title/stub. |
| Fantasy room | Fantasy | Fantasy roster | Poach filters | category, availability, position | Must remain read-only except existing POST-backed watch/favorite commands. |
| Admin room | Admin | Watchlist queue | Data/source inspector | data kind, source state, mutation result | Keep destructive/admin operations out of GET pane changes. |

The current five experiences are sufficient for this wave. Add new experiences
only if Pulse 01 findings prove a missing central workflow; otherwise avoid
scope creep.

## Placement rules

| Placement | Rule |
|---|---|
| Center workspace | Owns the primary scrollable task and keyboard-heavy table/card. Only one center workspace is active. |
| Left pane | User-owned navigation, pinned context, queues, or saved work. It should not change the center workspace unless the user activates a link/selection. |
| Right pane | Active-workspace context: inspectors, source state, related routes, summaries, or filters. |
| Top ribbon | Glanceable live/date/source state only. No long tables or forms. |
| Bottom status | Command input, parse feedback, flash messages, and results from existing explicit actions. |
| Overlay | Temporary pickers/help/admin tasks. Overlay state should not masquerade as a persistent pane. |

## TUI implementation plan

1. Pulse 02: expose typed pane binding/experience metadata through the TUI
   adapter and add integrity tests.
2. Pulse 03: store active experience plus left/right binding IDs in `MdiLayout`.
3. Pulse 03: add focus-safe controls:
   - `Tab` / `Shift+Tab` keeps zone traversal.
   - left pane focused: cycle/select left-supported pane bindings.
   - right pane focused: cycle/select right-supported pane bindings.
   - rail focused: Enter may apply an experience when the selected workspace has
     a no-argument TUI screen; otherwise keep the existing "needs argument"
     status.
4. Pulse 03: render active pane binding labels and fields in side-pane headers
   or compact header text.
5. Pulse 03: only render concrete pane bodies that have real TUI backing.
   Unsupported panes should be disabled or render explicit "not available in
   TUI yet" stubs; do not show Favorites content under a non-Favorites title.

## Web implementation plan

1. Pulse 02: expose shared pane binding/experience metadata through the web
   adapter, with route support and safe labels.
2. Pulse 04: add optional read-only query parameters for pane/experience state
   only if allowlisted, for example `left=` / `right=` / `experience=`.
3. Pulse 04: server-render left/right pane selector controls. No-JS users should
   see current selections and links to switch them.
4. Pulse 04: keep side-pane show/hide as local browser state; composition is
   read/navigation state.
5. Pulse 04: preserve `?partial=workspace` and canonical workspace route links.
6. Pulse 04: reject unsafe pane/experience query values the same way unsafe
   workspace paths are rejected.

## Stop conditions

- Stop if a desired pane needs a ViewModel field that is not represented by
  `WorkbenchFieldId`.
- Stop if a pane switch would trigger a favorite/watch/admin/cache mutation from
  a GET request.
- Stop if a TUI pane title can change while the body still renders unrelated
  content.
- Stop if web query state accepts arbitrary route/path/HTML values instead of a
  shared allowlisted ID.
- Stop if tests would need live network data.

## Pulse split confirmation

The existing split remains valid:

| Pulse | Decision |
|---|---|
| 02 | Extend shared workbench metadata and adapters before UI changes. |
| 03 | TUI pane controls after shared binding IDs exist. |
| 04 | Web pane controls after shared binding IDs exist. |
| 05 | Docs, proof, broad gates, release build, and closeout. |

No additional implementation pulse is required unless Pulse 02 discovers that
surface support metadata cannot be represented without a larger refactor.

## Test and docs matrix

| Area | Required tests/docs |
|---|---|
| Core metadata | L0 tests that every binding references existing pane models and fields; every experience center exists; surface support is explicit; action/status bindings are not GET targets. |
| TUI state | Tests for default active composition, left/right pane cycling, unsupported-pane handling, experience application, and `--classic` / `--standalone` compatibility. |
| TUI render | Tests that active pane labels match actual content or explicit unavailable stubs, and focus styling remains visible. |
| Web routing | Tests for safe `left` / `right` / `experience` query values, unsafe fallback/rejection, workspace normalization, and `partial=workspace` preservation. |
| Web templates | Tests for no-JS pane selectors, active experience state, active fields, and POST-backed mutation boundary text/links. |
| Docs | README and COMMANDS must explain pane composition controls; `surface-parity.md` must record shared workbench pane composition as a done or partial row at closeout. |
