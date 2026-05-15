---
wave: call-the-changes
date_open: 2026-05-15
status: active
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
| TUI navigation | Replace default MDI tab strip/cycling with a workbench shell: activity/catalog selection, center active screen, context panes, top live ribbon, and bottom command/status surface. | Delete `--classic` before compatibility is reviewed. |
| Web navigation | Add the same workbench catalog/zone model to `/dashboard` so users can open workspace panels without command syntax. | Convert the server-rendered dashboard into a SPA. |
| Docs and accessibility | Update keybinds, dashboard docs, and keyboard/ARIA expectations for direct screen selection. | Hide command bars; commands remain power-user shortcuts. |

## Workbench Zone Model

| Zone | Purpose | Candidate contents | Must not contain |
|---|---|---|---|
| Activity/catalog rail | Primary navigation and mode discovery. | Stats, Goalies, Scores, Schedule, Transactions, Playoffs, Favorites, Fantasy, Poach, Records, Reports, Docs. | Data tables, mutable forms, or hidden command-only actions. |
| Center workspace | The active document/screen. This is the only zone that should feel like the main task. | Leaders, player card, team depth, scoring reports, fantasy gaps, poach board, schedule detail, game detail. | Persistent global chrome or background live feeds. |
| Left context pane | User-owned context and shortcuts. | Favorites, watchlist, saved queries, groups, recent players/teams. | League-wide data that changes the task focus unexpectedly. |
| Right context pane | Active-screen secondary context. | Upcoming schedule, selected team/player context, related routes, source warnings, drilldown links. | Primary workflows that need full keyboard focus. |
| Top live ribbon | Always-glanceable live state. | Tonight scores, active season/type, selected date, sync/source status. | Deep analysis tables or long scrolling content. |
| Bottom command/status | Power-user command bar, parse/status feedback, and transient messages. | MDI commands, slash commands, errors, hints. | The only path to open a main screen. |
| Overlays | Temporary pickers and modal tasks. | Screen picker, season/date picker, docs/help, reports config, admin. | Long-lived workspace state. |

## Context Pane Option Bank

Pulse 01 should inventory these as workbench dimensions, not commit all of them
to Pulse 03/04. Pane content must be backed by existing ViewModels or route
summaries; no pane gets its own hockey math.

| Pane dimension | Candidate ViewModels | Useful as filters / pivots |
|---|---|---|
| Favorites and watch context | `FavoritesView`, `WatchlistView`, `WatchRulesView`, `WatchAlertsView` | favorite group, watch status, alert type, player/team entity, recent command target |
| Live slate and date context | `ScoresView`, `ScoresDayView`, `ScheduleView`, `ScheduleMatchupView` | date, team, home/away, game state, opponent, back-to-back/rest context |
| Team and roster context | `TeamDepthView`, `TeamDepthChartView`, `DepthLeagueView`, `TeamSeasonView` | team, position, line/pair, strength, home/away split, recent form |
| Player identity and career context | `PlayerCardView`, `CareerView`, `PlayerAwardsView`, `PlayerRecordsView` | player, season, league, award, record category, draft class, nationality |
| Query dimensions | `LeadersView`, `ReportView`, `DocsView`, `ConfigView` | stat catalog key, saved query, report type, source/report toggle, docs topic |
| Goalie dimensions | `GoaliesView`, `GameGoalieRow` | role, starts, save percentage, GAA, shutouts, nationality, team |
| Scoring intelligence | `GameScoringReportView`, `TeamScoringProfileView`, `PlayerScoringProfileView`, `TonightScoringIntelView`, `PlayerScoringPaceView`, `TeamScoringOutlookView` | goals, shots, attempts, inside-shot bucket, trend window, source state, projected finish availability |
| Fantasy and poach context | `FantasyLeagueView`, `FantasyRosterGapView`, `FantasySimulationView`, `PoachBoardView`, `PoachReportView` | league, team, category, availability, position, add/drop scenario, watch candidate |
| Game/playoff context | `GameView`, `PlayoffsView`, `RecordsOpponentRow`, `TeamRecordsView` | game id, series, round, opponent, home/away, record metric |
| Data/admin context | `DataStatusView`, `SnapshotView`, `MutationResultView` | data kind, source state, season/type, stale/missing status, last mutation result |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - MDI workbench inventory and zone contract | planned | `plans/pulse-01.md` |
| 02 - Shared workbench catalog and zone model | planned | depends on Pulse 01 |
| 03 - TUI full-MDI workbench shell | planned | depends on Pulse 02 |
| 04 - Web dashboard workbench catalog | planned | depends on Pulse 02 |
| 05 - Docs, regression gates, and closeout | planned | depends on Pulses 03-04 |

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
  entries are complete, picker selection changes active workspace, pane filters
  only use ViewModel-backed dimensions, and no command examples drift from
  catalog targets.

## Current Result

Wave opened after Sim the Spark closed and the release binary built. No code has
started. Pulse 01 should inventory the current TUI and web MDI shells, define the
workbench zone contract, decide the screen catalog shape, and split
implementation into safe slices.

## Next

Execute Pulse 01: MDI workbench inventory and zone contract.
