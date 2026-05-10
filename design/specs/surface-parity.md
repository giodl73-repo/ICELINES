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
| Leaders/skater leaderboard | `LeadersView` | `query leaders` | `tui stats` | `/leaders` | `/api/v1/leaders` | partial - CLI text, CLI JSON/CSV, web HTML, web JSON, and TUI stats result rendering build `LeadersView`; web JSON now applies repeated filters and discrete bio query filters; TUI execution/selection still needs Ted/Lester adapter verification | Campbell/Ted Lindsay |
| League/home preview | `HomeView` | n/a | dashboard shell | `/` | n/a | done - web home preview skaters/goalies build from `HomeView` | Ted Lindsay |
| Goalie leaderboard | `GoaliesView` | `query goalies` | `tui goalies` | `/goalies` | `/api/v1/goalies` | partial - CLI, TUI, web HTML, and web JSON build `GoaliesView`; Ted verifies parity | Campbell/Ted Lindsay |
| Player card | `PlayerCardView` | `query player <name>` | `tui player <name>` | `/player/:id` | `/api/v1/player/:id` | partial - web HTML and web JSON project from `PlayerCardView`; CLI/TUI adapter alignment remains | Campbell/Ted Lindsay |
| Team depth | `TeamDepthView` | `team <ABBR>` | `tui team <ABBR>` | `/team/:abbrev` | `/api/v1/team/:abbrev` | partial - CLI team, markdown export, web HTML, and web JSON build `TeamDepthView`; TUI team/depth alignment remains pending Ted/Messier | Campbell/Ted Lindsay |
| Cross-team depth | `DepthLeagueView` | `depth` or equivalent | `tui depth` | `/depth` | `/api/v1/depth` | partial - web HTML and web JSON build `DepthLeagueView`; CLI/TUI adapter alignment remains | Ted Lindsay |
| Compare/comps | `CompareView` | `query compare A B` | `tui comps <name>` | `/compare?...` | `/api/v1/compare?...` | partial - web HTML and web JSON build `CompareView`; CLI/TUI adapter alignment remains | Ted Lindsay |
| Career/cohort leaders | `CareerView` | `query career --league ...` | player/favorites affordances | `/career` | `/api/v1/career` | partial - web HTML and web JSON build `CareerView`; CLI adapter alignment remains | Calder/Ted Lindsay |
| Scouting report | `ReportView` | `scouting <name>` | player detail/report affordance | not mounted | not mounted | partial - report contract pending; web route deferred | Campbell/Ted Lindsay |
| Markdown export | `ReportView` | `export md <shape>` | n/a | n/a | n/a | partial - 5/7 shapes shipped | Campbell/Jim Gregory |

---

## Live and schedule surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Scores/tonight | `ScoresView` | `tonight` | `tui scores` | `/scores` | `/api/v1/scores` | partial - web HTML and web JSON build `ScoresView`; CLI/TUI adapter alignment remains | Ted Lindsay |
| Schedule | `ScheduleView` | `schedule` | `tui schedule` | `/schedule` | `/api/v1/schedule` | partial - web HTML and web JSON build `ScheduleView`; CLI/TUI adapter alignment remains | Lester Patrick/Ted Lindsay |
| Playoffs | `PlayoffsView` | `playoffs` | `tui playoffs` | `/playoffs` | `/api/v1/playoffs` | partial - web HTML and JSON now project through `PlayoffsView`; historical bundle gaps and CLI/TUI adapter alignment remain | Lester Patrick/Ted Lindsay |
| Transactions | `TransactionsView` | `transactions` | `tui transactions` | `/transactions` | `/api/v1/transactions` | partial - web HTML and web JSON build `TransactionsView`; CLI/TUI adapter alignment remains | Lester Patrick/Ted Lindsay |
| Game detail | `GameView` | n/a | `tui scores` drilldown | `/game/:id` | `/api/v1/game/:id` | partial - web HTML and web JSON build `GameView`; TUI drilldown alignment remains | Ted Lindsay |

---

## User/fantasy/product surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Favorites/groups | `FavoritesView` | `group ...` | favorites/group affordances | `/favorites` | `/api/v1/favorites` | partial - web HTML/read JSON project membership through `FavoritesView`; web mutation normalization uses shared intent; richer nightly dashboard alignment remains | Ted Lindsay |
| Fantasy league management | `FantasyLeagueView` | `fantasy ...` | groups/deep links | `/fantasy` stub only | not mounted | deferred - local CLI remains primary; web `/fantasy/*` fold-in is not shipped | Ted Lindsay |
| Poacher board | `PoachBoardView` | `poach` | Poach screen | `/poach` | `/api/v1/poach` | implemented - shared board ViewModel across CLI/TUI/web/JSON | Selke |
| Watch rules | `WatchRulesView` / `WatchlistView` | `watch ...` | watchlist workspace shows notes/rules/recent alerts; rule editor deferred | `/watchlist` | `/api/v1/watch-rules`; `/api/v1/watchlist` | partial - web watchlist HTML/JSON project notes through `WatchlistView`; web watch-rules JSON builds defaults plus persisted rules through `WatchRulesView`; editor/toggle UX remains deferred | Selke |
| Poach/weekly reports | `PoachReportView` | `report poach`, `report weekly` | report viewer deferred | `/reports/poach`, `/reports/weekly` | CLI `--json`; board JSON at `/api/v1/poach` | implemented - markdown/JSON/HTML render from shared report ViewModel | Selke |

