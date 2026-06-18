# IceLines surface parity matrix

**Date**: 2026-05-09
**Status**: Draft - Campbell seed, Ted Lindsay owns web verification

This is the source-of-truth matrix for whether a platform feature is reachable
through each user surface and whether it renders from a shared engine/ViewModel
path.

Status values:

- `done` - implemented and verified against the running build
- `verify` - likely implemented, but needs Ted Lindsay/Jennings verification
- `partial` - useful behavior exists, with named gaps
- `planned` - planned in a forward phase
- `deferred` - intentionally out of scope for now
- `n/a` - surface does not apply

---

## Contract

Every row must eventually name:

- shared engine or ViewModel path
- CLI command
- TUI screen/action
- web HTML route
- web JSON route
- static site/export artifact when the feature is reportable or documented as
  generated output
- status and tests
- exceptions or deferred owner

No documentation page should advertise a feature as shipped unless this matrix
has a `done` or clearly qualified `partial` row for it.

Static site and export coverage may live in the row notes while the matrix is
still compact, but Ted Lindsay/Jim Gregory must make it explicit before calling
a feature fully shipped.

---

## Core analytical surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Leaders/skater leaderboard | `LeadersView` | `query leaders` | `tui stats` | `/leaders` | `/api/v1/leaders` | done for non-report surfaces - CLI text, CLI JSON/CSV, web HTML, web JSON, and TUI stats result rendering build `LeadersView`; TUI query-result rank/primary projection is fenced by `l0_tui_leaders_view_preserves_query_result_rank_and_primary_metric`; web template/JSON projection is fenced by `l0_web_leaders_view_round_trips_template_and_json_rows`; markdown custom columns use `LeaderRow.catalog_metrics` | Campbell/Ted Lindsay |
| League/home preview | `HomeView` | n/a | dashboard shell | `/` | n/a | done - web home preview skaters/goalies build from `HomeView` | Ted Lindsay |
| Goalie leaderboard | `GoaliesView` | `query goalies` | `tui goalies` | `/goalies` | `/api/v1/goalies` | done - CLI, TUI, web HTML, and web JSON build `GoaliesView`; CLI JSON/CSV row identity fenced by `l2_cmd_query_goalies_json_csv_row_identity_match`; web JSON supports CLI-parity `saves` sort and explicit `gp_min`; CLI/TUI/web known-sort ordering uses shared `GoalieLeaderboardSort` with player-id tie-break; CLI-vs-web row identity fenced by `l2_query_goalies_cli_and_web_row_identity_match` | Campbell/Ted Lindsay |
| Player card | `PlayerCardView` | `query player <name>` | `tui player <name>` | `/player/:id` | `/api/v1/player/:id` | done for card surfaces - web HTML and web JSON project from `PlayerCardView`; web JSON row identity is fenced by `l1_player_json_rows_match_player_card_view`; CLI profile header/current-season block and TUI header/headshot/current-season stat strip use `PlayerCardView`; TUI career table renders selected catalog columns from `PlayerCardView.career.catalog_metrics`; pre-NHL local-store rows project through `PlayerCardView.pre_nhl_career` | Campbell/Ted Lindsay |
| Team depth | `TeamDepthView` / `TeamDepthChartView` | `team <ABBR>` | `tui team <ABBR>` | `/team/:abbrev` | `/api/v1/team/:abbrev` | done - CLI team, markdown export, web HTML, and web JSON build `TeamDepthView`; CLI empty checks use `TeamDepthView::is_empty`; web JSON row identity is fenced by `l1_team_json_rows_match_team_depth_view`; scoring-mode TUI chart renders from `TeamDepthChartView` and is fenced by `l0_team_depth_chart_view_projects_tui_columns` and `l1_tui_depth_team_render_matches_team_depth_chart_view_first_player` | Campbell/Ted Lindsay |
| Team season performance | `TeamSeasonView` | `team-season <ABBR>`; `export md team-season --team <ABBR>` | `team <ABBR> season` | `/team/:abbrev/season` | `/api/v1/team/:abbrev/season` | done for current major surfaces - core, CLI text/JSON, TUI, web HTML, web JSON, and markdown/report export project `TeamSeasonView`; distinct from roster/depth and raw schedule list surfaces; shipped metrics include record, points, home/away splits, one-goal split, recent form, remaining schedule counts, next opponents, standings/playoff-cutline context, schedule-strength faced/remaining opponent Pts%, top/middle/bottom buckets, quality wins, expected wins, bad losses, missed points, and source warnings; richer historical standings/cache persistence remains future polish | Presidents Trophy |
| Team/player streak leaders | `TeamPlayerStreaksView` / `PlayerStreaksView` | n/a | n/a | `/team/:abbrev/streaks`, `/player/:id/streaks` | `/api/v1/team/:abbrev/streaks`, `/api/v1/player/:id/streaks` | web-only team/player season streak boards read cached per-game boxscore rows; missing cache state renders empty views with in-UI game-cache loading instead of CLI instructions and does not create local data cache directories on GET | Ted Lindsay |
| Scoring reports | `GameScoringReportView` / `TeamScoringProfileView` / `PlayerScoringProfileView` / `TonightScoringIntelView` | n/a | n/a | `/game/:id/scoring`, `/team/:abbrev/scoring`, `/player/:id/scoring`, `/tonight/intel` | `/api/v1/game/:id/scoring`, `/api/v1/team/:abbrev/scoring`, `/api/v1/player/:id/scoring`, `/api/v1/tonight/intel` | web/API Rocket Richard reports from cached official NHL play-by-play scoring events; summaries include goals, SOG, missed, blocked, attempts, unblocked attempts, period/situation splits, top shooter IDs, player scoring profiles, and favorites-first daily scoring-intel rows; empty states distinguish missing play-by-play from loaded zero-event payloads, selected scoring/outlook/tonight-intel GET paths do not create local data cache directories when cache state is missing, and team/tonight pages offer POST-backed cache loading | Rocket Richard/Ted Lindsay |
| Individual records | `PlayerRecordsView` / `TeamRecordsView` | `records player <name>`, `records team <ABBR>` | player/team hints plus cmdbar handoff | `/records/player/:id?metric=...`, `/records/team/:abbrev?metric=...` | `/api/v1/records/player/:id?metric=...`, `/api/v1/records/team/:abbrev?metric=...` | player metrics: teams-scored-against, goalies-scored-against, fight-opponents; team metrics: players-scored-against-team, goalies-beaten-by-team, fight-opponents-by-team; goalie/fight metrics use cached play-by-play and do not infer from aggregates | Ted Lindsay |
| Cross-team depth | `DepthLeagueView` | `export md depth` team-strength section | `tui depth` | `/depth` | `/api/v1/depth` | done - web HTML, web JSON, TUI league ranking, and markdown team-strength export build from `DepthLeagueView`; web JSON row identity is fenced by `l1_depth_json_rows_match_depth_league_view`; TUI first-row projection is fenced by `l1_tui_depth_league_render_matches_depth_league_view_first_row`; TUI depth Enter navigation and ranking render use `league_view_from_app`; markdown export keeps supplemental player line-value detail as an intentional extra table | Ted Lindsay |
| Compare/comps | `CompareView` / `SimilarPlayersView` | `query compare A B` / `query compare A --similar N` | `tui comps <name>` plus cmdbar head-to-head handoff | `/compare?...` | `/api/v1/compare?...` | done - web HTML and web JSON build `CompareView`; web JSON card identity is fenced by `l1_compare_json_cards_match_compare_view`; CLI head-to-head and TUI comps target card use `CompareView` card projection; CLI similarity, TUI comps list, `/api/v1/compare?a=ID&similar=N`, and `/compare?a=ID&similar=N` project from `SimilarPlayersView`; TUI target is fenced by `l1_tui_comps_target_matches_compare_view_anchor`; TUI cmdbar preserves head-to-head intent by handing two-player compares to canonical CLI/web surfaces; Jack Adams Web dashboard command parsing accepts the same natural `compare A vs B` and comma-delimited head-to-head grammar; web similarity HTML is fenced by `l1_compare_html_similarity_renders_similar_players_section` | Ted Lindsay |
| Career/cohort leaders | `CareerView` | `query career --league ...` | player/favorites affordances plus cmdbar handoff | `/career` | `/api/v1/career` | partial, intentionally handoff-only in TUI - CLI, templated web HTML, web JSON, and Jack Adams Web dashboard summaries build from `CareerView`; web HTML uses the shared page shell and is fenced by `l1_career_html_uses_shared_page_shell`; web JSON row identity is fenced by `l1_api_career_rows_match_career_view`; CLI adapter alignment is fenced by `l0_cli_career_rows_project_from_career_view`; TUI cmdbar parses `career` cohort options and points users to canonical one-shot CLI/web surfaces because a dedicated board would duplicate the local-store cohort table without adding fields beyond `CareerView`; cold installs return an explicit `icelines fetch career --bundled-seasons 5` instruction, fenced by `l1_career_missing_store_errors_name_fetch_command` | Calder/Ted Lindsay |
| Scouting report | `ReportView` | `scouting <name>` | player detail/report affordance | `/scouting/:id` | `/api/v1/scouting/:id` | done - CLI scouting and web/API scouting wrap player-card projection in the shared `ReportView` contract with stable scouting section refs | Campbell/Ted Lindsay |
| Player signals (descriptive) | `PlayerSignalsView` | `signals <name>` (text + `--json` `signals.v1` envelope) | planned | planned | planned | partial, CLI-only by design (Phase Hurricane / WP-010 pulse-03) - CLI text and `--json` `signals.v1` envelope render `PlayerSignalsView` (Physical Engagement Rate, Puck Management Differential, Penalty Drag Rate); missing/partial evidence renders `unavailable`/`null` with evidence tier + missing inputs, never zero-fill; non-claim + disclosure copy travel with both encodings; Signals stay out of `StatId`, leaderboards, and the `--filter` catalog until a later pulse; TUI/Web parity is pulse-04; L0 in `commands::signals`, L2 in `signals_system.rs` | Phase Hurricane/PACE |
| Coach cache dashboard | `AnalyticsCacheConsumerView` | n/a | n/a | `/coach/dashboard` (`GET /coach/dashboard`) | `/api/v1/coach/dashboard` (`GET /api/v1/coach/dashboard`) | partial - first coach-specific Web/API analytics cache surface defaults to the active `coach_dashboard:<season>:<type>` cache key and a coach metric family; existing records render through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Player evidence card | `AnalyticsCacheConsumerView` | n/a | n/a | `/player/evidence-card` (`GET /player/evidence-card`) | `/api/v1/player/evidence-card` (`GET /api/v1/player/evidence-card`) | partial - first player evidence-card Web/API analytics cache surface defaults to the active `player_evidence_card:<season>:<type>` cache key and a player metric family; existing records render as `PlayerEvidenceCard` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Line combination explorer | `AnalyticsCacheConsumerView` | n/a | n/a | `/lines/explorer` (`GET /lines/explorer`) | `/api/v1/lines/explorer` (`GET /api/v1/lines/explorer`) | partial - first line-combination Web/API analytics cache surface defaults to the active `line_combination_explorer:<season>:<type>` cache key and a line metric family; existing records render as `LineCombinationExplorer` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Goalie readiness workload view | `AnalyticsCacheConsumerView` | n/a | n/a | `/goalies/readiness` (`GET /goalies/readiness`) | `/api/v1/goalies/readiness` (`GET /api/v1/goalies/readiness`) | partial - first goalie-readiness Web/API analytics cache surface defaults to the active `goalie_readiness:<season>:<type>` cache key and a goalie metric family; existing records render as `GoalieReadiness` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, workload, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Practice focus report | `AnalyticsCacheConsumerView` | n/a | n/a | `/practice/focus` (`GET /practice/focus`) | `/api/v1/practice/focus` (`GET /api/v1/practice/focus`) | partial - first practice-focus Web/API analytics cache surface defaults to the active `practice_focus:<season>:<type>` cache key and a practice metric family; existing records render as `PracticeFocusReport` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, practice priorities, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Postgame review report | `AnalyticsCacheConsumerView` | n/a | n/a | `/postgame/review` (`GET /postgame/review`) | `/api/v1/postgame/review` (`GET /api/v1/postgame/review`) | partial - first postgame-review Web/API analytics cache surface defaults to the active `postgame_review:<season>:<type>` cache key and a postgame metric family; existing records render as `PostgameReviewReport` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, postgame causality, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Postgame adjustment review | `AnalyticsCacheConsumerView` | n/a | n/a | `/postgame/adjustments` (`GET /postgame/adjustments`) | `/api/v1/postgame/adjustments` (`GET /api/v1/postgame/adjustments`) | partial - second postgame Web/API analytics cache surface defaults to the active `postgame_adjustments:<season>:<type>` cache key and the postgame consumer family; existing records render as `PostgameReviewReport` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, causality, correction plans, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Agent evidence summary | `AnalyticsCacheConsumerView` | n/a | n/a | `/agents/evidence` (`GET /agents/evidence`) | `/api/v1/agents/evidence` (`GET /api/v1/agents/evidence`) | partial - first agent-evidence Web/API analytics cache surface defaults to the active `agent_evidence:<season>:<type>` cache key and the agent evidence consumer family; existing records render as `AgentEvidence` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, recommendations, actions, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Opponent scout cache report | `AnalyticsCacheConsumerView` | n/a | n/a | `/scout/opponent` (`GET /scout/opponent`) | `/api/v1/scout/opponent` (`GET /api/v1/scout/opponent`) | partial - first opponent-scout Web/API analytics cache surface defaults to the active `opponent_scout:<season>:<type>` cache key and a scout metric family; existing records render as `OpponentScoutReport` through the cache store and consumer ViewModel without recomputing source-state, confidence, quality, freshness, or methodology, and missing cache renders explicit unavailable state without creating cache directories | Ted Lindsay/Campbell |
| Markdown export | `ReportView` / `PlayoffsView` / `PoachReportView` / `TeamSeasonView` | `export md <shape>` | n/a | n/a | n/a | done - all 8 markdown shapes ship; `series` renders a playoff game-log from `PlayoffsView`, `fantasy` renders a poacher report from `PoachReportView`, and `team-season` renders a season-performance report from `TeamSeasonView` with source-state/warning disclosure | Campbell/Jim Gregory/Presidents Trophy |

