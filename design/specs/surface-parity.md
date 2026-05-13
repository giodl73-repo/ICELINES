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
| Team season performance | `TeamSeasonView` | `team-season <ABBR>` | `team <ABBR> season` | `/team/:abbrev/season` | `/api/v1/team/:abbrev/season` | partial - core, CLI text/JSON, TUI, web HTML, and web JSON project `TeamSeasonView`; distinct from roster/depth and raw schedule list surfaces; shipped metrics include record, points, home/away splits, one-goal split, recent form, remaining schedule counts, next opponents, standings/playoff-cutline context, schedule-strength faced/remaining opponent Pts%, top/middle/bottom buckets, quality wins, expected wins, bad losses, and missed points; richer historical standings/cache persistence remains future polish | Presidents Trophy |
| Cross-team depth | `DepthLeagueView` | `export md depth` team-strength section | `tui depth` | `/depth` | `/api/v1/depth` | done - web HTML, web JSON, TUI league ranking, and markdown team-strength export build from `DepthLeagueView`; web JSON row identity is fenced by `l1_depth_json_rows_match_depth_league_view`; TUI first-row projection is fenced by `l1_tui_depth_league_render_matches_depth_league_view_first_row`; TUI depth Enter navigation and ranking render use `league_view_from_app`; markdown export keeps supplemental player line-value detail as an intentional extra table | Ted Lindsay |
| Compare/comps | `CompareView` / `SimilarPlayersView` | `query compare A B` / `query compare A --similar N` | `tui comps <name>` | `/compare?...` | `/api/v1/compare?...` | done - web HTML and web JSON build `CompareView`; web JSON card identity is fenced by `l1_compare_json_cards_match_compare_view`; CLI head-to-head and TUI comps target card use `CompareView` card projection; CLI similarity, TUI comps list, `/api/v1/compare?a=ID&similar=N`, and `/compare?a=ID&similar=N` project from `SimilarPlayersView`; TUI target is fenced by `l1_tui_comps_target_matches_compare_view_anchor`; web similarity HTML is fenced by `l1_compare_html_similarity_renders_similar_players_section` | Ted Lindsay |
| Career/cohort leaders | `CareerView` | `query career --league ...` | player/favorites affordances plus cmdbar handoff | `/career` | `/api/v1/career` | partial - CLI, templated web HTML, web JSON, and Jack Adams Web dashboard summaries build from `CareerView`; web HTML uses the shared page shell and is fenced by `l1_career_html_uses_shared_page_shell`; web JSON row identity is fenced by `l1_api_career_rows_match_career_view`; CLI adapter alignment is fenced by `l0_cli_career_rows_project_from_career_view`; TUI cmdbar parses `career` cohort options and points users to the canonical CLI/web surfaces; richer dedicated TUI board remains | Calder/Ted Lindsay |
| Scouting report | `ReportView` | `scouting <name>` | player detail/report affordance | `/scouting/:id` | `/api/v1/scouting/:id` | done - CLI scouting and web/API scouting wrap player-card projection in the shared `ReportView` contract with stable scouting section refs | Campbell/Ted Lindsay |
| Markdown export | `ReportView` / `PlayoffsView` / `PoachReportView` | `export md <shape>` | n/a | n/a | n/a | done - all 7 markdown shapes ship; `series` renders a playoff game-log from `PlayoffsView` and `fantasy` renders a poacher report from `PoachReportView` | Campbell/Jim Gregory |

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
| Favorites/groups | `FavoritesView` / `MutationResultView` | `group ...` | favorites/group affordances | `/favorites` | `/api/v1/favorites`, `/api/v1/favorites/add`, `/api/v1/favorites/remove` | partial - web HTML, read JSON, and Jack Adams dashboard side panes project membership through `FavoritesView` and share the same best-effort nightly `stat_line` lookup; web add/remove resolves through `FavoriteMutationIntent`; HTML routes redirect and JSON mutation twins return `MutationResultView` | Ted Lindsay |
| Fantasy league management / roster gaps / simulation | `FantasyLeagueView` / `FantasyRosterGapView` / `FantasySimulationView` / `MutationResultView` | `fantasy ...`, `fantasy gaps`, `fantasy simulate` | `fantasy gaps`, `fantasy simulate` screens | `/fantasy` | `/api/v1/fantasy/gaps`, `/api/v1/fantasy/simulate` | done for read/product views, intentionally deferred for main-dashboard mutations - CLI league/team listing projects through `FantasyLeagueView`; CLI and legacy `fantasy serve` remain the write surfaces for league/team mutations this phase; `fantasy team-use <name>` marks the user's roster for poach availability; roster-gap read surfaces share `FantasyRosterGapView`; league simulation plus add/drop/drop-only scenario projection share `FantasySimulationView` across CLI/TUI/web/JSON; scenario resolution canonicalizes player names and invalid drops render explicit errors | Ted Lindsay/Selke |
| Poacher board | `PoachBoardView` | `poach` | Poach screen | `/poach` | `/api/v1/poach` | implemented - shared board ViewModel across CLI/TUI/web/JSON; `scoring_categories` resolves from explicit query categories or the selected built-in scheme; CLI/TUI/web read active fantasy-league rosters when present to mark `rostered_by_user`, `imported_rostered`, and `imported_available`; CLI/web expose the shared availability filter including `imported-available` for waiver-wire candidates | Selke |
| Watch rules | `WatchRulesView` / `WatchlistView` / `MutationResultView` | `watch ...` | watchlist workspace shows notes/rules/recent alerts; rule editor deferred | `/watchlist`, `/watch-rules/create`, `/watch-rules/set-enabled`, `/watch-rules/delete` | `/api/v1/watch-rules`; `/api/v1/watch-rules/set-enabled`; `/api/v1/watchlist` | partial - web watchlist HTML/JSON and Jack Adams dashboard side panes project notes through `WatchlistView`; web watch-rules JSON builds defaults plus persisted rules through `WatchRulesView`; CLI and web enable/disable resolve through `WatchRuleMutationIntent` and `MutationResultView`; `/watchlist` includes player-rule creation, persisted rule toggles, and deletion; richer arbitrary-rule editing remains deferred | Selke |
| Poach/weekly reports | `PoachReportView` | `report poach`, `report weekly` | cmdbar report handoff; report viewer deferred | `/reports/poach`, `/reports/weekly` | CLI `--json`; board JSON at `/api/v1/poach` | implemented - markdown/JSON/HTML render from shared report ViewModel, including resolved scoring categories and source omissions; TUI cmdbar parses `report poach` / `report weekly` options and points users to canonical CLI/web report surfaces | Selke |

