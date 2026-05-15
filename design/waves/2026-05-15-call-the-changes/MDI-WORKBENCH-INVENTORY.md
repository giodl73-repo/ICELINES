# Pulse 01 - MDI Workbench Inventory and Zone Contract

## Purpose

Call the Changes turns the existing Jack Adams MDI shells into a true workbench.
The goal is not just a screen picker. It is a zone contract: what belongs in the
center workspace, what can live in left/right context panes, what stays in the
top live ribbon, what belongs in the bottom command/status surface, and which
ViewModel-backed fields can be exposed through navigators, inspectors, filters,
summaries, timelines, comparisons, queues, source-state panes, and action/status
panes.

This pulse is documentation only. No code changes are authorized here.

## Current TUI state

| Area | Current implementation | Finding |
|---|---|---|
| Workspace identity | `App::screen: Screen` is the MDI workspace discriminator; `MdiLayout` deliberately avoids a parallel workspace enum. | Good foundation. Keep a single identity path and layer catalog metadata over it. |
| Main screens | `Screen` includes Home, Depth, Queries, Goalies, Favorites, Poach, Tonight, Schedule, Transactions, Playoffs, plus player/team/game/details and fantasy screens. | Catalog must distinguish main workbench entries from drilldown/document entries. |
| MDI layout | `render_mdi` renders top scores ribbon, body split, active workspace, per-screen keybind row, verb cheat sheet, and command bar. | The zones exist, but activity/catalog navigation is missing. |
| Side panes | `MdiLayout` supports only `Favorites` and `Schedule` side panes, with `Ctrl+H` / `Ctrl+L` and slash commands. | Need typed pane slots and a broader pane catalog instead of two hardcoded booleans. |
| Tab behavior | In MDI mode, Tab is a no-op reserved for command-bar autocomplete; classic mode still cycles tabs. | Default MDI can repurpose Tab for focus movement across zones. `--classic` keeps tab cycling. |
| Direct selection | Numeric `Action::GoToTab` maps ten screens: Home, Depth, Queries, Goalies, Favorites, Poach, Tonight, Schedule, Transactions, Playoffs. | Useful legacy seed, but a hidden ordered list is not enough for a workbench. |
| Command grammar | `tui::command::Command` covers workspace swaps, filters, fantasy actions, records, reports, and pane hide/show. | Commands should remain shortcuts, not the only discoverable navigation path. |
| Chrome/help | `ScreenChrome` still lists global `Tab = next tab`; MDI help is command-bar centered. | Docs/chrome must change when default MDI is no longer tab-first. |

## Current web state

| Area | Current implementation | Finding |
|---|---|---|
| Dashboard route | `GET /dashboard` renders a no-JS shell; `?partial=workspace` returns only the workspace fragment. | Good foundation. Keep canonical route identity. |
| Workspace identity | `/dashboard?workspace=<internal-route>` is canonical; side-pane visibility is browser-local state. | Keep this invariant. Do not encode pane state or command history in the URL. |
| Activity nav | `dashboard.html` has a hardcoded horizontal `jaw-workbench-nav` with Leaders, Goalies, Depth, Scores, Team season, Poach, Fantasy, Docs. | This is an embryonic catalog, not yet complete, searchable, grouped, or shared with TUI. |
| Left pane | Favorites and Watchlist list from dashboard rows. | Good default left context, but it should become one selectable pane dimension. |
| Right pane | Static schedule links. | Good default right context, but active-screen inspectors/source warnings should also fit here. |
| Workspace summaries | `handlers::dashboard::workspace_summary` already summarizes leaders, goalies, depth, poach, fantasy, scores, schedule, player, game, transactions, playoffs, favorites, watchlist, career, team season, and team depth. | Strong source for center/right-pane summary cards. |
| Command parser | `dashboard_command.rs` maps deterministic commands to internal routes or POST-backed mutation intents; panes are only Favorites/Schedule. | Needs a shared catalog target map or parity fixture so web and TUI do not drift. |
| JS behavior | `dashboard.js` progressively loads workspace fragments, updates history, stores side-pane preference, and handles command submission. | Keep progressive enhancement; add catalog/pane behavior without a SPA rewrite. |