---

## Live and schedule surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Scores/tonight | `ScoresView` / `TeamTradeImpactView` | `tonight`; `tonight trade` | `tui scores` | `/scores` | `/api/v1/scores` | done - CLI `tonight`, TUI scores, web HTML, and web JSON build `ScoresView`; CLI `tonight trade` renders before/after swap rows from `TeamTradeImpactView`; `ScoreGameRow` carries stable `game_id` and raw UTC start time so CLI/TUI navigation and ET display are preserved | Ted Lindsay/Campbell |
| Schedule | `ScheduleView` / `ScheduleTeamView` / `ScheduleMatchupView` | `schedule` | `tui schedule` | `/schedule` | `/api/v1/schedule` | done for non-fantasy surfaces - CLI schedule, TUI week list, web HTML, and web JSON build `ScheduleView`; `ScheduleGameRow` carries stable `game_id`, raw UTC start time, score/status fields, game type, and playoff context; TUI team-season record/list and head-to-head regular/playoff splits now project through schedule subviews | Lester Patrick/Ted Lindsay |
| Playoffs | `PlayoffsView` | `playoffs` | `tui playoffs` | `/playoffs` | `/api/v1/playoffs` | done - CLI playoffs, TUI bracket list, TUI series-detail header/summary/game log, web HTML, and web JSON project through `PlayoffsView`; `PlayoffsSeriesRow` carries stable letter, seed-rank, winner, games-played, and per-game rows for adapter output | Lester Patrick/Ted Lindsay |
| Transactions | `TransactionsView` | `transactions` | `tui transactions` | `/transactions` | `/api/v1/transactions` | done - CLI, TUI, web HTML, and web JSON row projection build from `TransactionsView`; shared contract handles the `LEAGUE` teamless bucket; CLI uses the unlimited constructor after applying explicit filters/top | Lester Patrick/Ted Lindsay |
| Game detail | `GameView` | n/a | `tui scores` drilldown | `/game/:id` | `/api/v1/game/:id` | done - web HTML, web JSON, and TUI drilldown goals/goalies/stat leaders build `GameView`; `GameGoalRow`, `GameGoalieRow`, and widened `GameSkaterRow` carry scoring, goalie, and boxscore leader context for adapter output | Ted Lindsay |