---

## Operational surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Data install/list/remove | `DataStatusView` / `MutationResultView` | `data ...` | admin overlay | `/admin`, `/admin/data/verify` | `/api/v1/admin/data-status`, `/api/v1/admin/data/verify` | partial - `data status` projects manifest rows through `DataStatusView`; CLI install/remove/verify resolve through `DataMutationIntent`; web HTML/JSON now render `DataStatusView`; web data verify HTML forms and JSON mutations resolve through `DataMutationIntent` and return/derive `MutationResultView`; destructive web install/remove remains deferred to Jim Gregory | Jim Gregory/Ted Lindsay |
| Snapshot operations | `SnapshotView` / `MutationResultView` | `snapshot ...` | admin overlay | `/admin`, `/admin/snapshots/activate`, `/admin/snapshots/delete` | `/api/v1/admin/snapshots`, `/api/v1/admin/snapshots/activate`, `/api/v1/admin/snapshots/delete` | partial - `snapshot list` and `snapshot show` project through `SnapshotView`; CLI use/delete resolve through `SnapshotMutationIntent`; web HTML/JSON now render `SnapshotView`; web activate/delete HTML forms and JSON mutations resolve through `SnapshotMutationIntent` and return/derive `MutationResultView`; delete controls are only rendered for inactive snapshots and backend guards still apply | Jim Gregory/Ted Lindsay |
| Config/report toggles | `ConfigView` / `MutationResultView` | `config ...` | reports overlay | `/admin`, `/admin/config/set`, `/admin/config/reset` | `/api/v1/admin/config`, `/api/v1/admin/config/set`, `/api/v1/admin/config/reset` | partial - `config get/list` project through `ConfigView`; season-type and CLI config set/reset resolve through shared mutation intents; web HTML/JSON expose runtime web config through `ConfigView`; web runtime config set/reset HTML forms and JSON mutations reuse `ConfigMutationIntent` and return/derive `MutationResultView`; full persistent CLI config/report-toggle web UI remains planned | Jim Gregory/Ted Lindsay |
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
| `GET /dashboard` | `handlers/dashboard.rs` | HTML shell | Jack Adams Web dashboard | server-rendered no-JS shell with scores ribbon, Favorites/Watchlist context, allowlisted `workspace` URL state, schedule pane, command-palette affordance, `?partial=workspace` fragment, progressive workspace loader, collapsible side-pane bodies, responsive mobile/tablet breakpoints, and local side-pane state; covered by `l1_dashboard_shell_renders_no_js_regions`, `l1_dashboard_rejects_unsafe_workspace_paths`, `l1_dashboard_workspace_partial_renders_fragment_only`, and dashboard static-asset tests |
| `POST /dashboard/command` | `handlers/dashboard.rs` | HTML command action | Jack Adams Web dashboard | deterministic command form endpoint; read commands redirect to allowlisted dashboard workspace URLs, pane commands preserve URL state, command errors render explicit text labels, and favorite/watch mutations delegate to existing POST handlers/intents; progressively enhanced by `dashboard.js` |
| `GET /static/:asset` | `static_assets` | asset | Static assets | done |
| `GET /leaders` | `handlers/leaders.rs` | HTML | Leaders/skater leaderboard | projects template rows from `LeadersView`; adapter round-trip covered by `l0_web_leaders_view_round_trips_template_and_json_rows` |
| `GET /api/v1/leaders` | `handlers/leaders.rs` | JSON | Leaders/skater leaderboard | projects JSON rows from `LeadersView`; success and bad-filter JSON envelopes are tested |
| `GET /player/:id` | `handlers/player.rs` | HTML | Player card | projects player page from `PlayerCardView`; CLI profile header/current-season block and TUI header/current-season stat strip are aligned; TUI dynamic career table renders from `PlayerCardView.career.catalog_metrics` |
| `GET /api/v1/player/:id` | `handlers/player.rs` | JSON | Player card | projects stable success and error envelopes from `PlayerCardView`; covered by `l1_player_json_*` |
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
| `GET /depth` | `handlers/depth.rs` | HTML | Cross-team depth | projects depth rankings from `DepthLeagueView`; markdown export includes a `DepthLeagueView` team-strength section |
| `GET /api/v1/depth` | `handlers/depth.rs` | JSON | Cross-team depth | projects stable success and error envelopes from `DepthLeagueView`; row identity covered by `l1_depth_json_rows_match_depth_league_view`; error/envelope shape covered by `l1_depth_json_*` |
| `GET /poach` | `handlers/poach.rs` | HTML | Poacher board | done |
| `GET /reports/poach` | `handlers/poach.rs` | HTML report | Poach/weekly reports | done |
| `GET /reports/weekly` | `handlers/poach.rs` | HTML report | Poach/weekly reports | done |
| `GET /api/v1/poach` | `handlers/poach.rs` | JSON | Poacher board | done - intentionally returns the board ViewModel contract, not the shared API envelope |
| `GET /api/v1/watch-rules` | `handlers/poach.rs` | JSON | Watch rules | partial - projects default and persisted rules through `WatchRulesView`; intentionally returns the rules ViewModel contract |
| `POST /api/v1/watch-rules/set-enabled` | `handlers/poach.rs` | JSON mutation | Watch rules | partial - toggles persisted rules through `WatchRuleMutationIntent` and returns `MutationResultView`; covered by `l1_watch_rule_toggle_json_returns_mutation_result_view` |
| `POST /watch-rules/create` | `handlers/poach.rs` | HTML mutation | Watch rules | partial - creates persisted player watch rules for promotion/availability triggers and redirects back to `/watchlist`; covered by `l1_watch_rule_create_form_redirects_and_persists_rule` |
| `POST /watch-rules/set-enabled` | `handlers/poach.rs` | HTML mutation | Watch rules | partial - form twin for `/watchlist` rule toggles; resolves through `WatchRuleMutationIntent` and redirects back to `/watchlist`; covered by `l1_watch_rule_toggle_form_redirects_and_updates_rule` |
| `POST /watch-rules/delete` | `handlers/poach.rs` | HTML mutation | Watch rules | partial - deletes persisted watch rules and redirects back to `/watchlist`; covered by `l1_watch_rule_delete_form_redirects_and_removes_rule` |
| `GET /career` | `handlers/career.rs` | HTML | Career/cross-league cohorts | partial - projects cohort rows from `CareerView` through the shared `base.html` shell; covered by `l1_career_html_uses_shared_page_shell` |
| `GET /api/v1/career` | `handlers/career.rs` | JSON | Career/cross-league cohorts | partial - projects stable success and bad-request envelopes from `CareerView`; covered by `l1_api_career_envelope_shape` |
| `GET /docs` | `handlers/docs.rs` | HTML | Docs reference | renders `COMMANDS.md` through `DocsView`; TUI overlay uses the same docs contract for source metadata/body |
| `GET /season-type/:kind` | `handlers/season_type.rs` | mutating redirect | Config/report toggles | normalizes season-type mutation intent through shared config support; covered by `l1_season_type_*` |
| `GET /scores` | `handlers/scores.rs` | HTML | Scores/tonight | projects score days from `ScoresView`; CLI `tonight` and TUI scores alignment landed |
| `GET /api/v1/scores` | `handlers/scores.rs` | JSON | Scores/tonight | projects stable data/meta envelope from `ScoresView`; live source failures are `meta.source_error`; covered by `l1_scores_json_envelope_shape` |
| `GET /schedule` | `handlers/schedule.rs` | HTML | Schedule | projects schedule rows from `ScheduleView`; richer TUI-only season-team and matchup projections are covered by `ScheduleTeamView` and `ScheduleMatchupView` |
| `GET /api/v1/schedule` | `handlers/schedule.rs` | JSON | Schedule | projects stable data/meta envelope from `ScheduleView`; live source failures are `meta.source_error`; covered by `l1_schedule_json_envelope_shape` |
| `GET /playoffs` | `handlers/playoffs.rs` | HTML | Playoffs | projects bundled/live bracket through `PlayoffsView` |
| `GET /api/v1/playoffs` | `handlers/playoffs.rs` | JSON | Playoffs | projects bundled/live bracket through `PlayoffsView`; live source failures are `meta.source_error`; covered by `l1_playoffs_json_envelope_shape` |
| `GET /favorites` | `handlers/favorites.rs` | HTML | Favorites/groups | partial - projects group membership through `FavoritesView` |
| `GET /api/v1/favorites` | `handlers/favorites.rs` | JSON | Favorites/groups | partial - projects stable `favorites.v1` read payload from `FavoritesView`, including the shared `stat_line` row slot; covered by `l1_favorites_json_returns_group_members` |
| `GET /watchlist` | `handlers/favorites.rs` | HTML | Watch rules | partial - projects watchlist notes through `WatchlistView` |
| `GET /api/v1/watchlist` | `handlers/favorites.rs` | JSON | Watch rules | partial - projects stable `watchlist.v1` payload from `WatchlistView`; covered by `l1_watchlist_json_returns_watch_reason_metadata` |
| `GET /game/:id` | `handlers/game.rs` | HTML | Game detail | projects boxscore detail from `GameView`; TUI drilldown goals/goalies/stat-leader panel render from `GameView` rows |
| `GET /api/v1/game/:id` | `handlers/game.rs` | JSON | Game detail | projects stable data/meta envelope from `GameView`; live fetch failures are `meta.source_error`; covered by `l1_conn_smythe_c3_game_json_envelope_shape` |
| `POST /favorites/add` | `handlers/favorites.rs` | mutation | Favorites/groups | partial - normalizes favorite mutation intent through `FavoritesView` support |
| `POST /favorites/remove` | `handlers/favorites.rs` | mutation | Favorites/groups | partial - normalizes favorite mutation intent through `FavoritesView` support |
| `POST /api/v1/favorites/add` | `handlers/favorites.rs` | JSON mutation | Favorites/groups | partial - returns `MutationResultView`; covered by `l1_favorites_add_json_returns_mutation_result_view` |
| `POST /api/v1/favorites/remove` | `handlers/favorites.rs` | JSON mutation | Favorites/groups | partial - returns `MutationResultView`; covered by `l1_favorites_remove_json_returns_mutation_result_view` |
| `GET /transactions` | `handlers/transactions.rs` | HTML | Transactions | projects transaction feed from `TransactionsView`; CLI/TUI row projection is aligned |
| `GET /api/v1/transactions` | `handlers/transactions.rs` | JSON | Transactions | projects stable data/meta success envelope and shared bad-filter error envelope from `TransactionsView`; covered by `l1_transactions_json_envelope_shape` |
| `GET /fantasy` | `handlers/fantasy.rs` | HTML page | Fantasy roster gaps and simulation scenarios | done for read/product views; renders `FantasyRosterGapView` and `FantasySimulationView`, including scenario warnings |
| `GET /api/v1/fantasy/gaps` | `handlers/fantasy.rs` | JSON API | Fantasy roster gaps | done for read/product views; returns `FantasyRosterGapView` |
| `GET /api/v1/fantasy/simulate` | `handlers/fantasy.rs` | JSON API | Fantasy league simulation and add/drop/drop-only scenario projection | done for read/product views; returns `FantasySimulationView` and explicit scenario-resolution errors |
| `GET /admin` | `handlers/admin.rs` | HTML page | Data status, snapshot operations, config/report toggles | partial - operational shell renders `DataStatusView`, `SnapshotView`, and runtime `ConfigView` plus safe HTML forms for runtime config, data verify, and inactive snapshot activate/delete; covered by `l1_admin_html_renders_operational_viewmodels` and the focused admin form tests |
| `GET /api/v1/admin/data-status` | `handlers/admin.rs` | JSON API | Data install/list/remove | partial - returns `DataStatusView`; covered by `l1_admin_data_status_json_returns_viewmodel_contract` |
| `GET /api/v1/admin/snapshots` | `handlers/admin.rs` | JSON API | Snapshot operations | partial - returns `SnapshotView`; covered by `l1_admin_snapshots_json_returns_viewmodel_contract` |
| `GET /api/v1/admin/config` | `handlers/admin.rs` | JSON API | Config/report toggles | partial - returns runtime web `ConfigView`; covered by `l1_admin_config_json_returns_runtime_config_viewmodel` |
| `POST /api/v1/admin/config/set` | `handlers/admin.rs` | JSON mutation | Config/report toggles | partial - updates runtime web config through `ConfigMutationIntent` and returns `MutationResultView`; covered by `l1_admin_config_set_json_returns_mutation_result_view` |
| `POST /api/v1/admin/config/reset` | `handlers/admin.rs` | JSON mutation | Config/report toggles | partial - resets runtime web config keys through `ConfigMutationIntent` and returns `MutationResultView`; covered by `l1_admin_config_reset_json_returns_noop_when_already_default` |
| `POST /admin/config/set` | `handlers/admin.rs` | HTML mutation | Config/report toggles | partial - updates runtime web config through `ConfigMutationIntent`, derives a `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_config_set_form_redirects_and_updates_runtime_config` |
| `POST /admin/config/reset` | `handlers/admin.rs` | HTML mutation | Config/report toggles | partial - resets runtime web config through `ConfigMutationIntent`, derives a `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_config_reset_form_redirects_and_restores_runtime_config` |
| `POST /api/v1/admin/snapshots/activate` | `handlers/admin.rs` | JSON mutation | Snapshot operations | partial - activates sealed snapshots through `SnapshotMutationIntent` and returns `MutationResultView`; covered by `l1_admin_snapshot_activate_json_returns_mutation_result_view` |
| `POST /admin/snapshots/activate` | `handlers/admin.rs` | HTML mutation | Snapshot operations | partial - activates sealed snapshots through `SnapshotMutationIntent`, derives `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_html_renders_snapshot_activate_form_for_sealed_inactive_rows` and `l1_admin_snapshot_activate_form_redirects_and_sets_active_snapshot` |
| `POST /api/v1/admin/snapshots/delete` | `handlers/admin.rs` | JSON mutation | Snapshot operations | partial - deletes inactive snapshots through `SnapshotMutationIntent` and returns `MutationResultView`; covered by `l1_admin_snapshot_delete_json_returns_mutation_result_view` |
| `POST /admin/snapshots/delete` | `handlers/admin.rs` | HTML mutation | Snapshot operations | partial - deletes inactive snapshots through `SnapshotMutationIntent`, derives `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_snapshot_delete_form_redirects_and_removes_inactive_snapshot` |
| `POST /api/v1/admin/data/verify` | `handlers/admin.rs` | JSON mutation | Data install/list/remove | partial - resolves a safe data verification intent, rejects unknown targets, and returns `MutationResultView`; covered by `l1_admin_data_verify_json_returns_mutation_result_view` and `l1_admin_data_verify_json_rejects_unknown_target` |
| `POST /admin/data/verify` | `handlers/admin.rs` | HTML mutation | Data install/list/remove | partial - resolves a safe data verification intent, derives `MutationResultView`, and redirects back to `/admin`; covered by `l1_admin_html_renders_data_verify_form_for_manifest_rows` and `l1_admin_data_verify_form_redirects_for_known_target` |

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
| `/player/:id` | `PlayerCardView` | ready |
| `/scores` | scores route result from `ScoresView` | ready |
| `/schedule` | schedule route result from `ScheduleView` | ready |
| `/game/:id` | game route result from `GameView` | ready |
| `/poach` | `PoachBoardView` | ready |
| `/fantasy` | `FantasyRosterGapView` / `FantasySimulationView` | ready |
| `/transactions` | transactions route result from `TransactionsView` | ready |
| `/playoffs` | playoffs route result from `PlayoffsView` | ready |
| `/favorites` | `FavoritesView` | ready |
| `/watchlist` | `WatchlistView` plus recent alert count | ready |
| `/career` | `CareerView` | ready |
| `/docs` | canonical docs route only; no product summary needed | allowed |

Dashboard command routes and workspace links preserve canonical route state; side
pane visibility remains local browser state. Responsive shell behavior keeps the
workspace primary, collapses Schedule first on medium screens when there is no
saved preference, and leaves visible Show/Hide handles for both context panes.
The focused dashboard projection tests live in `handlers::dashboard::tests::*`,
while `l1_dashboard_*` route tests fence shell rendering, URL allowlisting,
fragment behavior, responsive/accessibility tokens, and command redirect safety.
Responsive visual capture is available through
`scripts/test-slice.ps1 web-captures`, which writes desktop and mobile dashboard
screenshots under `dist/web-dashboard-captures/` using installed Edge/Chrome
headless.