---

## Operational surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Data install/list/remove | `DataStatusView` | `data ...` | admin overlay | not mounted | not mounted | partial - CLI/TUI exist; web admin deferred to Jim Gregory | Jim Gregory/Ted Lindsay |
| Snapshot operations | `SnapshotView` | `snapshot ...` | admin overlay | not mounted | not mounted | partial - CLI/TUI exist; web snapshot admin deferred to Jim Gregory | Jim Gregory/Ted Lindsay |
| Config/report toggles | `ConfigView` | `config ...` | reports overlay | not mounted | not mounted | partial - web season-type mutation and read-side parsing use shared core support; report toggles remain planned | Jim Gregory/Ted Lindsay |
| Docs reference | `DocsView` | `docs` | in-TUI docs overlay planned | `/docs` | n/a | partial - web docs render from `DocsView`; TUI overlay alignment remains | Lester Patrick/Ted Lindsay |

---

## Static site and export surfaces

These are first-class surfaces when the feature is used as a durable artifact or
published reference, even if they are not interactive.

| Artifact | ViewModel/source | Expected output | Status | Owner |
|---|---|---|---|---|
| Generated team pages | `TeamDepthView` / team summary projection | static HTML/markdown team page | verify - current claims need route/export check | Ted Lindsay/Jim Gregory |
| Leaderboard exports | `LeadersView` | markdown/JSON/CSV where supported | partial - markdown default leaders uses `LeadersView`; custom columns and stable JSON/CSV remain on catalog projections | Lester Patrick/Campbell |
| Scouting reports | `ReportView` / `PlayerCardView` | markdown report and optional HTML page | partial - contract pending | Campbell/Ted Lindsay |
| Poacher reports | `PoachReportView` | markdown/JSON report plus `/reports/poach` and `/reports/weekly` web pages | implemented | Selke |
| Docs/spec site | `DocsView` | generated docs reference | partial - source of truth exists, generated state needs verification | Jim Gregory |

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
| `GET /static/:asset` | `static_assets` | asset | Static assets | done |
| `GET /leaders` | `handlers/leaders.rs` | HTML | Leaders/skater leaderboard | partial - parity checks continue in TedLindsay.3 |
| `GET /api/v1/leaders` | `handlers/leaders.rs` | JSON | Leaders/skater leaderboard | partial - success and bad-filter JSON envelopes are tested; parity checks continue in TedLindsay.3 |
| `GET /player/:id` | `handlers/player.rs` | HTML | Player card | partial - projects player page from `PlayerCardView`; CLI/TUI alignment remains |
| `GET /api/v1/player/:id` | `handlers/player.rs` | JSON | Player card | partial - projects stable success and error envelopes from `PlayerCardView`; covered by `l1_player_json_*` |
| `GET /compare` | `handlers/compare.rs` | HTML | Compare/comps | partial - projects compare page from `CompareView`; CLI/TUI alignment remains |
| `GET /api/v1/compare` | `handlers/compare.rs` | JSON | Compare/comps | partial - projects stable data/meta success and shared bad-input error envelopes from `CompareView`; covered by `l1_compare_json_*` |
| `GET /goalies` | `handlers/goalies.rs` | HTML | Goalie leaderboard | partial - parity checks continue in TedLindsay.3 |
| `GET /api/v1/goalies` | `handlers/goalies.rs` | JSON | Goalie leaderboard | partial - projects stable data/meta success envelope from `GoaliesView`; covered by `l1_goalies_json_envelope_shape` |
| `GET /team/:abbrev` | `handlers/team.rs` | HTML | Team depth | partial - renders from `TeamDepthView`, HTML projection shape preserved |
| `GET /api/v1/team/:abbrev` | `handlers/team.rs` | JSON | Team depth | partial - projects stable success and error envelopes from `TeamDepthView`; covered by `l1_team_json_*` |
| `GET /depth` | `handlers/depth.rs` | HTML | Cross-team depth | partial - projects depth rankings from `DepthLeagueView`; CLI/TUI alignment remains |
| `GET /api/v1/depth` | `handlers/depth.rs` | JSON | Cross-team depth | partial - projects stable success and error envelopes from `DepthLeagueView`; covered by `l1_depth_json_*` |
| `GET /poach` | `handlers/poach.rs` | HTML | Poacher board | done |
| `GET /reports/poach` | `handlers/poach.rs` | HTML report | Poach/weekly reports | done |
| `GET /reports/weekly` | `handlers/poach.rs` | HTML report | Poach/weekly reports | done |
| `GET /api/v1/poach` | `handlers/poach.rs` | JSON | Poacher board | done - intentionally returns the board ViewModel contract, not the shared API envelope |
| `GET /api/v1/watch-rules` | `handlers/poach.rs` | JSON | Watch rules | partial - projects default and persisted rules through `WatchRulesView`; intentionally returns the rules ViewModel contract |
| `GET /career` | `handlers/career.rs` | HTML | Career/cross-league cohorts | partial - projects cohort rows from `CareerView` |
| `GET /api/v1/career` | `handlers/career.rs` | JSON | Career/cross-league cohorts | partial - projects stable success and bad-request envelopes from `CareerView`; covered by `l1_api_career_envelope_shape` |
| `GET /docs` | `handlers/docs.rs` | HTML | Docs reference | partial - renders `COMMANDS.md` through `DocsView` |
| `GET /season-type/:kind` | `handlers/season_type.rs` | mutating redirect | Config/report toggles | partial - normalizes season-type mutation intent through shared config support |
| `GET /scores` | `handlers/scores.rs` | HTML | Scores/tonight | partial - projects score days from `ScoresView`; CLI/TUI alignment remains |
| `GET /api/v1/scores` | `handlers/scores.rs` | JSON | Scores/tonight | partial - projects stable data/meta envelope from `ScoresView`; live source failures are `meta.source_error`; covered by `l1_scores_json_envelope_shape` |
| `GET /schedule` | `handlers/schedule.rs` | HTML | Schedule | partial - projects schedule rows from `ScheduleView`; CLI/TUI alignment remains |
| `GET /api/v1/schedule` | `handlers/schedule.rs` | JSON | Schedule | partial - projects stable data/meta envelope from `ScheduleView`; live source failures are `meta.source_error`; covered by `l1_schedule_json_envelope_shape` |
| `GET /playoffs` | `handlers/playoffs.rs` | HTML | Playoffs | projects bundled/live bracket through `PlayoffsView` |
| `GET /api/v1/playoffs` | `handlers/playoffs.rs` | JSON | Playoffs | projects bundled/live bracket through `PlayoffsView`; live source failures are `meta.source_error`; covered by `l1_playoffs_json_envelope_shape` |
| `GET /favorites` | `handlers/favorites.rs` | HTML | Favorites/groups | partial - projects group membership through `FavoritesView` |
| `GET /api/v1/favorites` | `handlers/favorites.rs` | JSON | Favorites/groups | partial - projects stable `favorites.v1` read payload from `FavoritesView`; covered by `l1_favorites_json_returns_group_members` |
| `GET /watchlist` | `handlers/favorites.rs` | HTML | Watch rules | partial - projects watchlist notes through `WatchlistView` |
| `GET /api/v1/watchlist` | `handlers/favorites.rs` | JSON | Watch rules | partial - projects stable `watchlist.v1` payload from `WatchlistView`; covered by `l1_watchlist_json_returns_watch_reason_metadata` |
| `GET /game/:id` | `handlers/game.rs` | HTML | Game detail | partial - projects boxscore detail from `GameView`; TUI alignment remains |
| `GET /api/v1/game/:id` | `handlers/game.rs` | JSON | Game detail | partial - projects stable data/meta envelope from `GameView`; live fetch failures are `meta.source_error`; covered by `l1_conn_smythe_c3_game_json_envelope_shape` |
| `POST /favorites/add` | `handlers/favorites.rs` | mutation | Favorites/groups | partial - normalizes favorite mutation intent through `FavoritesView` support |
| `POST /favorites/remove` | `handlers/favorites.rs` | mutation | Favorites/groups | partial - normalizes favorite mutation intent through `FavoritesView` support |
| `GET /transactions` | `handlers/transactions.rs` | HTML | Transactions | partial - projects transaction feed from `TransactionsView`; CLI/TUI alignment remains |
| `GET /api/v1/transactions` | `handlers/transactions.rs` | JSON | Transactions | partial - projects stable data/meta success envelope and shared bad-filter error envelope from `TransactionsView`; covered by `l1_transactions_json_envelope_shape` |
| `GET /fantasy` | `handlers/coming_soon.rs` | HTML stub | Fantasy league management | deferred |

TedLindsay.3 should use this table as the route-by-route checklist for parity,
not `design/specs/web-dashboard.md` claims.