---

## User/fantasy/product surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Shared workbench shell | `WORKBENCH_CATALOG`, `WORKBENCH_PANE_BINDINGS`, `WORKBENCH_FIELDS`, `WORKBENCH_EXPERIENCES` | n/a | default `icelines tui` MDI workbench; `--classic` preserves legacy tab cycling; `--standalone` locks one workspace | `/dashboard` | n/a | done - TUI and web lower from `icelines-core::workbench` through surface adapters; default TUI exposes the activity/catalog rail, zone focus traversal, bound experience state, shared left/right pane binding IDs, focused side-pane cycling, truthful side-pane titles/bodies, and command-bar shortcut; web renders grouped catalog entries, bound experience tabs, allowlisted `left`/`right`/`experience` query state, pane selector chips, shared pane-model cards, active field cards, and no-mutation stubs server-side; dashboard GET navigation is read-only and write actions stay POST-backed | Jack Adams/Compose the Bench |
| Favorites/groups | `FavoritesView` / `MutationResultView` | `group ...` | favorites/group affordances; cmdbar `/fav add/remove` supports players and team abbrevs | `/favorites`, `/favorites?group=<name>` | `/api/v1/favorites`, `/api/v1/favorites?group=<name>`, `/api/v1/favorites/add`, `/api/v1/favorites/remove` | partial - web HTML, read JSON, and Jack Adams dashboard side panes project membership through `FavoritesView`; web `/favorites` can select any SQLite group in read-only mode while POST-backed add/remove remains scoped to canonical `Favorites`; favorite player `stat_line` rendering is cache-only and must not create manifest/boxscore state or call live NHL APIs on GET; web add/remove resolves through `FavoriteMutationIntent`; dashboard `favorites group=...` / `group show ...` opens the read-only group view and rejects group create/delete/rename/member edits instead of turning them into GET mutations; HTML routes redirect and JSON mutation twins return `MutationResultView`; arbitrary group create/rename/delete/member editing stays on CLI/TUI until a shared group mutation contract exists | Ted Lindsay/Guard the Operations |
| Fantasy league management / roster gaps / simulation | `FantasyLeagueView` / `FantasyRosterGapView` / `FantasySimulationView` / `MutationResultView` | `fantasy ...`, `fantasy gaps`, `fantasy simulate` | `fantasy gaps`, `fantasy simulate` screens | `/fantasy` | `/api/v1/fantasy/gaps`, `/api/v1/fantasy/simulate` | done for read/product views, intentionally deferred for main-dashboard mutations - CLI league/team listing projects through `FantasyLeagueView`; CLI and legacy `fantasy serve` remain the write surfaces for league/team mutations this phase; `fantasy team-use <name>` marks the user's roster for poach availability; roster-gap read surfaces share `FantasyRosterGapView`; league simulation plus add/drop/drop-only scenario projection share `FantasySimulationView` across CLI/TUI/web/JSON; scenario resolution canonicalizes player names and invalid drops render explicit errors; selected fantasy JSON GET reads refuse missing local FantasyDb state without creating `~/.icelines`, existing-FantasyDb Web reads use a read-only SQLite path without creating WAL/SHM sidecar state, and the focused VAL-007 transcript covers CLI/TUI/Web read and deferral surfaces with accepted active-writer risk | Ted Lindsay/Selke |
| Fantasy roster CSV import | `FantasyImportView` | `fantasy import-yahoo --file <path> --league <name> [--my-team <name>] [--dry-run] [--json]` | cmdbar handoff: `fantasy import file=... league ... [dry-run]` | deferred | n/a | done for CLI and handoffs, intentionally deferred for browser mutation - CLI dry-run/apply text and JSON render from `FantasyImportView`; `icelines-fetch::fantasy_import` parses Yahoo roster CSV headers with BOM/flexible-row handling, validates known-player names from NHL/bundled truth, reports unresolved/duplicate/skipped/error rows, and writes only local FantasyDb league/team/roster membership; TUI command bar points to the CLI flow; Jack Adams Web dashboard command parsing rejects browser import as a POST-only deferral so GET navigation cannot mutate FantasyDb; tests cover core contract, parser/importer, CLI L2 import, TUI handoff, and dashboard deferral | Import the Rosters |
| Fantasy daily delta | `FantasyDailyDeltaView` | `fantasy daily --date YYYY-MM-DD` | cmdbar handoff: `fantasy daily date=YYYY-MM-DD` | n/a | `/api/v1/fantasy/daily?date=YYYY-MM-DD` | done - descriptive daily fantasy points are computed from local FantasyDb roster snapshots plus cached finalized boxscores through `build_fantasy_daily_delta_view`; CLI text/JSON and web JSON use the shared ViewModel; TUI and web dashboard command bars hand off to canonical CLI/API surfaces; missing cache and unfinalized games emit warnings/source-state instead of zero-shaped success; selected web JSON missing-cache reads render from existing FantasyDb snapshots without creating `~/.icelines/data`; tests cover core scoring, cached/missing/unfinalized data, CLI JSON, web JSON, and command handoffs | Score the Day |
| Fantasy weekly matchups | `FantasyMatchupWeekView` | `fantasy matchup-set --week ... --home ... [--away ...]`; `fantasy matchup --date YYYY-MM-DD` | cmdbar handoff: `fantasy matchup date=YYYY-MM-DD` | n/a | `/api/v1/fantasy/matchup?date=YYYY-MM-DD` | done - local `fl_matchups` schedule rows define weekly head-to-head pairings and byes; weekly results aggregate cached finalized `FantasyDailyDeltaView` team totals across the ISO week through `build_fantasy_matchup_week_view`; CLI text/JSON and web JSON use the shared ViewModel; TUI and web dashboard command bars hand off to canonical read surfaces; missing schedule/cache and unfinalized games emit empty/source-state warnings instead of zero-shaped success; selected web JSON missing-cache reads render from existing FantasyDb snapshots and matchup rows without creating `~/.icelines/data`; tests cover core outcomes, schedule persistence, cached weekly builder, CLI JSON, web JSON, and command handoffs | Match the Week |
| Fantasy roster shape validation | `RosterShapeValidationView` | `fantasy roster-shape`, `fantasy roster-shape-set <shape>`, `fantasy roster-shape-validate [--team <name>] [--json]` | cmdbar handoff: `fantasy roster-shape [show|validate]`; mutation handoff to CLI | n/a | `/api/v1/fantasy/roster-shape[?team=<name>]` | done - CLI is the canonical mutation surface for league roster-shape presets; CLI and web JSON validation project persisted FantasyDb rosters through `RosterShapeValidationView` using canonical player positions; TUI and web dashboard command bars hand off to read-only validation/API or reject GET-backed mutation; tests cover core contract, FantasyDb/import wiring, CLI L2 JSON, TUI handoffs, web JSON, and dashboard deferral | Shape the Rosters |
| Poacher board | `PoachBoardView` | `poach` | Poach screen | `/poach` | `/api/v1/poach` | implemented - shared board ViewModel across CLI/TUI/web/JSON; `scoring_categories` resolves from explicit query categories or the selected built-in scheme; CLI/TUI/web read active fantasy-league rosters when present to mark `rostered_by_user`, `imported_rostered`, and `imported_available`; CLI/web expose the shared availability filter including `imported-available` for waiver-wire candidates; selected Web imported-availability GET reads use a read-only SQLite path without creating WAL/SHM sidecar state | Selke |
| Watch rules | `WatchRulesView` / `WatchlistView` / `MutationResultView` | `watch ...` | watchlist workspace shows notes/rules/recent alerts; cmdbar can create player rules and enable/disable persisted rules; arbitrary team/deployment editor and destructive delete remain deferred outside web | `/watchlist`, `/watch-rules/create`, `/watch-rules/set-enabled`, `/watch-rules/delete` | `/api/v1/watch-rules`; `/api/v1/watch-rules/set-enabled`; `/api/v1/watchlist` | partial - web watchlist HTML/JSON and Jack Adams dashboard side panes project notes through `WatchlistView`; web watch-rules JSON builds defaults plus persisted rules through `WatchRulesView`; CLI, web, and TUI cmdbar enable/disable resolve through `WatchRuleMutationIntent` and `MutationResultView`-compatible persistence paths; `/watchlist` includes player-rule creation, persisted rule toggles, and deletion; Jack Adams dashboard watch commands create player rules, preserve `when=` for player triggers, and return to the active dashboard workspace; dashboard and TUI cmdbar reject `watch deployment`/team-edit phrases instead of silently creating player rules; TUI cmdbar saves player rules with `watch player <name> when=<trigger>` and toggles persisted rules with `watch enable|disable <id>` while preserving watch notes and fired-alert history; richer arbitrary team/deployment editing remains deferred because the shared mutation intent has no validated team/deployment fields | Selke |
| Poach/weekly reports | `PoachReportView` | `report poach`, `report weekly` | cmdbar report handoff; report viewer deferred | `/reports/poach`, `/reports/weekly` | CLI `--json`; board JSON at `/api/v1/poach` | implemented - markdown/JSON/HTML render from shared report ViewModel, including resolved scoring categories and source omissions; TUI cmdbar parses `report poach` / `report weekly` options and points users to canonical CLI/web report surfaces; Jack Adams Web dashboard command bar opens both report pages as dashboard-ready workspaces with summaries projected from `PoachReportView` | Selke |