## Workbench zone contract

| Zone | Contract | Default content | Expansion rule |
|---|---|---|---|
| Activity catalog rail | Primary mode discovery and active-workspace selection. It must be visible in default MDI at usable widths and reachable by keyboard. | Grouped screen catalog: League, Stats, Goalies, Scores, Schedule, Transactions, Playoffs, Favorites, Fantasy, Poach, Records, Reports, Docs, Admin. | Entries come from the shared workbench catalog; no surface-local hidden screen list. |
| Center workspace | The active document/screen. This is the main task surface and owns deep keyboard focus. | Current `App::screen` in TUI; current `/dashboard?workspace=` route in web. | Only one primary workspace is active at a time in this wave. Multi-document tabs are a future wave. |
| Left context pane | User-owned context, shortcuts, and durable personal state. | Favorites + Watchlist. | May switch to groups, saved queries, fantasy roster, recent entities, pinned reports, queues, or activity timelines. |
| Right context pane | Active-workspace inspector and secondary context. | Schedule/upcoming games. | May switch to filters, source warnings, selected entity details, summaries, comparisons, related routes, scoring context, or data state. |
| Top live ribbon | Always-glanceable live/current state. It should not steal focus. | Scores ribbon plus active season/type/date/source status. | May add sync/freshness badges, selected date, and warning count. |
| Bottom command/status | Power-user command bar, status, errors, and transient hints. | Existing TUI/web command bar. | Commands remain shortcuts. This zone must never be the only way to open a main screen. |
| Overlays | Temporary pickers/config/docs/admin panels. | Screen picker, help/docs, season/date picker, reports overlay, admin. | No long-lived pane state. Close returns focus to the prior zone. |

## Tabs as bound experiences

Tabs are allowed in the final web and TUI workbench, but they should no longer
mean "cycle through every screen." A tab should represent a bound experience: a
named composition of center workspace, left pane model, right pane model, top
ribbon scope, bottom command/status scope, and active fields.

Candidate shape:

```text
WorkbenchExperience {
  id: WorkbenchExperienceId,
  label: &'static str,
  center: WorkbenchId,
  left_pane: Option<WorkbenchPaneModelId>,
  right_pane: Option<WorkbenchPaneModelId>,
  ribbon_scope: WorkbenchRibbonScope,
  fields: &'static [WorkbenchFieldId],
}
```

Examples:

| Experience tab | Center | Left pane | Right pane | Active fields |
|---|---|---|---|---|
| Tonight bench | Scores | Favorites navigator | Schedule inspector | date, favorite group, game state, source state |
| Scoring room | Stats | Saved queries | Stat/filter inspector | stat key, report source, team, position |
| Team room | Depth or Team | Recent teams | Team inspector | team, position, line/pair, opponent |
| Fantasy room | Fantasy gaps | Fantasy roster | Poach filters | league, category, availability, position |
| Admin room | Admin | Queue/checklist | Source/data state | data kind, stale/missing, mutation result |

Swapping tabs should swap these bindings together. Users can still change the
center workspace or panes inside a tab, but the tab is a recomposable workbench
layout/preset, not the screen catalog itself. In default TUI MDI, the `Tab` key
can remain zone focus movement; experience-tab cycling should use an explicit
binding if implemented. `--classic` remains the compatibility mode where
Tab/Shift+Tab cycle screens.

## Default workbench layout decision

### TUI default MDI

Use a VS Code-style layout adapted to terminal constraints:

```text
top:    live scores + active season/type/date + source warning count
body:   activity/catalog rail | left context pane | center workspace | right inspector
bottom: per-screen key hints + command/status bar
```

