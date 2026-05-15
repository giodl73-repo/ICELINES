---
wave: call-the-changes
date_open: 2026-05-15
status: closed
source: user request for final MDI stage after Sim the Spark closeout
---

# Call the Changes

## Mission

Finish IceLines MDI by making the app behave like a true workbench, not a tab
strip with a command bar. The TUI and web dashboard should expose explicit
screen selection, durable context panes, and clear placement rules for what
belongs in the center workspace, side panes, top live ribbon, and bottom command
surface.

## Award Fit

This continues the Jack Adams / Masterton product arc: a coach's bench does not
cycle blindly through lines; it calls the right change at the right time. IceLines
already has MDI shells, command bars, standalone launchers, and panel-ready web
routes. This wave turns that foundation into a VS Code-style workbench for
hockey: the catalog chooses the active tool, the center pane hosts the active
screen, and supporting panes keep live/contextual information visible without
forcing tab cycling.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Workbench zone contract | Define which experiences belong in the activity/catalog rail, center workspace, left pane, right pane, top ribbon, bottom command/status area, and overlays. | Let each surface invent its own placement rules. |
| Screen catalog contract | Shared list of main screen entries, labels, aliases, help text, zone defaults, and surface route targets. | New analytics or data loading behavior. |
| Pane model contract | Treat pane content as typed models over shared fields: navigators, inspectors, filters/dimensions, summaries, timelines, comparisons, queues, source-state panels, and action/status panels. | Limit panes to one-off side filters or surface-local widgets. |
| Bound experience tabs | Allow tabs as named compositions of center workspace, pane bindings, ribbon scope, and active fields that can be swapped together. | Recreate the old screen-cycling tab strip as the primary navigation model. |
| TUI navigation | Replace default MDI tab strip/cycling with a workbench shell: activity/catalog selection, center active screen, context panes, top live ribbon, and bottom command/status surface. | Delete `--classic` before compatibility is reviewed. |
| Web navigation | Add the same workbench catalog/zone model to `/dashboard` so users can open workspace panels without command syntax. | Convert the server-rendered dashboard into a SPA. |
| Docs and accessibility | Update keybinds, dashboard docs, and keyboard/ARIA expectations for direct screen selection. | Hide command bars; commands remain power-user shortcuts. |

## Workbench Zone Model

| Zone | Purpose | Candidate contents | Must not contain |
|---|---|---|---|
| Activity/catalog rail | Primary navigation and mode discovery. | Stats, Goalies, Scores, Schedule, Transactions, Playoffs, Favorites, Fantasy, Poach, Records, Reports, Docs. | Data tables, mutable forms, or hidden command-only actions. |
| Center workspace | The active document/screen. This is the only zone that should feel like the main task. | Leaders, player card, team depth, scoring reports, fantasy gaps, poach board, schedule detail, game detail. | Persistent global chrome or background live feeds. |
| Experience tabs | Optional bound layouts/presets above or near the center workspace. | Tonight bench, scoring room, team room, fantasy room, admin room. | The complete screen catalog or a hidden replacement for the activity rail. |
| Left context pane | User-owned context and shortcuts. | Favorites, watchlist, saved queries, groups, recent players/teams, pinned reports, queues. | League-wide data that changes the task focus unexpectedly. |
| Right context pane | Active-screen secondary context. | Upcoming schedule, selected team/player context, related routes, source warnings, drilldown links, summaries, comparisons, timelines. | Primary workflows that need full keyboard focus. |
| Top live ribbon | Always-glanceable live state. | Tonight scores, active season/type, selected date, sync/source status. | Deep analysis tables or long scrolling content. |
| Bottom command/status | Power-user command bar, parse/status feedback, and transient messages. | MDI commands, slash commands, errors, hints. | The only path to open a main screen. |
| Overlays | Temporary pickers and modal tasks. | Screen picker, season/date picker, docs/help, reports config, admin. | Long-lived workspace state. |

## Pane Model Option Bank

Pulse 01 should inventory these as workbench fields and pane models, not commit
all of them to Pulse 03/04. Pane content must be backed by existing ViewModels
or route summaries; no pane gets its own hockey math.

| Pane model | Purpose | Examples |
|---|---|---|
| Navigator | Move between entities, routes, screens, reports, or saved work. | Favorites, recent players, saved queries, catalog search, docs topics. |
| Inspector | Explain the selected entity/workspace without taking over the center. | Player/team/game summary, source warnings, related routes. |
| Filter/dimension | Narrow or pivot the center workspace through shared fields. | Team, position, date, game state, source state, stat key. |
| Summary/KPI | Show compact aggregates for the active workspace or selection. | Goal pace, shot trend, fantasy category gap, watch alert count. |
| Timeline/activity | Show ordered events or recent changes. | Transactions, schedule, game events, mutation results, sync events. |
| Compare | Hold another entity or baseline next to the center workspace. | Player vs peer, team vs opponent, favorite vs league rank. |
| Queue/checklist | Track actionable but explicit user/admin work. | Cache gaps, admin load tasks, watch rules to review. |
| Source/data state | Make loaded/missing/stale coverage visible. | Play-by-play loaded state, snapshot freshness, report toggles. |
| Action/status | Surface existing POST-backed actions and their results. | Favorite add/remove result, watch rule mutation result, cache load result. |