---

## Operational surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Data install/list/remove | `DataStatusView` / `MutationResultView` | `data ...` | admin overlay plus cmdbar handoff | `/admin`, `/admin/data/verify`, `/admin/game-cache/load`, `/admin/game-cache/load-favorites` | `/api/v1/admin/data-status`, `/api/v1/admin/data/verify`, `/api/v1/admin/game-cache/load`, `/api/v1/admin/game-cache/load-favorites` | partial - `data status` projects manifest rows through `DataStatusView`; CLI install/remove/verify resolve through `DataMutationIntent`; web HTML/JSON render `DataStatusView`; web data verify HTML forms and JSON mutations resolve through `DataMutationIntent` and return/derive `MutationResultView`; web game-cache load populates per-game boxscore/play-by-play artifacts through the unified manifest cache for records and streaks; web Favorites cache load warms favorite player career teams/seasons and favorite team active-year rows; `/admin` labels game-cache controls as POST-backed cache warmers, not release bundle install/remove operations; web data install remains deferred because it performs live/network release downloads; web data remove remains deferred because it is destructive filesystem mutation; install/remove routes remain unmounted until a scoped safety contract exists; TUI cmdbar opens admin and hands `data ...` to canonical CLI/web targets without running long/destructive operations | Jim Gregory/Ted Lindsay/Guard the Operations |
| Snapshot operations | `SnapshotView` / `MutationResultView` | `snapshot ...` | admin overlay plus cmdbar handoff | `/admin`, `/admin/snapshots/activate`, `/admin/snapshots/delete` | `/api/v1/admin/snapshots`, `/api/v1/admin/snapshots/activate`, `/api/v1/admin/snapshots/delete` | partial - `snapshot list` and `snapshot show` project through `SnapshotView`; CLI use/delete resolve through `SnapshotMutationIntent`; web HTML/JSON now render `SnapshotView`; web activate/delete HTML forms and JSON mutations resolve through `SnapshotMutationIntent` and return/derive `MutationResultView`; delete controls are only rendered for inactive snapshots and backend guards still apply; TUI cmdbar hands `snapshot ...` to canonical CLI/web targets | Jim Gregory/Ted Lindsay |
| Config/report toggles | `ConfigView` / `MutationResultView` | `config ...` | reports/admin overlay plus cmdbar handoff | `/admin`, `/admin/config/set`, `/admin/config/reset` | `/api/v1/admin/config`, `/api/v1/admin/config/set`, `/api/v1/admin/config/reset` | partial - `config get/list` project through `ConfigView`; season-type and CLI config set/reset resolve through shared mutation intents; web HTML/JSON expose runtime web config through `ConfigView`; web runtime config set/reset HTML forms and JSON mutations reuse `ConfigMutationIntent` and return/derive `MutationResultView`; web admin now labels these keys as runtime-only and emits a `ConfigView.warnings` entry that persistent report toggles are deferred; report-toggle writes remain intentionally absent from web because durable report config still lives in the CLI/TUI `Config` type; TUI Reports overlay (`R`) remains the persistent report-toggle writer to `~/.icelines/config.toml`; TUI cmdbar hands `config ...` to canonical CLI/web targets | Jim Gregory/Ted Lindsay/Guard the Operations |
| Docs reference | `DocsView` | `docs` | in-TUI docs overlay | `/docs` | n/a | done - CLI docs, web `/docs`, and the TUI docs overlay use `DocsView`/embedded `COMMANDS.md`; generated MkDocs nav keeps durable guide/reference pages when team rankings regenerate | Lester Patrick/Ted Lindsay/Jim Gregory |

---

## Static site and export surfaces

These are first-class surfaces when the feature is used as a durable artifact or
published reference, even if they are not interactive.