At narrower widths, collapse in this order:

1. Right inspector collapses first.
2. Left context pane collapses second.
3. Activity rail compacts to icons/slugs.
4. Under the existing MDI minimum width, fall back to classic single-document
   rendering for that frame.

Default key semantics:

- `Ctrl+B` toggles the activity/catalog rail.
- `Ctrl+H` keeps toggling the left context pane.
- `Ctrl+L` keeps toggling the right context pane.
- `Tab` / `Shift+Tab` move focus between visible zones in default MDI.
- `Enter` activates the selected catalog entry or focused row.
- `:` and `/` keep focusing the command bar.
- `--classic` preserves the older tab strip and Tab screen cycling.
- `--standalone` remains a focused single-workspace mode with no workbench zones.

### Web dashboard

Promote the current horizontal workspace nav into a workbench activity catalog
that works without JavaScript and progressively enhances with filtering/grouping.

Keep these invariants:

- The center workspace remains `/dashboard?workspace=<route>`.
- Full routes remain canonical and usable without the dashboard.
- Side-pane state remains local browser state.
- Command input/history remain local/session-only state.
- Mutations remain POST-backed and never become GET navigation.

## Shared field and pane model

The workbench should treat every reusable pane input as a field, not as a
surface-local widget property. A field can be a true filter dimension, a summary
value, a route/entity identity, a source-state flag, a comparison baseline, or an
action/status value. This lets panes summarize active fields, expose available
pivots, and move between pane models without duplicating hockey logic.

Candidate core shape:

```text
WorkbenchField {
  id: WorkbenchFieldId,
  label: &'static str,
  scope: Entity | Workspace | Route | Source | Mutation | System,
  value_kind: Bool | Integer | Decimal | Text | Enum | Date | EntityRef | Route,
  source: ViewModel | RouteSummary | Catalog | CommandResult,
  operators: &'static [Equals | Range | In | Search | Sort | Group | Pin],
  summary: None | Count | MinMax | Latest | Status | Sparkline,
}

WorkbenchPaneModel {
  id: WorkbenchPaneModelId,
  label: &'static str,
  kind: Navigator | Inspector | Filter | Summary | Timeline | Compare | Queue |
        SourceState | ActionStatus | Help,
  supported_zones: &'static [Left | Right | Overlay | Bottom],
  fields: &'static [WorkbenchFieldId],
}
```

The first implementation does not need every operator or pane kind. It should
define the vocabulary now so Pulse 03/04 can implement a small safe subset while
future panes remain compatible.

Pane model meanings:

| Pane model | Purpose | Examples |
|---|---|---|
| Navigator | Move between workspaces, entities, reports, docs, or saved work. | Activity catalog, favorites, saved queries, recent players/teams/games. |
| Inspector | Explain the selected workspace/entity without replacing the center. | Player summary, game source state, related scoring/report links. |
| Filter/dimension | Narrow, group, sort, or pivot the active workspace using shared fields. | Team, position, date, game state, stat key, source state. |
| Summary/KPI | Compact aggregate or trend cards for the active workspace. | Goal pace, shot trend, fantasy gaps, watch alert count. |
| Timeline/activity | Ordered activity stream tied to the active scope. | Transactions, schedule, game events, sync events, mutation results. |
| Compare | Hold another entity, route, or baseline next to the center. | Player vs peer, team vs opponent, favorite vs league leader. |
| Queue/checklist | Actionable work that is explicit and user/admin initiated. | Cache gaps, watch rules to review, reports to generate. |
| Source/data state | Loaded/missing/stale coverage and config state. | Play-by-play availability, snapshot freshness, report toggles. |
| Action/status | Existing mutation intents plus their results. | Favorite/watch mutations, admin load results, config saves. |
| Help/docs | Contextual docs and command examples. | Screen help, field explanations, route docs. |

## Shared catalog shape

