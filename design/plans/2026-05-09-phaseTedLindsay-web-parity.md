# Phase Ted Lindsay - web parity and web architecture

**Date**: 2026-05-09
**Status**: In progress - TedLindsay.2 handler split landed
**Trophy**: Ted Lindsay Award. Fit: the players' choice. This phase makes the browser surface something a regular hockey user would actually choose, while keeping it aligned with CLI and TUI.
**Target release**: post-Campbell, post-Messier, and post-Lester Patrick
**Estimated**: 4-7 sub-phases

---

## Why

The web dashboard has real routes and real value, but it is not yet a fully
synced platform surface:

- `icelines-web/src/lib.rs` is too large and carries many handlers inline.
- Some routes are real, some are partial, some are coming-soon.
- Web query/filter behavior must be brought into parity with the Art Ross and
  Messier grammar.
- HTML pages and JSON twins need a single truth matrix so docs stop drifting.

Ted Lindsay is the web-sync phase: split the architecture, fill the most
important gaps, and make parity measurable.

---

## Role review gates

| Role | Gate |
|---|---|
| HART | Every route that reads player data names active season/type and avoids season-ambiguous caches. |
| KEEL | Every web page/API route maps to the same engine path as CLI/TUI, or is explicitly web-only. |
| TAPE | Pages that mix sources surface missing/partial source state. |
| FORGE | Split the web monolith before adding substantial new behavior; handlers return typed errors. |
| PACE | Pagination, cache TTL, and route performance claims are measured or marked estimates. |
| BENCH | Each route added or changed gets HTML and/or JSON tests at the right tier. |
| EDGE | Test empty filters, repeated filters, bad season/type, missing player, duplicate names, and no data. |
| WIRE | API responses use consistent envelopes, schema versions, and error kinds. |
| SCOUT | Team/depth/player pages remain hockey-sensible after parity changes. |
| GLASS/Broadcast | Active context, filters, sort, empty/error states, bookmarkable URLs, and accessible HTML are visible. |

---

## API contract seed

TedLindsay.1 must either confirm this envelope or replace it before any new
JSON route is considered complete:

```json
{
  "schema_version": "v1",
  "data": {},
  "meta": {
    "season": 20252026,
    "season_type": "regular",
    "source_state": []
  },
  "error": null
}
```

Rules:

- success responses carry `data`, `meta`, and `error: null`;
- error responses carry `data: null`, `meta` where known, and a typed `error`
  with `kind`, `message`, and optional `details`;
- paginated responses put `limit`, `offset` or cursor, and total/has_more in
  `meta`;
- HTML and JSON routes expose the same active context and filter state;
- route-specific exceptions must be recorded in `design/specs/surface-parity.md`.

## Platform contracts consumed

Ted Lindsay consumes `design/specs/platform-contracts.md` this way:

- **Data context**: every HTML and JSON route exposes season/type, source state,
  completeness, and active filters.
- **Query/filter intent**: URL query params lower through the same typed
  parser/planner/filter state as CLI/TUI.
- **ViewModel**: route handlers render ViewModels or explicit JSON projections
  from ViewModels; handlers do not recompute hockey ranking/filtering.
- **Surface parity**: `design/specs/surface-parity.md` is the web truth table.
- **Visual language**: HTML uses shared semantic tokens and bookmarkable state;
  final polish moves to Prince of Wales.

## Sub-phase ordering

```text
TedLindsay.1  Web route inventory and surface matrix
TedLindsay.2  Split icelines-web handler modules
TedLindsay.3  Web query/filter parity
TedLindsay.4  Missing high-value routes and JSON twins
TedLindsay.5  Web UX/accessibility/browser hardening
TedLindsay.6  Docs and closeout
```

---

## TedLindsay.1 - Web route inventory and surface matrix

Create a source-of-truth matrix covering:

- CLI command
- TUI screen
- Web HTML route
- Web JSON route
- Status: `done`, `partial`, `stub`, `deferred`, `n/a`
- Shared engine path
- Tests present

Suggested file:

- `design/specs/surface-parity.md`

Acceptance:

- Every route mounted in `icelines_web::router` appears in the matrix.
- Every README/COMMANDS web claim maps to a matrix row.
- Coming-soon routes are visible as such.