| Artifact | ViewModel/source | Expected output | Status | Owner |
|---|---|---|---|---|
| Generated team pages | `TeamDepthView` / team summary projection | static HTML/markdown team page | done - `icelines-site` builds each generated team page from `TeamDepthView`; fenced by `l1_render_team_page_uses_team_depth_view_slots` | Ted Lindsay/Jim Gregory |
| Leaderboard exports | `LeadersView` | markdown/JSON/CSV where supported | done - markdown default leaders and custom `--columns` render from `LeadersView` rows; custom columns are backed by `LeaderRow.catalog_metrics`; CLI JSON/CSV use the same row contract | Lester Patrick/Campbell |
| Scouting reports | `ReportView` / `PlayerCardView` | markdown report and optional HTML page | done - CLI scouting plus `/scouting/:id` and `/api/v1/scouting/:id` use `ReportView` around the player-card projection | Campbell/Ted Lindsay |
| Poacher reports | `PoachReportView` | markdown/JSON report plus `/reports/poach` and `/reports/weekly` web pages | implemented | Selke |
| Docs/spec site | `DocsView` | generated docs reference | done - MkDocs nav generation preserves the guide/reference section alongside generated team pages; fenced by `l1_update_nav_keeps_guides_reference_section` | Jim Gregory |

---

## Campbell migration target

Campbell does not need to migrate every row. It must:

1. Establish the ViewModel contract.
2. Build `LeadersView`, `TeamDepthView`, and `GoaliesView`.
3. Mark every remaining row as `partial`, `planned`, `verify`, or `deferred`.
4. Leave Ted Lindsay with a web-route verification checklist rather than stale
   route promises.

---

## Ted Lindsay verification checklist

For every mounted web route:

- confirm route exists in `icelines_web::router`;
- confirm active season/type is visible in HTML or JSON `meta`;
- confirm applied filters/sort are visible/bookmarkable where applicable;
- confirm JSON envelope shape;
- confirm shared ViewModel path or record exception;
- confirm HTMX/partial fragments do not drop active context, applied filters, or
  accessible status labels;
- add test reference.

---

## Ted Lindsay web route inventory

Verified from `icelines-web/src/lib.rs` after the handler-module split.