Pulse 02 should add a typed catalog identity that both TUI and web adapters use.
Keep route/screen execution in the owning surface, but share the stable IDs,
labels, grouping, default zone, aliases, pane models, and pane fields.

Candidate core shape:

```text
WorkbenchEntry {
  id: WorkbenchId,
  label: &'static str,
  group: WorkbenchGroup,
  aliases: &'static [&'static str],
  default_zone: WorkbenchZone,
  document_kind: Main | Drilldown | Context | Admin | Docs,
  pane_models: &'static [WorkbenchPaneModelId],
  fields: &'static [WorkbenchFieldId],
}
```

Surface adapters then map:

- TUI: `WorkbenchId -> Screen` or resolver requiring an argument.
- Web: `WorkbenchId -> canonical route` or resolver requiring route params.
- Commands: aliases resolve through the same IDs before lowering to TUI screen or
  web route.

## Candidate catalog entries

| ID | Group | Label | TUI target | Web target | Default zone | Context panes |
|---|---|---|---|---|---|---|
| `league` | League | League | `Screen::Home` | `/` or `/leaders` | center | standings/depth summary, source warnings |
| `stats` | Analytics | Stats | `Screen::Queries` | `/leaders` | center | stat catalog, saved queries, active filters |
| `goalies` | Analytics | Goalies | `Screen::Goalies` | `/goalies` | center | goalie filters, team/nationality pivots |
| `depth` | Teams | Depth | `Screen::Depth` | `/depth` | center | team filter, position filter, line/pair context |
| `team` | Teams | Team | `Screen::Team(abbrev)` | `/team/:abbrev` | center | roster filters, schedule, records, scoring links |
| `player` | Players | Player | `Screen::PlayerById(pid)` | `/player/:id` | center | career, awards, records, scoring, favorites |
| `scores` | Live | Scores | `Screen::Tonight` | `/scores` | center/top | date picker, game state, game detail links |
| `schedule` | Live | Schedule | `Screen::Schedule` | `/schedule` | center/right | team, date, opponent, home/away |
| `transactions` | Live | Transactions | `Screen::Transactions` | `/transactions` | center/right | team, kind, search |
| `playoffs` | Live | Playoffs | `Screen::Playoffs` | `/playoffs` | center | round, series, game links |
| `game` | Live | Game | `Screen::GameDetail(id)` | `/game/:id` | center | goals, goalies, scoring report, source state |
| `favorites` | My bench | Favorites | `Screen::Favorites` | `/favorites` | left/center | group, entity kind, stat line, source state |
| `watchlist` | My bench | Watchlist | command/workspace handoff | `/watchlist` | left/center | alert type, rule status, player/team |
| `groups` | My bench | Groups | `Screen::Groups` | no full route yet | left/context | group name, entity kind |
| `fantasy` | Fantasy | Roster gaps | `Screen::FantasyGaps` | `/fantasy` | center | categories, position, roster need |
| `simulate` | Fantasy | Simulation | `Screen::FantasySim` | `/fantasy?...` | center | add/drop, weeks, scenario warnings |
| `poach` | Fantasy | Poach | `Screen::Poach` | `/poach` | center | availability, position, categories, watch status |
| `reports` | Reports | Reports | command handoff | `/reports/poach`, `/reports/weekly` | center | report type, categories, availability |
| `records` | Reports | Records | player/team command handoff | `/records/player/:id`, `/records/team/:abbr` | center/right | metric, opponent, game/source state |
| `career` | Players | Career cohorts | command handoff | `/career` | center/right | league, tier, season, sort |
| `docs` | System | Docs | docs overlay | `/docs` | overlay/center | topic, command examples |
| `admin` | System | Admin | admin overlay | `/admin` | overlay/center | data kind, snapshot, config, mutation result |

## Context pane option bank

These pane options are intentionally broad. Pulse 03/04 should implement a small
safe subset first, but the catalog should leave room for these pane models and
fields.