Progress:

- 2026-05-09: `design/specs/surface-parity.md` now includes the mounted web
  route inventory from `icelines_web::router`, including HTML, JSON, mutation,
  asset, report, and stub routes.
- 2026-05-10: Added `icelines-web/tests/ted_lindsay_route_inventory.rs`,
  which fails if a mounted router route is missing from
  `design/specs/surface-parity.md`.
- 2026-05-10: Migrated `/team/:abbrev` and `/api/v1/team/:abbrev` to build
  from `TeamDepthView` while preserving the existing HTML template rows and
  stable JSON envelope projection.
- 2026-05-10: Added core `PlayerCardView` and migrated
  `/api/v1/player/:id` to project its existing stable JSON envelope from that
  ViewModel. HTML player-card rendering remains a follow-up adapter.
- 2026-05-10: Migrated `/player/:id` HTML rendering to project from
  `PlayerCardView` while preserving the existing template fields, career-table
  filtering, compare suggestions, and headshot fallback behavior.
- 2026-05-10: Added core `CompareView` and migrated `/compare` plus
  `/api/v1/compare` to project existing HTML/JSON cards from the shared
  compare contract.
- 2026-05-10: Added core `DepthLeagueView` and migrated `/depth` plus
  `/api/v1/depth` to project existing cross-team depth rankings from the
  shared league-depth contract.
- 2026-05-10: Added core `ScoresView` and migrated `/scores` plus
  `/api/v1/scores` to project date grouping, game status labels, start times,
  and playoff series context from the shared scores contract.
- 2026-05-10: Added core `ScheduleView` and migrated `/schedule` plus
  `/api/v1/schedule` to project date/team schedule rows, team chips, and
  active-team home/away perspective from the shared schedule contract.
- 2026-05-10: Added core `TransactionsView` and migrated `/transactions` plus
  `/api/v1/transactions` to project filtering, pretty labels, coverage flags,
  and row truncation from the shared transactions contract.
- 2026-05-10: Added core `GameView` and migrated `/game/:id` plus
  `/api/v1/game/:id` to project boxscore score state, goalie lines, goal log,
  and top skaters from the shared game-detail contract.
- 2026-05-10: Added core `PlayoffsView` and migrated `/playoffs` plus
  `/api/v1/playoffs` to project bundled/live bracket rounds and series from
  the shared playoffs contract.
- 2026-05-10: Added ViewModel-level `FavoritesView` for local group
  membership and migrated `/favorites` plus `/api/v1/favorites` to project
  membership rows/counts through that shared contract while preserving the
  existing `favorites.v1` JSON payload.
- 2026-05-10: Added ViewModel-level `WatchlistView` for persisted watchlist
  membership plus notes and migrated `/watchlist` plus `/api/v1/watchlist` to
  project through that shared read contract while keeping richer
  `WatchRulesView` rule/history work separate.
- 2026-05-10: Added core `CareerView` and migrated `/career` plus
  `/api/v1/career` to project cross-league cohort rows, resolved season, sort,
  counts, and empty state through the shared career contract.
- 2026-05-10: Added core `HomeView` and migrated `/` to select top skater and
  goalie preview rows through the shared home contract while preserving the
  existing template and headshot projection.
- 2026-05-10: Added core `DocsView` and migrated `/docs` to carry rendered
  `COMMANDS.md` plus source metadata through the shared docs contract.
- 2026-05-10: Moved watch-rule source-state assembly and persisted-rule merge
  into the shared `WatchRulesView` builder used by `/api/v1/watch-rules`.
- 2026-05-10: Added shared favorite mutation intent normalization so
  `/favorites/add` and `/favorites/remove` use the same team/player detection,
  player-name normalization, entity-ref construction, and safe redirect logic
  as the platform contract instead of duplicating it in the web helper.
- 2026-05-10: Added shared season-type mutation intent normalization so
  `/season-type/:kind` uses one core contract for season-type whitelisting and
  safe same-origin redirect selection while the web layer only mutates state.

---

## TedLindsay.2 - Split `icelines-web` handler modules

Move inline handler modules out of `icelines-web/src/lib.rs`. The route
inventory decides the final module boundaries; this default split is the
starting point, not a mandate:

