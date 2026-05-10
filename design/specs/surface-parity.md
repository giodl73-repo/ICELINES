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
| Leaders/skater leaderboard | `LeadersView` | `query leaders` | `tui stats` | `/leaders` | `/api/v1/leaders` | partial - CLI text, web HTML, and web JSON build `LeadersView`; TUI stats and stable CLI JSON/CSV still need Ted/Lester verification | Campbell/Ted Lindsay |
| Goalie leaderboard | `GoaliesView` | `query goalies` | `tui goalies` | `/goalies` | `/api/v1/goalies` | partial - CLI, TUI, web HTML, and web JSON build `GoaliesView`; Ted verifies parity | Campbell/Ted Lindsay |
| Player card | `PlayerCardView` | `query player <name>` | `tui player <name>` | `/player/:id` | `/api/v1/player/:id` | partial - ViewModel pending | Campbell/Ted Lindsay |
| Team depth | `TeamDepthView` | `team <ABBR>` | `tui team <ABBR>` | `/team/:abbrev` | `/api/v1/team/:abbrev` | partial - CLI team and markdown export build `TeamDepthView`; TUI/web depth routes remain local render paths pending Ted/Messier | Campbell/Ted Lindsay |
| Cross-team depth | `TeamDepthView` / `DepthLeagueView` | `depth` or equivalent | `tui depth` | `/depth` | `/api/v1/depth` | verify - route exists, parity needs matrix tests | Ted Lindsay |
| Compare/comps | `CompareView` | `query compare A B` | `tui comps <name>` | `/compare?...` | planned/verify | partial - JSON twin needs verification | Ted Lindsay |
| Scouting report | `ReportView` | `scouting <name>` | player detail/report affordance | planned/verify | planned/verify | partial - report contract pending | Campbell/Ted Lindsay |
| Markdown export | `ReportView` | `export md <shape>` | n/a | n/a | n/a | partial - 5/7 shapes shipped | Campbell/Jim Gregory |

---

## Live and schedule surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Scores/tonight | `ScoresView` | `tonight` | `tui scores` | `/scores` | planned/verify | partial - JSON status needs Ted verification | Ted Lindsay |
| Schedule | `ScheduleView` | `schedule` | `tui schedule` | `/schedule` | planned/verify | partial - CLI exists, web JSON needs verification | Lester Patrick/Ted Lindsay |
| Playoffs | `PlayoffsView` | `playoffs` | `tui playoffs` | `/playoffs` | planned/verify | partial - historical bundle gaps remain | Lester Patrick/Ted Lindsay |
| Transactions | `TransactionsView` | `transactions` | `tui transactions` | `/transactions` | planned/verify | partial - cache/source-state contract pending | Lester Patrick/Ted Lindsay |
| Game detail | `GameView` | n/a | `tui scores` drilldown | `/game/:id` | planned/verify | partial | Ted Lindsay |

---

## User/fantasy/product surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Favorites/groups | `FavoritesView` | `group ...` | favorites/group affordances | `/favorites` | planned/verify | partial - mutation/query parity needs Ted verification | Ted Lindsay |
| Fantasy league management | `FantasyLeagueView` | `fantasy ...` | groups/deep links | `/fantasy...` | planned/verify | deferred/partial - local-only stance remains | Ted Lindsay |
| Poacher board | `PoachBoardView` | `poach` | Poach screen | `/poach` | `/api/v1/poach` | implemented - shared board ViewModel across CLI/TUI/web/JSON | Selke |
| Watch rules | `WatchRulesView` | `watch ...` | watchlist workspace; rule editor deferred | `/watchlist` | `/api/v1/watch-rules`; `/api/v1/watchlist` | partial - rule preview/list/watch-note wired; persistent rule editor/history deferred | Selke |
| Poach/weekly reports | `PoachReportView` | `report poach`, `report weekly` | report viewer deferred | `/reports/poach`, `/reports/weekly` | CLI `--json`; board JSON at `/api/v1/poach` | implemented - markdown/JSON/HTML render from shared report ViewModel | Selke |

---

## Operational surfaces

| Feature | ViewModel | CLI | TUI | Web HTML | Web JSON | Status | Owner |
|---|---|---|---|---|---|---|---|
| Data install/list/remove | `DataStatusView` | `data ...` | admin overlay | admin/snapshots planned/verify | planned/verify | partial | Jim Gregory/Ted Lindsay |
| Snapshot operations | `SnapshotView` | `snapshot ...` | admin overlay | `/admin/snapshots` planned/verify | `/api/v1/admin/snapshots` planned/verify | partial | Jim Gregory/Ted Lindsay |
| Config/report toggles | `ConfigView` | `config ...` | reports overlay | reports/settings planned/verify | `/api/v1/reports` planned/verify | partial | Jim Gregory/Ted Lindsay |
| Docs reference | `DocsView` | `docs` | in-TUI docs overlay planned | `/docs` | n/a | partial | Lester Patrick/Ted Lindsay |

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