| Pane option | Best zone | Pane model | Existing ViewModels | Fields / pivots / summaries |
|---|---|---|---|---|
| Favorites navigator | left | Navigator + Summary | `FavoritesView`, `FavoriteMemberRow` | group, entity kind, player/team, stat-line source state, member count |
| Watchlist alerts | left | Navigator + Queue | `WatchlistView`, `WatchRulesView`, `WatchAlertsView` | alert type, enabled/disabled, availability trigger, entity, unresolved count |
| Groups | left | Navigator | `FavoritesView`-adjacent group rows | group name, entity kind, recent additions |
| Saved queries | left | Navigator + Filter | query saved-list state, `LeadersView` | query name, stat key, filter text, last-used |
| Recent entities | left | Navigator | screen history derived from workspace transitions | player, team, game, report, route |
| Fantasy roster | left | Navigator + Summary | `FantasyLeagueView`, `FantasyRosterGapView` | active league/team, scoring category, weak category, gap count |
| Activity catalog filters | left or overlay | Navigator + Filter | workbench catalog | group, search text, favorite/pinned entries |
| Schedule inspector | right | Inspector + Timeline + Filter | `ScheduleView`, `ScheduleTeamView`, `ScheduleMatchupView` | date, team, opponent, home/away, game state, next game |
| Selected player inspector | right | Inspector + Summary | `PlayerCardView`, `CareerView`, `PlayerAwardsView` | position, team, season, league, award, records, favorite state |
| Selected team inspector | right | Inspector + Compare | `TeamDepthView`, `TeamSeasonView`, `TeamPlayerStreaksView` | position, line/pair, split, recent form, streak metric, opponent |
| Stat catalog/filter inspector | right | Filter + Help | `LeadersView`, `ReportView`, stat catalog rows | stat key, category, unit, source report, sort direction |
| Goalie inspector | right | Inspector + Filter | `GoaliesView`, `GameGoalieRow` | role, starts, save percentage, GAA, shutouts, nationality |
| Game inspector | right | Inspector + Timeline | `GameView`, `GameScoringReportView` | period, situation, goal scorer, goalie, shot type/source state |
| Scoring trend inspector | right | Summary + Compare | `PlayerScoringProfileView`, `TeamScoringProfileView`, `TonightScoringIntelView` | window, goals, shots, attempts, inside bucket, source state |
| Outlook inspector | right | Summary | `PlayerScoringPaceView`, `TeamScoringOutlookView` | pace row, below-threshold, projected-finish availability |
| Poach filters | right | Filter + Queue | `PoachBoardView`, `PoachReportView` | availability, position, category, candidate kind, watch status |
| Fantasy simulation inspector | right | Compare + Action/status | `FantasySimulationView` | add, drop, weeks, category deltas, scenario warning |
| Records inspector | right | Inspector + Summary | `PlayerRecordsView`, `TeamRecordsView` | metric, opponent, source coverage, entity |
| Career cohort inspector | right | Filter + Compare | `CareerView` | league, tier, season, sort stat, player |
| Data/source inspector | right | Source/data state + Queue | `DataStatusView`, `SnapshotView`, `ConfigView`, `MutationResultView` | data kind, stale/missing, active season/type, last mutation |
| Docs/help inspector | overlay/right | Help | `DocsView` | topic, command verb, screen ID, field explanation |

## Compatibility rules

1. Default `icelines tui` stays MDI, but becomes workbench-first instead of
   command-first.
2. Tabs, when present, are bound workbench experiences/presets. They are not the
   primary screen list.
3. `--classic` is the only mode where Tab/Shift+Tab mean screen cycling.
4. `--standalone` remains one-screen focus mode and should not grow side panes.
5. Existing command grammar remains supported and should lower through catalog
   IDs where possible.
6. Web `/dashboard` remains server-rendered and no-JS useful.
7. Full web routes remain canonical; dashboard workspace panels are wrappers.
8. Pane models can only expose fields already present in ViewModels, route
   summaries, catalog metadata, or command/mutation results. No pane-local hockey
   math.