- Shared row-projection and headshot helpers already moved into
  `icelines-web/src/handlers/shared.rs` so `lib.rs` no longer owns the
  cross-route projection seam.
- 2026-05-09 wave: inline web handlers moved out of `lib.rs` into
  `icelines-web/src/handlers/*.rs`. `lib.rs` now keeps the router, shared
  module declarations, and the thin `handlers` facade.

```text
icelines-web/src/handlers/coming_soon.rs
icelines-web/src/handlers/compare.rs
icelines-web/src/handlers/depth.rs
icelines-web/src/handlers/docs.rs
icelines-web/src/handlers/favorites.rs
icelines-web/src/handlers/game.rs
icelines-web/src/handlers/goalies.rs
icelines-web/src/handlers/home.rs
icelines-web/src/handlers/leaders.rs
icelines-web/src/handlers/player.rs
icelines-web/src/handlers/playoffs.rs
icelines-web/src/handlers/poach.rs
icelines-web/src/handlers/schedule.rs
icelines-web/src/handlers/scores.rs
icelines-web/src/handlers/season_type.rs
icelines-web/src/handlers/team.rs
icelines-web/src/handlers/transactions.rs
icelines-web/src/handlers/not_found.rs
icelines-web/src/handlers/shared.rs
```

Keep `lib.rs` focused on:

- module declarations,
- public re-exports,
- router construction.

Acceptance:

- No route behavior changes.
- Existing web tests pass.
- `icelines-web/src/lib.rs` is reduced to a router/facade, not a handler monolith.

Verification:

- `cargo test -p icelines-web`

---

## TedLindsay.3 - Web query/filter parity

Unify web filters with the same query stack used by CLI/TUI:

- repeated `filter=` means AND,
- `sort` keys align with `StatId` where applicable,
- nationality/country/min-gp/position behavior matches Messier and Art Ross,
- goalie filters behave consistently with goalie CLI/TUI surfaces,
- bad filters produce useful HTML and JSON errors.

Acceptance:

- Web `/api/v1/leaders` and CLI `query leaders --json` agree on row identity for a fixture set.
- Repeated-filter tests exist.
- Bad-filter tests cover HTML and JSON.
- Applied filters are visible in the page URL and on the page.

---

## TedLindsay.4 - Missing high-value routes and JSON twins

Prioritize:

1. leaders
2. player
3. team
4. goalies
5. depth
6. scores/schedule/playoffs
7. transactions
8. favorites
9. fantasy
10. docs/search/admin snapshots

Rules:

- HTML without JSON is allowed only if the matrix marks JSON deferred.
- JSON without schema/envelope test is not complete.
- Stubs may remain, but they must not be advertised as shipped.

Acceptance:

- Every high-value route is either implemented with tests or marked deferred with rationale.
- `/fantasy` is no longer silently "coming soon" if docs claim fantasy web exists.

---

## TedLindsay.5 - Web UX/accessibility/browser hardening

Broadcast/GLASS checklist:

- every page shows active season/type,
- filters and sort are visible,
- state is bookmarkable in the URL,
- tables use semantic `<table>` markup,
- no color-only meaning,
- useful 404 with recovery,
- empty filter results offer remove-filter affordances,
- static assets have correct MIME and cache headers,
- LAN mode warnings remain clear in `serve`.

Acceptance:

- HTML route tests assert active context header.
- Basic accessibility structure tests exist for major pages.
- CSS color contract matches GLASS/Broadcast expectations.

---

## TedLindsay.6 - Docs and closeout

Update:

- `COMMANDS.md` web route section
- `README.md`
- `design/specs/web-dashboard.md`
- `design/plans/INDEX.md`
- `CHANGELOG.md`

Acceptance:

- Surface matrix is the source of truth for web status.
- Docs stop promising unimplemented web behavior.
- Remaining beauty/readability work is handed to Prince of Wales with route
  truth already settled. Ted Lindsay makes the web honest and structured;
  Prince of Wales makes it visually excellent.

---

## Out of scope

- Public internet deployment, auth, TLS, accounts.
- SPA rewrite or JavaScript build system.
- Web fetch/write workflows beyond already-approved local POST mutators.
- New analytics beyond parity and route completion.