| Route | Handler module | Surface | Matrix row | Status |
|---|---|---|---|---|
| `GET /` | `handlers/home.rs` | HTML | League/home | done - preview skaters/goalies project from `HomeView`; covered by `cargo test -p icelines-web` |
| `GET /dashboard` | `handlers/dashboard.rs` | HTML shell | Shared workbench shell | server-rendered no-JS shell with grouped shared activity catalog, bound experience tabs, scores ribbon, allowlisted `workspace` / `left` / `right` / `experience` URL state, left/right pane selector chips, truthful Favorites/Watchlist/Schedule pane bodies, explicit no-mutation stubs for metadata-only pane bindings, active pane-model and field affordances, command-palette examples, `?partial=workspace` fragment, progressive workspace loader, collapsible side-pane bodies, report workspace links, responsive mobile/tablet breakpoints, local side-pane visibility state, and shared base-shell viewport/skip-link/no-JS guidance; covered by `l0_dashboard_composition_query_is_allowlisted`, `l0_dashboard_pane_options_are_safe_get_navigation`, `l1_dashboard_shell_renders_no_js_regions`, `l1_dashboard_rejects_unsafe_workspace_paths`, `l1_dashboard_workspace_partial_renders_fragment_only`, `l1_html_shell_exposes_no_js_viewport_and_recovery_navigation`, and dashboard static-asset tests |
| `POST /dashboard/command` | `handlers/dashboard.rs` | HTML command action | Jack Adams Web dashboard | deterministic command form endpoint; read commands redirect to allowlisted dashboard workspace URLs, including TUI-shaped phrases such as `poach rw cats=hits,blocks free top=12`, `fantasy poach top=8 available`, `fantasy simulate add Connor_McDavid drop Bench_Forward`, `team EDM season`, and `class 2015`; pane commands preserve URL state, command errors render explicit text labels, and favorite/watch mutations delegate to existing POST handlers/intents; progressively enhanced by `dashboard.js` |
| `GET /static/:asset` | `static_assets` | asset | Static assets | done |
| `GET /leaders` | `handlers/leaders.rs` | HTML | Leaders/skater leaderboard | projects template rows from `LeadersView`; adapter round-trip covered by `l0_web_leaders_view_round_trips_template_and_json_rows` |
| `GET /api/v1/leaders` | `handlers/leaders.rs` | JSON | Leaders/skater leaderboard | projects JSON rows from `LeadersView`; success and bad-filter JSON envelopes are tested |
| `GET /player/:id` | `handlers/player.rs` | HTML | Player card | projects player page from `PlayerCardView`; CLI profile header/current-season block and TUI header/current-season stat strip are aligned; TUI dynamic career table renders from `PlayerCardView.career.catalog_metrics` |
| `GET /api/v1/player/:id` | `handlers/player.rs` | JSON | Player card | projects stable success and error envelopes from `PlayerCardView`; covered by `l1_player_json_*` |
| `GET /player/:id/scoring` | `handlers/scoring.rs` | HTML | Scoring reports | renders a player scoring profile from cached play-by-play, with summary/split/event tables and no local cache directory creation when cache state is missing |
| `GET /api/v1/player/:id/scoring` | `handlers/scoring.rs` | JSON | Scoring reports | returns `PlayerScoringProfileView` in the standard data/meta envelope with play-by-play source-state and no local cache directory creation when cache state is missing |
| `GET /player/:id/streaks` | `handlers/streaks.rs` | HTML | Team/player streak leaders | renders player streak rows from cached boxscores, with in-UI cache-load recovery and no local cache directory creation when cache state is missing |
| `GET /api/v1/player/:id/streaks` | `handlers/streaks.rs` | JSON | Team/player streak leaders | returns `PlayerStreaksView` in the standard data/meta envelope without creating local cache state when cache state is missing |
| `GET /scouting/:id` | `handlers/scouting.rs` | HTML | Scouting report | renders a player-card-backed `ReportView` |
| `GET /api/v1/scouting/:id` | `handlers/scouting.rs` | JSON | Scouting report | returns the player-card-backed `ReportView` contract |
| `GET /compare` | `handlers/compare.rs` | HTML | Compare/comps | projects compare page from `CompareView`; `?a=ID&similar=N` projects similarity rows from `SimilarPlayersView` |
| `GET /api/v1/compare` | `handlers/compare.rs` | JSON | Compare/comps | projects stable data/meta success and shared bad-input error envelopes from `CompareView`; covered by `l1_compare_json_*` |
| `GET /goalies` | `handlers/goalies.rs` | HTML | Goalie leaderboard | projects goalie leaderboard rows from `GoaliesView` |
| `GET /api/v1/goalies` | `handlers/goalies.rs` | JSON | Goalie leaderboard | projects stable data/meta success envelope from `GoaliesView`; covered by `l1_goalies_json_envelope_shape` |
| `GET /team/:abbrev` | `handlers/team.rs` | HTML | Team depth | renders from `TeamDepthView`; TUI scoring chart uses separate `TeamDepthChartView` contract |
| `GET /api/v1/team/:abbrev` | `handlers/team.rs` | JSON | Team depth | projects stable success and error envelopes from `TeamDepthView`; row identity covered by `l1_team_json_rows_match_team_depth_view`; error envelopes covered by `l1_team_json_*` |
| `GET /team/:abbrev/season` | `handlers/team.rs` | HTML | Team season performance | projects `TeamSeasonView` with record, points, home/away splits, form, remaining schedule, standings/cutline context, schedule-strength labels, and quality ledger |
| `GET /api/v1/team/:abbrev/season` | `handlers/team.rs` | JSON | Team season performance | returns `TeamSeasonView` in the standard data/meta envelope, including standings context, schedule strength, and quality ledger fields |
| `GET /team/:abbrev/streaks` | `handlers/team.rs` | HTML | Team/player streak leaders | renders the active-season team skater leaders for goal, assist, and point streaks from cached boxscores, with an in-UI cache-load recovery form and no local cache directory creation when cache state is missing |
| `GET /api/v1/team/:abbrev/streaks` | `handlers/team.rs` | JSON | Team/player streak leaders | returns `TeamPlayerStreaksView` in the standard data/meta envelope with source-state and loaded game/player counts, without creating local cache state when cache state is missing |
| `GET /team/:abbrev/scoring` | `handlers/scoring.rs` | HTML | Scoring reports | renders the active-season team scoring profile from cached play-by-play, with summary/split/event tables, POST-backed cache-load recovery, and no local cache directory creation when cache state is missing |
| `GET /api/v1/team/:abbrev/scoring` | `handlers/scoring.rs` | JSON | Scoring reports | returns `TeamScoringProfileView` in the standard data/meta envelope with play-by-play source-state and no local cache directory creation when cache state is missing |
| `GET /records/player/:id` | `handlers/records.rs` | HTML | Individual records | metric-aware `PlayerRecordsView`; defaults to teams-scored-against, accepts goalies-scored-against and fight-opponents |
| `GET /records/team/:abbrev` | `handlers/records.rs` | HTML | Individual records | metric-aware `TeamRecordsView`; defaults to players-scored-against-team, accepts goalies-beaten-by-team and fight-opponents-by-team |
| `GET /api/v1/records/player/:id` | `handlers/records.rs` | JSON | Individual records | returns metric-aware `PlayerRecordsView` in the standard data/meta envelope |
| `GET /api/v1/records/team/:abbrev` | `handlers/records.rs` | JSON | Individual records | returns metric-aware `TeamRecordsView` in the standard data/meta envelope |
| `GET /depth` | `handlers/depth.rs` | HTML | Cross-team depth | projects depth rankings from `DepthLeagueView`; markdown export includes a `DepthLeagueView` team-strength section |
| `GET /api/v1/depth` | `handlers/depth.rs` | JSON | Cross-team depth | projects stable success and error envelopes from `DepthLeagueView`; row identity covered by `l1_depth_json_rows_match_depth_league_view`; error/envelope shape covered by `l1_depth_json_*` |
| `GET /poach` | `handlers/poach.rs` | HTML | Poacher board | done |
| `GET /reports/poach` | `handlers/poach.rs` | HTML report | Poach/weekly reports | done |
| `GET /reports/weekly` | `handlers/poach.rs` | HTML report | Poach/weekly reports | done |
| `GET /reports/analytics-cache` | `handlers/analytics_cache_report.rs` | HTML report | Major analytics cache | partial - renders a named analytics cache record through `AnalyticsCacheConsumerView`, preserving source state, quality, methodology, disclosures, non-claims, metric evidence, and explicit unavailable copy without recomputing analytics or fetching live data; covered by `l2_wp009_analytics_cache_report_*` |
| `GET /api/v1/poach` | `handlers/poach.rs` | JSON | Poacher board | done - intentionally returns the board ViewModel contract, not the shared API envelope |
| `GET /api/v1/reports/analytics-cache` | `handlers/analytics_cache_report.rs` | JSON report | Major analytics cache | partial - JSON twin for the named cache report; ready responses serialize the preserved consumer ViewModel and unavailable responses return structured status/reason/guidance with 4xx/5xx status mapped from cache-store errors; covered by `l2_wp009_analytics_cache_report_*` |
| `GET /player/evidence-card` | `handlers/analytics_cache_report.rs` | HTML report | Major analytics cache | partial - active-context player evidence card that defaults to `player_evidence_card:<season>:<type>`, renders through `AnalyticsCacheConsumerView` as `PlayerEvidenceCard`, and keeps unavailable/no-recompute/non-claim copy explicit; covered by `l2_wp009_player_evidence_card_*` |
| `GET /api/v1/player/evidence-card` | `handlers/analytics_cache_report.rs` | JSON report | Major analytics cache | partial - JSON twin for the player evidence card; ready responses serialize the `PlayerEvidenceCard` consumer ViewModel and unavailable responses return structured status/reason/guidance without creating cache storage; covered by `l2_wp009_player_evidence_card_*` |
| `GET /scout/opponent` | `handlers/analytics_cache_report.rs` | HTML report | Major analytics cache | partial - active-context opponent scout cache report that defaults to `opponent_scout:<season>:<type>`, renders through `AnalyticsCacheConsumerView` as `OpponentScoutReport`, and keeps unavailable/no-recompute/non-claim copy explicit; covered by `l2_wp009_opponent_scout_*` |
| `GET /api/v1/scout/opponent` | `handlers/analytics_cache_report.rs` | JSON report | Major analytics cache | partial - JSON twin for the opponent scout cache report; ready responses serialize the `OpponentScoutReport` consumer ViewModel and unavailable responses return structured status/reason/guidance without creating cache storage; covered by `l2_wp009_opponent_scout_*` |
| `GET /api/v1/watch-rules` | `handlers/poach.rs` | JSON | Watch rules | partial - projects default and persisted rules through `WatchRulesView`; intentionally returns the rules ViewModel contract |
| `POST /api/v1/watch-rules/set-enabled` | `handlers/poach.rs` | JSON mutation | Watch rules | partial - toggles persisted rules through `WatchRuleMutationIntent` and returns `MutationResultView`; covered by `l1_watch_rule_toggle_json_returns_mutation_result_view` |
| `POST /watch-rules/create` | `handlers/poach.rs` | HTML mutation | Watch rules | partial - creates persisted player watch rules for promotion/availability triggers and redirects to a safe caller return target or `/watchlist`; covered by `l1_watch_rule_create_form_redirects_and_persists_rule` and `l1_dashboard_command_watch_returns_to_dashboard_workspace` |
| `POST /watch-rules/set-enabled` | `handlers/poach.rs` | HTML mutation | Watch rules | partial - form twin for `/watchlist` rule toggles; resolves through `WatchRuleMutationIntent` and redirects back to `/watchlist`; covered by `l1_watch_rule_toggle_form_redirects_and_updates_rule` |
| `POST /watch-rules/delete` | `handlers/poach.rs` | HTML mutation | Watch rules | partial - deletes persisted watch rules and redirects back to `/watchlist`; covered by `l1_watch_rule_delete_form_redirects_and_removes_rule` |
| `GET /career` | `handlers/career.rs` | HTML | Career/cross-league cohorts | partial - projects cohort rows from `CareerView` through the shared `base.html` shell and returns an explicit fetch instruction when the local career-history store is missing; covered by `l1_career_html_uses_shared_page_shell` and `l1_career_missing_store_errors_name_fetch_command` |
| `GET /api/v1/career` | `handlers/career.rs` | JSON | Career/cross-league cohorts | partial - projects stable success and bad-request envelopes from `CareerView` and returns the same missing-store fetch instruction as CLI; covered by `l1_api_career_envelope_shape`, `l1_api_career_rows_match_career_view`, and `l1_career_missing_store_errors_name_fetch_command` |
| `GET /docs` | `handlers/docs.rs` | HTML | Docs reference | renders `COMMANDS.md` through `DocsView`; TUI overlay uses the same docs contract for source metadata/body; career fetch prerequisites are fenced by `l1_docs_route_includes_career_fetch_instruction` |
| `POST /season-type/:kind` | `handlers/season_type.rs` | mutating redirect | Config/report toggles | normalizes season-type mutation intent through shared config support; GET is method-not-allowed and does not mutate state; covered by `l1_season_type_*` |
| `GET /scores` | `handlers/scores.rs` | HTML | Scores/tonight | projects score days from `ScoresView`; CLI `tonight` and TUI scores alignment landed |
| `GET /api/v1/scores` | `handlers/scores.rs` | JSON | Scores/tonight | projects stable data/meta envelope from `ScoresView`; live source failures are `meta.source_error`; covered by `l1_scores_json_envelope_shape` |
| `GET /schedule` | `handlers/schedule.rs` | HTML | Schedule | projects schedule rows from `ScheduleView`; richer TUI-only season-team and matchup projections are covered by `ScheduleTeamView` and `ScheduleMatchupView` |
| `GET /api/v1/schedule` | `handlers/schedule.rs` | JSON | Schedule | projects stable data/meta envelope from `ScheduleView`; live source failures are `meta.source_error`; covered by `l1_schedule_json_envelope_shape` |
| `GET /playoffs` | `handlers/playoffs.rs` | HTML | Playoffs | projects bundled/live bracket through `PlayoffsView` |
| `GET /api/v1/playoffs` | `handlers/playoffs.rs` | JSON | Playoffs | projects bundled/live bracket through `PlayoffsView`; live source failures are `meta.source_error`; covered by `l1_playoffs_json_envelope_shape` |
| `GET /favorites` | `handlers/favorites.rs` | HTML | Favorites/groups | partial - projects selected group membership through `FavoritesView`; `?group=<name>` is read-only and `Favorites` keeps POST-backed add/remove controls; player stat-line lookup reads only existing manifest/boxscore files and does not create cache state on GET; covered by `l1_favorites_html_supports_read_only_group_selection` and `l1_favorites_get_does_not_create_data_cache_when_missing` |
| `GET /api/v1/favorites` | `handlers/favorites.rs` | JSON | Favorites/groups | partial - projects stable `favorites.v1` read payload from `FavoritesView`, including the shared `stat_line` row slot and optional `?group=<name>` selection; covered by `l1_favorites_json_returns_group_members` and `l1_favorites_json_can_read_named_group_without_mutating` |
| `GET /tonight/intel` | `handlers/scoring.rs` | HTML | Scoring reports | renders favorites-first daily scoring intel from cached play-by-play, with favorite team/player scoring rows, POST-backed cache-load recovery, and no local cache directory creation when cache state is missing |
| `GET /api/v1/tonight/intel` | `handlers/scoring.rs` | JSON | Scoring reports | returns `TonightScoringIntelView` in the standard data/meta envelope with play-by-play source-state and no local cache directory creation when cache state is missing |
| `GET /watchlist` | `handlers/favorites.rs` | HTML | Watch rules | partial - projects watchlist notes through `WatchlistView` |
| `GET /api/v1/watchlist` | `handlers/favorites.rs` | JSON | Watch rules | partial - projects stable `watchlist.v1` payload from `WatchlistView`; covered by `l1_watchlist_json_returns_watch_reason_metadata` |
| `GET /game/:id` | `handlers/game.rs` | HTML | Game detail | projects boxscore detail from `GameView`; TUI drilldown goals/goalies/stat-leader panel render from `GameView` rows |
| `GET /api/v1/game/:id` | `handlers/game.rs` | JSON | Game detail | projects stable data/meta envelope from `GameView`; live fetch failures are `meta.source_error`; covered by `l1_conn_smythe_c3_game_json_envelope_shape` |
| `GET /game/:id/scoring` | `handlers/scoring.rs` | HTML | Scoring reports | renders a game scoring report from cached play-by-play, including team/period/situation splits and top shooter IDs, with no local cache directory creation when cache state is missing |
| `GET /api/v1/game/:id/scoring` | `handlers/scoring.rs` | JSON | Scoring reports | returns `GameScoringReportView` in the standard data/meta envelope with play-by-play source-state and no local cache directory creation when cache state is missing |
| `POST /favorites/add` | `handlers/favorites.rs` | mutation | Favorites/groups | partial - normalizes favorite mutation intent through `FavoritesView` support |
| `POST /favorites/remove` | `handlers/favorites.rs` | mutation | Favorites/groups | partial - normalizes favorite mutation intent through `FavoritesView` support |
| `POST /api/v1/favorites/add` | `handlers/favorites.rs` | JSON mutation | Favorites/groups | partial - returns `MutationResultView`; covered by `l1_favorites_add_json_returns_mutation_result_view` |
| `POST /api/v1/favorites/remove` | `handlers/favorites.rs` | JSON mutation | Favorites/groups | partial - returns `MutationResultView`; covered by `l1_favorites_remove_json_returns_mutation_result_view` |
| `GET /transactions` | `handlers/transactions.rs` | HTML | Transactions | projects transaction feed from `TransactionsView`; CLI/TUI row projection is aligned |
| `GET /api/v1/transactions` | `handlers/transactions.rs` | JSON | Transactions | projects stable data/meta success envelope and shared bad-filter error envelope from `TransactionsView`; covered by `l1_transactions_json_envelope_shape` |
| `GET /fantasy` | `handlers/fantasy.rs` | HTML page | Fantasy roster gaps and simulation scenarios | done for read/product views; renders `FantasyRosterGapView` and `FantasySimulationView`, including scenario warnings |
| `GET /api/v1/fantasy/gaps` | `handlers/fantasy.rs` | JSON API | Fantasy roster gaps | done for read/product views; returns `FantasyRosterGapView` |
| `GET /api/v1/fantasy/simulate` | `handlers/fantasy.rs` | JSON API | Fantasy league simulation and add/drop/drop-only scenario projection | done for read/product views; returns `FantasySimulationView` and explicit scenario-resolution errors |
| `GET /api/v1/fantasy/daily` | `handlers/fantasy.rs` | JSON API | Fantasy daily delta scoring | done; returns `FantasyDailyDeltaView` from cached finalized boxscores and local FantasyDb rosters, with explicit missing-cache/unfinalized source state |
| `GET /api/v1/fantasy/matchup` | `handlers/fantasy.rs` | JSON API | Fantasy weekly head-to-head matchups | done; returns `FantasyMatchupWeekView` from local `fl_matchups` schedule rows plus cached finalized daily-delta totals, with explicit missing-schedule/cache/unfinalized source state |
| `GET /api/v1/fantasy/roster-shape` | `handlers/fantasy.rs` | JSON API | Fantasy roster-shape validation | done; returns `RosterShapeValidationView` rows for persisted FantasyDb rosters and canonical player positions; no GET-backed roster-shape mutation |
| `GET /admin` | `handlers/admin.rs` | HTML page | Data status, snapshot operations, config/report toggles | partial - operational shell renders `DataStatusView`, `SnapshotView`, and runtime `ConfigView` plus safe HTML forms for runtime config, data verify, and inactive snapshot activate/delete; data-status rendering does not create local data cache state when manifest state is missing; covered by `l1_admin_html_renders_operational_viewmodels` and the focused admin form tests |
| `GET /api/v1/admin/data-status` | `handlers/admin.rs` | JSON API | Data install/list/remove | partial - returns `DataStatusView` without creating local data cache state when manifest state is missing; covered by `l1_admin_data_status_json_returns_viewmodel_contract` |
| `GET /api/v1/admin/snapshots` | `handlers/admin.rs` | JSON API | Snapshot operations | partial - returns `SnapshotView`; covered by `l1_admin_snapshots_json_returns_viewmodel_contract` |
| `GET /api/v1/admin/config` | `handlers/admin.rs` | JSON API | Config/report toggles | partial - returns runtime web `ConfigView`; covered by `l1_admin_config_json_returns_runtime_config_viewmodel` |
| `POST /api/v1/admin/config/set` | `handlers/admin.rs` | JSON mutation | Config/report toggles | partial - updates runtime web config through `ConfigMutationIntent` and returns `MutationResultView`; covered by `l1_admin_config_set_json_returns_mutation_result_view` |
| `POST /api/v1/admin/config/reset` | `handlers/admin.rs` | JSON mutation | Config/report toggles | partial - resets runtime web config keys through `ConfigMutationIntent` and returns `MutationResultView`; covered by `l1_admin_config_reset_json_returns_noop_when_already_default` |
| `POST /admin/config/set` | `handlers/admin.rs` | HTML mutation | Config/report toggles | partial - updates runtime web config through `ConfigMutationIntent`, derives a `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_config_set_form_redirects_and_updates_runtime_config` |
| `POST /admin/config/reset` | `handlers/admin.rs` | HTML mutation | Config/report toggles | partial - resets runtime web config through `ConfigMutationIntent`, derives a `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_config_reset_form_redirects_and_restores_runtime_config` |
| `POST /api/v1/admin/snapshots/activate` | `handlers/admin.rs` | JSON mutation | Snapshot operations | partial - activates sealed snapshots through `SnapshotMutationIntent` and returns `MutationResultView`; covered by `l1_admin_snapshot_activate_json_returns_mutation_result_view` |
| `POST /admin/snapshots/activate` | `handlers/admin.rs` | HTML mutation | Snapshot operations | partial - activates sealed snapshots through `SnapshotMutationIntent`, derives `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_html_renders_snapshot_activate_form_for_sealed_inactive_rows` and `l1_admin_snapshot_activate_form_redirects_and_sets_active_snapshot` |
| `POST /api/v1/admin/snapshots/delete` | `handlers/admin.rs` | JSON mutation | Snapshot operations | partial - deletes inactive snapshots through `SnapshotMutationIntent` and returns `MutationResultView`; covered by `l1_admin_snapshot_delete_json_returns_mutation_result_view` and `l1_admin_snapshot_delete_json_rejects_active_snapshot` |
| `POST /admin/snapshots/delete` | `handlers/admin.rs` | HTML mutation | Snapshot operations | partial - deletes inactive snapshots through `SnapshotMutationIntent`, derives `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_snapshot_delete_form_redirects_and_removes_inactive_snapshot` |
| `POST /api/v1/admin/data/verify` | `handlers/admin.rs` | JSON mutation | Data install/list/remove | partial - resolves a safe data verification intent, rejects unknown targets, and returns `MutationResultView`; covered by `l1_admin_data_verify_json_returns_mutation_result_view` and `l1_admin_data_verify_json_rejects_unknown_target` |
| `POST /admin/data/verify` | `handlers/admin.rs` | HTML mutation | Data install/list/remove | partial - resolves a safe data verification intent, derives `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_html_renders_data_verify_form_for_manifest_rows` and `l1_admin_data_verify_form_redirects_for_known_target` |
| `POST /api/v1/admin/game-cache/load` | `handlers/admin.rs` | JSON mutation | Data install/list/remove | partial - loads active-season team game-cache artifacts through `icelines_fetch::game_cache` and returns a cache summary |
| `POST /admin/game-cache/load` | `handlers/admin.rs` | HTML mutation | Data install/list/remove | partial - loads active-season team game-cache artifacts from web forms and redirects back to the source page |
| `POST /api/v1/admin/game-cache/load-favorites` | `handlers/admin.rs` | JSON mutation | Data install/list/remove | partial - loads favorite player career game-cache artifacts plus favorite team active-year artifacts and returns a cache summary |
| `POST /admin/game-cache/load-favorites` | `handlers/admin.rs` | HTML mutation | Data install/list/remove | partial - loads favorite player career game-cache artifacts plus favorite team active-year artifacts from web forms and redirects back to admin |