| Pane field set | Candidate ViewModels | Useful as pane models / fields |
|---|---|---|
| Favorites and watch context | `FavoritesView`, `WatchlistView`, `WatchRulesView`, `WatchAlertsView` | navigator, queue, summary: favorite group, watch status, alert type, player/team entity, recent command target |
| Live slate and date context | `ScoresView`, `ScoresDayView`, `ScheduleView`, `ScheduleMatchupView` | timeline, inspector, filter: date, team, home/away, game state, opponent, back-to-back/rest context |
| Team and roster context | `TeamDepthView`, `TeamDepthChartView`, `DepthLeagueView`, `TeamSeasonView` | inspector, compare, filter: team, position, line/pair, strength, home/away split, recent form |
| Player identity and career context | `PlayerCardView`, `CareerView`, `PlayerAwardsView`, `PlayerRecordsView` | inspector, summary, compare: player, season, league, award, record category, draft class, nationality |
| Query and docs context | `LeadersView`, `ReportView`, `DocsView`, `ConfigView` | filter, help, navigator: stat catalog key, saved query, report type, source/report toggle, docs topic |
| Goalie context | `GoaliesView`, `GameGoalieRow` | inspector, filter, summary: role, starts, save percentage, GAA, shutouts, nationality, team |
| Scoring intelligence | `GameScoringReportView`, `TeamScoringProfileView`, `PlayerScoringProfileView`, `TonightScoringIntelView`, `PlayerScoringPaceView`, `TeamScoringOutlookView` | summary, compare, source-state: goals, shots, attempts, inside-shot bucket, trend window, source state, projected finish availability |
| Fantasy and poach context | `FantasyLeagueView`, `FantasyRosterGapView`, `FantasySimulationView`, `PoachBoardView`, `PoachReportView` | navigator, filter, compare, action/status: league, team, category, availability, position, add/drop scenario, watch candidate |
| Game/playoff context | `GameView`, `PlayoffsView`, `RecordsOpponentRow`, `TeamRecordsView` | timeline, inspector, compare: game id, series, round, opponent, home/away, record metric |
| Data/admin context | `DataStatusView`, `SnapshotView`, `MutationResultView` | source-state, queue, action/status: data kind, source state, season/type, stale/missing status, last mutation result |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - MDI workbench inventory and zone contract | complete | `MDI-WORKBENCH-INVENTORY.md`; `plans/pulse-01.md`; `panels/pulse-01-workbench-contract/` |
| 02 - Shared workbench catalog, fields, and pane models | complete | `icelines-core/src/workbench.rs`; `icelines-cli/src/tui/workbench.rs`; `icelines-web/src/workbench.rs`; `plans/pulse-02.md` |
| 03 - TUI full-MDI workbench shell | complete | `icelines-cli/src/tui/mdi.rs`; `icelines-cli/src/tui/screens/mod.rs`; `icelines-cli/src/tui/app.rs`; `plans/pulse-03.md` |
| 04 - Web dashboard workbench catalog | complete | `icelines-web/src/handlers/dashboard.rs`; `icelines-web/templates/dashboard.html`; `icelines-web/templates/dashboard_workspace.html`; `plans/pulse-04.md` |
| 05 - Docs, regression gates, and closeout | complete | `README.md`; `COMMANDS.md`; `design/specs/surface-parity.md`; `plans/pulse-05.md` |

## Role Notes

- **keel**: TUI and web must converge on the same screen catalog identity and
  zone-placement rules. The command grammar can remain a shortcut, but it must
  not be the only source of workspace truth.
- **glass**: default MDI should be glanceable. A user should see how to open
  Stats, Goalies, Scores, Schedule, Transactions, Playoffs, Favorites, Fantasy,
  and team/player drilldowns without memorizing verbs, and should understand
  what each workbench zone is for.
- **forge**: keep catalog types small and shared through existing crate
  boundaries. Do not introduce route-local clones or long-lived web state.
- **wire**: screen selection is navigation only. GET routes may load panels from
  existing cache/read paths, never trigger mutations or live fetch side effects.
- **bench**: add tests that prove tab-era regressions are intentional, catalog
  entries are complete, picker selection changes active workspace, pane fields
  only use ViewModel-backed sources, and no command examples drift from catalog
  targets.

## Current Result

Call the Changes is closed. IceLines now has one shared workbench model across
TUI and web: catalog identity, zone placement, pane models, fields, bound
experiences, default TUI activity-rail navigation, and a server-rendered web
dashboard catalog with no-JS workspace summaries. README, COMMANDS, and the
surface-parity matrix document the default MDI behavior, Tab focus traversal,
bound web experience tabs, pane/field affordances, and POST-backed mutation
boundary.

## Closeout Gates

- `cargo fmt --check`
- `cargo test -p icelines-core --quiet`
- `cargo test -p icelines-cli --quiet`
- `cargo test -p icelines-web --quiet`
- `cargo clippy -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-call-the-changes README.md COMMANDS.md design\specs\surface-parity.md --errors-only`
- `cargo build --release -p icelines-cli`

## Next

Open the next wave from `design/waves/PHASES.md` when a new product direction is
ready.