9. GET navigation must not mutate favorites, watch rules, caches, snapshots, or
   config.

## Pulse split after inventory

| Pulse | Scope | Notes |
|---|---|---|
| 02 - Shared workbench catalog, fields, and pane models | Add typed IDs, groups, zones, aliases, pane models, shared fields, and adapter target maps. | Prefer pure catalog types in `icelines-core` plus TUI/web adapters for concrete targets. |
| 03 - TUI full-MDI workbench shell | Add activity rail/catalog, zone focus, context-pane selection model, and MDI Tab focus semantics. | Preserve `--classic` tab cycling and `--standalone` behavior. |
| 04 - Web dashboard workbench catalog | Promote hardcoded nav to grouped catalog, add pane picker/filter affordances, and align URL/local-state rules. | Progressive enhancement only; no SPA rewrite. |
| 05 - Docs, regression gates, and closeout | Update README, COMMANDS, surface parity, help/chrome docs, run gates, close wave. | Include release smoke or web capture gate if layout changed materially. |

## Required tests

### Pulse 02

- Catalog IDs are unique.
- Aliases are unique or intentionally shared through a documented preferred ID.
- Every main TUI workspace has a catalog entry.
- Every dashboard-ready web workspace has a catalog entry or an explicit
  deferred reason.
- Pane fields name an existing ViewModel-backed, route-summary, catalog, or
  command-result source.
- Pane models include at least navigator, inspector, filter, summary, timeline,
  source-state, and action/status variants, with zone compatibility tests.
- Bound experience tabs compose center workspace, pane models, ribbon scope, and
  active fields without replacing the shared screen catalog.
- TUI and web command examples resolve through catalog IDs or parity fixtures.

### Pulse 03

- Default MDI renders the activity/catalog rail at wide widths.
- Selecting a catalog entry changes `App::screen` and resets selection safely.
- Tab/Shift+Tab move focus across workbench zones in MDI.
- If experience tabs are rendered, swapping them updates the center/pane/field
  bindings together and does not revert to legacy screen cycling.
- `--classic` still cycles screens with Tab/Shift+Tab.
- `--standalone` remains locked to one screen.
- Left/right pane toggles preserve existing `Ctrl+H` and `Ctrl+L` behavior.
- Narrow widths collapse inspector/context panes before falling back to SDI.

### Pulse 04

- `/dashboard` renders catalog landmarks without JavaScript.
- Catalog links preserve `/dashboard?workspace=...` canonical state.
- Workspace partial fetch still returns only the workspace fragment.
- Side-pane/catalog state does not enter the URL.
- Pane filter controls use GET for read-only workspace filters and POST for
  existing mutation intents only.
- Accessibility tests cover labels, `aria-expanded`, focus target, and no
  color-only status.

### Pulse 05

- `COMMANDS.md`, README, and `surface-parity.md` describe the workbench model.
- TUI `?` help and chrome text no longer advertise Tab as default MDI screen
  cycling.
- Release gates and CI pass.

## Role review findings

- **KEEL**: one catalog identity must be shared between TUI and web; surface
  adapters may own concrete `Screen` and route lowering, but they cannot invent
  independent screen lists.
- **GLASS**: a true MDI app needs visible zones and screen discovery. The catalog
  rail plus center/left/right/top/bottom model is the minimum readable shape.
- **FORGE**: keep shared catalog types small and pure. Do not push web route
  handlers or TUI `Screen` into `icelines-core`; use adapters for surface
  targets.
- **WIRE**: pane and catalog actions are navigation/read filters unless they
  explicitly delegate to existing POST-backed mutation intents.
- **BENCH**: tests must prove the intentional Tab behavior split: MDI uses Tab
  for focus movement; classic uses Tab for screen cycling; standalone ignores
  cross-screen movement.