### Web admin operation safety matrix

Pulse 07 keeps the admin surface intentionally conservative. Implemented web
mutations are typed, POST-backed, and covered by fixture-backed tests:
runtime web config set/reset, data verify, sealed snapshot activate, and
inactive snapshot delete. Dangerous or incomplete operations remain explicit
deferrals: web data install is deferred because it performs live/network release
downloads; web data remove is deferred because it is destructive filesystem
mutation without a scoped confirmation contract; `/admin` now labels both
deferrals and fences game-cache forms as cache warmers only; persistent
report-toggle web UI
is explicitly labeled as deferred on `/admin` and in `ConfigView.warnings` until
it can share the CLI/TUI `~/.icelines/config.toml` report contract. The durable
decision table is
`design/waves/2026-05-13-backcheck-the-phases/ADMIN-OPERATIONS-INVENTORY.md`.

TedLindsay.3 should use this table as the route-by-route checklist for parity,
not `design/specs/web-dashboard.md` claims.

---

## Jack Adams Web Dashboard Panel Readiness

`/dashboard?workspace=...` keeps full routes canonical while giving the browser
shell useful no-JS workspace summaries and progressive fragment replacement.
The central dashboard panel is summary-ready for:

| Workspace | Dashboard summary source | Status |
|---|---|---|
| `/`, `/leaders` | `HomeView.top_skaters` | ready |
| `/goalies` | `HomeView.top_goalies` | ready |
| `/depth` | `DepthLeagueView` | ready |
| `/team/:abbrev` | `TeamDepthView` | ready |
| `/team/:abbrev/season` | `TeamSeasonView` | ready |
| `/team/:abbrev/streaks` | `TeamPlayerStreaksView` | ready |
| `/records/player/:id` | `PlayerRecordsView` | ready |
| `/records/team/:abbrev` | `TeamRecordsView` | ready |
| `/player/:id` | `PlayerCardView` | ready |
| `/scores` | scores route result from `ScoresView` | ready |
| `/schedule` | schedule route result from `ScheduleView` | ready |
| `/game/:id` | game route result from `GameView` | ready |
| `/poach` | `PoachBoardView` | ready |
| `/fantasy` | `FantasyRosterGapView` / `FantasySimulationView` | ready |
| `/transactions` | transactions route result from `TransactionsView` | ready |
| `/playoffs` | playoffs route result from `PlayoffsView` | ready |
| `/favorites` | `FavoritesView` | ready |
| `/tonight/intel` | `TonightScoringIntelView` | ready |
| `/player/:id/scoring` | `PlayerScoringProfileView` | ready |
| `/watchlist` | `WatchlistView` plus recent alert count | ready |
| `/career` | `CareerView` | ready |
| `/reports/poach` | `PoachReportView` | ready |
| `/reports/weekly` | `PoachReportView` | ready |
| `/docs` | canonical docs route only; no product summary needed | allowed |

Dashboard command routes and workspace links preserve canonical route state. Pane
composition is explicit read-only URL state (`left`, `right`, `experience`) and
is allowlisted against shared IDs; side-pane visibility remains local browser
state. The catalog, pane binding labels, pane-model labels, field cards, and
bound experience tabs are projected from `icelines-core::workbench` through the
web adapter, while canonical pages remain the full-route source of truth.
Responsive shell behavior keeps the workspace primary, collapses Schedule/right
context first on medium screens when there is no saved preference, and leaves
visible Show/Hide handles for both context panes. The focused dashboard
projection tests live in `handlers::dashboard::tests::*`, while `l1_dashboard_*`
route tests fence shell rendering, URL allowlisting, fragment behavior,
responsive/accessibility tokens, and command redirect safety. Responsive visual
capture is available through
`scripts/test-slice.ps1 web-captures`, which writes desktop and mobile dashboard
screenshots under `dist/web-dashboard-captures/` using installed Edge/Chrome
headless.
