# Web Dashboard — King Clancy spec

> *"The King Clancy Memorial Trophy is awarded to the player who best exemplifies leadership qualities on and off the ice and has made a noteworthy humanitarian contribution in his community."* — NHL.com
>
> Phase King Clancy = take icelines to the people via the browser. Same projection model the TUI uses; new HTML / JSON surface so users who never open a terminal can still drive the analytics.

## Status

Draft — pre-implementation. **Reviews requested before any code lands.**

## Decisions locked (2026-05-03, refined 2026-05-04 after role review)

| Question | Decision | Notes |
|----------|----------|-------|
| `serve` rename | Plain `icelines serve` = web dashboard. mkdocs path → `icelines site serve` (under `icelines site {build,serve,deploy}`). | Staged migration in Migration mechanics. Old aliases removed in v0.14. |
| Default port | **8000** | As-built dashboard parity routes live here; the legacy `fantasy serve --port 8080` mutation server remains separate. |
| Auto-open browser | **Default on**. `--no-open` opt-out. | URL printed BEFORE open attempt. Open errors swallowed. |
| HTMX + CSS + logo | **Vendored** (`include_bytes!`) — ~30 KB total. | No CDN. No internet at runtime. |
| Fantasy read/product routes | **Folded** as `/fantasy` and `/api/v1/fantasy/*` for gaps/simulation. | Legacy standalone `fantasy serve` mutation routes remain documented in `fantasy-leagues.md`. |
| `schema_version` | **Per-envelope integer**, additive-only within a version. | Renames/removals → that route bumps to `/api/v2/...`. Replaces the original "global v1 additive-only" plan. |
| API prefix | **`/api/v1/`** from day 1. | Cheap now, painful to retrofit. |
| Pagination | `?limit=N&offset=N` with `meta.total` and `meta.next_offset`. Default 50, max 500. | All list routes. |
| Filter encoding | URL grammar identical to `--filter`. Repeated `?filter=` ANDs. axum `Query<Vec<(String,String)>>`. | Frozen golden test. |
| Concurrency model | `Arc<RwLock<StatsRepository>>` with reads non-blocking. Fallback: `LocalSet` + `Rc<RefCell>` if Send-conversion is too invasive. | Lazy career loads use temp repo + brief swap. |
| Response cache | 30-second TTL keyed per (season, type, route, sort, filter_hash, top, preset). PATCH invalidates. | `moka::sync::Cache`. |
| Rate-stat floor | Implicit `gp_min` floor on per-game sorts (skaters: 10, goalies: 5). Echoed in `meta.implicit_filters`. `?include_below_threshold=true` to bypass. | scout-flag mitigation. |
| Web crate location | New `icelines-web` crate (peer of `icelines-site`). | Avoids the 1700-line `commands/fantasy.rs` smell. Handlers + templates + WebError live there. |
| Error model | `WebError` thiserror enum + `IntoResponse` impl. PATCH bodies use `serde(deny_unknown_fields)`. | Forge contract. |

## Goal

A single command — `icelines serve` — that boots a localhost web dashboard exposing every analytics surface the TUI does. No internet required, no JS toolchain required, no build step for the user. Every release artifact already serves the latest dashboard the moment the user runs `icelines serve`.

## Why now

- The 38-season bundle (L.7b) means a fresh binary already has decades of data on disk.
- Phase Reports' overlay model (toggleable Tier-1 reports) is a natural fit for a sticky web sidebar.
- UX.1's lazy career loader is HTTP-friendly: each `/player/:id` request fans out across bundles into the repo cache.
- The fantasy axum server (`fantasy serve`) already proves the axum + HTML + JSON shape works in this codebase.
- A web surface unblocks two personas the TUI doesn't reach: the casual user who'd never open a terminal, and embedders who want the JSON API to power their own UI.

## Non-goals

- **Not a public-internet deployment.** Localhost only by default. We ship `--bind 0.0.0.0` for users who explicitly want LAN exposure, but no auth, no TLS — those are user-deployment concerns, not what King Clancy delivers.
- **Not a SPA.** Server-rendered HTML with HTMX for interactivity. No React / Vue / build step. The user runs one binary; the binary serves everything.
- **Not a replacement for the TUI.** The TUI stays first-class. The web is a parallel surface, not a successor.
- **Not a fetch trigger.** The web app reads bundled / installed data. Fetching is still a CLI-only operation (`icelines fetch`). King Clancy's job is to surface the data, not pull new data.

## Design principles

1. **One binary, one command.** `icelines serve` boots and prints `→ http://localhost:8000` (or whatever port). Browser does the rest.
2. **Reuse the projection model.** Routes call into `StatsRepository`, `PlayerFilter`, `parse_filter_expr` — the same layer the CLI and TUI use. No duplicate query logic.
3. **JSON keys are `StatId::cli_key`.** KEEL-B1 contract holds: every `/api/...` response uses the same keys as `query leaders --json`. An AI / scripter / spreadsheet user can grep across surfaces.
4. **HTML pages mirror TUI screens 1:1.** Same data; just a browser-friendly layout. If you can navigate the TUI, the web app is obvious.
5. **Reports overlay + season picker are first-class.** The same `~/.icelines/config.toml` toggles drive both surfaces. Toggle on the web → TUI sees the change next launch.
6. **Vendored interactivity, no CDN.** HTMX + CSS + logo all `include_bytes!`'d into the binary. No internet at runtime. ~30 KB total — negligible against the 56 MB bundle.

## Command surface

### `icelines serve`

```bash
icelines serve                       # bind 127.0.0.1:8000, auto-open browser
icelines serve --port 9000
icelines serve --bind 0.0.0.0:8000   # LAN-accessible (explicit opt-in)
icelines serve --no-open             # don't auto-open the browser
icelines serve --reload              # rebuild data caches on each request (dev mode)
```

### `icelines site` (renamed mkdocs path)

```bash
icelines site build                  # was: icelines build
icelines site serve                  # was: icelines serve  (old behavior preserved)
icelines site deploy                 # was: icelines deploy
```

The bare `icelines serve` now means the King Clancy web dashboard. Old `icelines build/serve/deploy` aliases stay for one release with a deprecation warning, then drop. Fantasy read/product views are folded into the dashboard as `/fantasy`, `/api/v1/fantasy/gaps`, and `/api/v1/fantasy/simulate`; legacy `fantasy serve` mutation routes remain a separate local server boundary.

## URL structure

Every CLI command and every TUI screen has an HTML route and a JSON twin under `/api`. Coverage matrix is in the next section.

```
# ── Home + cross-surface ──────────────────────────────────────────────────
GET  /                           Home (today's notable stats + nav)
GET  /search?q=mcdavid           Cross-surface search (players, teams, goalies)
GET  /docs                       Embedded COMMANDS.md rendered as HTML
GET  /reports                    Reports overlay UI (toggle Tier-1 reports)

# ── Leaderboards + queries ─────────────────────────────────────────────────
GET  /leaders                    Leaderboard with form-based filter UI
GET  /leaders?sort=goals&season=20242025&top=20&pos=C&age_max=24&filter=hits>=200
GET  /leaders?type=playoff       Season-type toggle (regular | playoff)
GET  /rank                       Top-N by pace score (CLI `rank`)
GET  /class/:year                Draft class breakdown (CLI `class`)
GET  /goalies                    Goalie leaderboard
GET  /goalies?filter=save-pct>=0.92

# ── Player surfaces ────────────────────────────────────────────────────────
GET  /player/:id                 Player card — full 38-season career table
GET  /player/:id?seasons=5       Career arc capped to N seasons
GET  /player/:id?preset=two-way  Career-table preset (TUI `[/]` cycle parity)
GET  /player/:id/comps           Similarity / peers panel (CLI `peers`, `compare --similar`)
GET  /player/:id/mates           Linemate analysis (CLI `mates`)
GET  /player/:id/scouting        Full 8-section scouting report (CLI `scouting`)
GET  /player/:id/project         Rest-of-season projection (CLI `project`)
GET  /player/by-name/:name       Name-based redirect → /player/:id
GET  /goalie/:id                 Goalie detail card

# ── Compare + trade ────────────────────────────────────────────────────────
GET  /compare?p1=NAME&p2=NAME    Side-by-side comparison (CLI `compare`)
GET  /trade?out=NAME&in=NAME&team=ABBREV
                                 Trade evaluator — depth before/after (CLI `trade`)

# ── Team surfaces ──────────────────────────────────────────────────────────
GET  /team/:abbrev               Team depth chart with cross-team fit
GET  /depth                      League-wide team depth rankings (TUI `Depth`)

# ── Schedule + scores + playoffs ───────────────────────────────────────────
GET  /scores                     Tonight's games (and date picker)
GET  /scores/:date               Specific date — YYYY-MM-DD
GET  /schedule                   Weekly schedule grid
GET  /schedule/:team             Full-season schedule for one team
GET  /schedule/:teamA/:teamB     Head-to-head game log
GET  /playoffs                   Playoff bracket (current season)
GET  /playoffs/:season           Playoff bracket for a historical season
GET  /playoffs/series/:letter    One series detail (CLI `query playoff series`)
GET  /game/:id                   Game boxscore (TUI `GameDetail`)

# ── Transactions ───────────────────────────────────────────────────────────
GET  /transactions               League-wide transactions feed
GET  /transactions?team=SEA&kind=trade&since=2026-01-01
GET  /transactions?player=NAME

# ── Groups + watchlists (CRUD) ─────────────────────────────────────────────
GET  /groups                     List of named groups (CLI `group list`)
GET  /group/:name                Members + per-member pace stats
POST /group                      Create group (form-encoded)
PATCH /group/:name               Add/remove player
DELETE /group/:name               Remove group

# ── Games attended tracker ─────────────────────────────────────────────────
GET  /games                      Attended-games list (CLI `games show`)
POST /games                      Add a game by id
DELETE /games/:id                 Remove a game

# ── Fantasy (folded in — King.9) ───────────────────────────────────────────
GET  /fantasy                    Roster gaps and simulation scenarios
GET  /api/v1/fantasy/gaps        FantasyRosterGapView JSON
GET  /api/v1/fantasy/simulate    FantasySimulationView JSON

# ── Season picker + admin ──────────────────────────────────────────────────
GET  /seasons                       Season picker (TUI `y` parity)
GET  /admin/snapshots                Snapshot list (TUI overlay parity, CLI `snapshot list`)

# ── JSON API — every HTML page has a JSON twin under /api/v1/ ──────────────
GET  /api/v1/leaders?...             JSON envelope of player rows
GET  /api/v1/player/:id?...
GET  /api/v1/player/:id/comps
GET  /api/v1/player/:id/mates
GET  /api/v1/player/:id/scouting
GET  /api/v1/player/:id/project
GET  /api/v1/player/by-name/:name
GET  /api/v1/career/:id              Full bundled career
GET  /api/v1/team/:abbrev
GET  /api/v1/depth
GET  /api/v1/goalies?...
GET  /api/v1/goalie/:id
GET  /api/v1/compare?p1=&p2=
GET  /api/v1/trade?out=&in=
GET  /api/v1/class/:year
GET  /api/v1/rank
GET  /api/v1/scores/:date
GET  /api/v1/schedule
GET  /api/v1/schedule/:team
GET  /api/v1/schedule/:teamA/:teamB
GET  /api/v1/playoffs
GET  /api/v1/playoffs/:season
GET  /api/v1/playoffs/series/:letter
GET  /api/v1/game/:id
GET  /api/v1/transactions?...
GET  /api/v1/search?q=
GET  /api/v1/groups
GET  /api/v1/group/:name
POST /api/v1/group
PATCH /api/v1/group/:name
DELETE /api/v1/group/:name
GET  /api/v1/games
POST /api/v1/games
DELETE /api/v1/games/:id
GET  /api/v1/fantasy/gaps            FantasyRosterGapView JSON
GET  /api/v1/fantasy/simulate        FantasySimulationView JSON
GET  /api/v1/reports
PATCH /api/v1/reports                 Persists via Config::save_reports
GET  /api/v1/seasons                  { bundled, installed, active, active_type }
GET  /api/v1/active-season
PATCH /api/v1/active-season           Validates against bundled+installed before persist
GET  /api/v1/presets                  CareerTablePreset::ALL
GET  /api/v1/admin/snapshots
POST /api/v1/leaders/query            JSON body for filters that exceed URL length (King.10)

# ── Static assets (vendored via include_bytes!) ────────────────────────────
GET  /static/htmx.min.js             ~14 KB, vendored. Cache-Control: immutable
GET  /static/style.css               ~5 KB, vendored
GET  /static/icelines.svg            Logo
```

## Coverage matrix (CLI + TUI → web)

Every shippable surface accounted for. ✓ = covered in v1; D = deferred to a follow-up phase with rationale.

### CLI commands

| CLI command | Web route | Status |
|---|---|---|
| `query leaders` | `/leaders`, `/api/leaders` | ✓ King.2 |
| `query player NAME` | `/player/:id`, `/player/by-name/:name` | ✓ King.3 |
| `query compare A B` | `/compare?p1=A&p2=B`, `/api/compare` | ✓ King.3 |
| `query compare A --similar N` | `/player/:id/comps` | ✓ King.3 |
| `query goalies` | `/goalies`, `/api/goalies` | ✓ King.5 |
| `team ABBREV` | `/team/:abbrev` | ✓ King.4 |
| `rank` | `/rank` (or `/leaders?sort=pace-score`) | ✓ King.2 |
| `class YEAR` | `/class/:year` | ✓ King.4 |
| `peers PLAYER` | `/player/:id/comps` | ✓ King.3 |
| `compare A B` (top-level) | `/compare?p1=A&p2=B` | ✓ King.3 |
| `history PLAYER` | `/player/:id` (career table) | ✓ King.3 |
| `mates PLAYER` | `/player/:id/mates` | ✓ King.4 |
| `project PLAYER` | `/player/:id/project` | ✓ King.4 |
| `scouting PLAYER` | `/player/:id/scouting` | ✓ King.4 |
| `players` | `/leaders` (covers player filter set) | ✓ King.2 |
| `trade A for B` | `/trade?out=A&in=B`, `/api/trade` | ✓ King.4 |
| `tonight` | `/scores` | ✓ King.7 |
| `schedule` | `/schedule`, `/schedule/:team`, `/schedule/:a/:b` | ✓ King.7 |
| `transactions` | `/transactions` | ✓ King.8 |
| `group` (list/show/add/remove/create) | `/groups`, `/group/:name`, POST/PATCH/DELETE | ✓ King.8 |
| `games` (show/add/remove) | `/games`, POST/DELETE | ✓ King.8 |
| `scheme` (list/show) | CLI only in current dashboard parity matrix | partial |
| `fantasy gaps` / `fantasy simulate` | `/fantasy`, `/api/v1/fantasy/gaps`, `/api/v1/fantasy/simulate` | ✓ Selke/Ted Lindsay follow-up |
| `tui` / `dashboard` | (web is the alternative — not a route) | N/A |
| `docs` | `/docs` | ✓ King.8 |
| `export md`, `x` | `/api/*` covers JSON/CSV; markdown export per route via `?format=md` | ✓ King.8 |
| `fetch` | (write op, defer) | **D** — fetch stays CLI-only per non-goals |
| `snapshot` | `/admin/snapshots` (list-only in v1) | ✓ King.10 |
| `build` / `site build` | (mkdocs path, separate command) | N/A |
| `serve` (mkdocs, renamed) | `icelines site serve` | N/A |
| `deploy` / `site deploy` | (mkdocs path) | N/A |

### TUI screens

| TUI screen / overlay | Web route | Status |
|---|---|---|
| `Home` | `/` | ✓ King.1 |
| `Team(abbrev)` | `/team/:abbrev` | ✓ King.4 |
| `PlayerById(pid)` | `/player/:id` | ✓ King.3 |
| `CompsById(pid)` | `/player/:id/comps` | ✓ King.3 |
| `Depth` (league-wide) | `/depth` | ✓ King.4 |
| `DepthTeam(team)` | `/team/:abbrev` | ✓ King.4 |
| `Search` | `/search` | ✓ King.8 |
| `Tonight` | `/scores` | ✓ King.7 |
| `Projections` | `/projections` (catch-all) or per-player | ✓ King.4 |
| `Queries` (interactive builder) | `/leaders` with form | ✓ King.2 |
| `Groups`, `GroupDetail` | `/groups`, `/group/:name` | ✓ King.8 |
| `Fetch` overlay | (defer — write op) | **D** |
| `Help` | `/docs` | ✓ King.8 |
| `Schedule`, `ScheduleTeam`, `ScheduleMatchup` | `/schedule`, `/schedule/:team`, `/schedule/:a/:b` | ✓ King.7 |
| `Playoffs`, `SeriesDetail`, `GameDetail` | `/playoffs`, `/playoffs/series/:letter`, `/game/:id` | ✓ King.7 |
| `Goalies`, `GoalieDetailById` | `/goalies`, `/goalie/:id` | ✓ King.5 |
| `Transactions` | `/transactions` | ✓ King.8 |
| Reports overlay (R key) | `/reports`, PATCH `/api/reports` | ✓ King.6 |
| Season picker (y key) | `/seasons`, PATCH `/api/active-season` | ✓ King.6 |
| Season-type toggle (Shift+P) | `?type=regular\|playoff` query param everywhere | ✓ King.2 |
| Career-table preset cycle (`[/]`) | `?preset=NAME` on `/player/:id` | ✓ King.3 |
| Sort picker (`/` key) | inline form on `/leaders` | ✓ King.2 |
| Section toggle (`o`) | (HTML lists are always-expanded; collapse-on-click is HTMX nicety) | ✓ King.2 |

## URL & API contract

### Routing prefix
All JSON routes mount under `/api/v1/`. `schema_version` is per-envelope so a route-specific bump never forces a global flip; `/api/v2/` is reserved for the day a breaking change can't be expressed additively.

### Response envelope

```json
{
  "schema_version": 1,
  "route": "leaders",
  "data": [ ... ],
  "meta": {
    "season": "20252026",
    "season_type": "regular",
    "active_filters": ["g>=50"],
    "implicit_filters": ["gp>=10"],
    "total": 1487,
    "limit": 50,
    "offset": 0,
    "next_offset": 50
  }
}
```

- `schema_version`: per-envelope integer. Additive changes do NOT bump it. Renames or removals DO bump it (and that route alone moves to `/api/v2/...` next major release). **Sunset policy**: when route X gets a `/api/v2/`, the `/api/v1/X` stays one full release (deprecated, `Deprecation: true` HTTP header), removed in the release after. So embedders pin one version at a time, not forever.
- `route`: discriminator so embedders can pin per-shape (`"leaders"`, `"player"`, `"goalies"`).
- `data`: payload (array or object).
- `meta`: pagination + active-filter echo + (season, season_type) self-description + `implicit_filters` (e.g. default `gp_min` floor on rate-stat sorts).

### JSON-key contract beyond stats
KEEL-B1 covers stat keys (use `StatId::cli_key`). For bio / identity / structural keys, the contract is `snake_case` always: `nhl_id` not `nhlId`, `team_abbrev` not `team`, `season_type` not `seasonType`. Frozen via `l1_api_keys_are_snake_case`.

### Pagination
Every list route (`/api/leaders`, `/api/goalies`, `/api/transactions`, `/api/depth`, `/api/class/:year`, `/api/groups`, `/api/games`) accepts `?limit=N` (default 50, max 500) and `?offset=N` (default 0). Response `meta.total` is the unpaginated count; `meta.next_offset` is null at the end. No Link headers — bad fit for JSON consumers.

### Error envelope

```json
{
  "schema_version": 1,
  "error": {
    "kind": "BadFilter",
    "message": "filter parse error at column 14",
    "hint": "did you mean '>=' instead of '=>'?",
    "details": { "filter": "g=>50", "column": 14 },
    "request_id": "01HZQ..."
  }
}
```

Error kinds: `UnknownStat`, `UnknownSort`, `UnknownSeason`, `UnknownPlayer`, `BadFilter`, `BadParam`, `ConflictingParams`, `NotFound`, `RateLimited`, `Internal`, `CorruptSnapshot`. HTTP status: 400 for client errors, 404 for missing resources, 421 for DNS-rebinding rejects, 500 for `Internal`. `request_id` is a ULID generated via the `ulid` crate per request, attached as both an HTTP response header (`X-Request-Id: ...`) AND inside the error envelope; logged at WARN+ via `tracing` so server-side correlation works. Clients include it in bug reports.

`WebError` is a `thiserror` enum implementing `IntoResponse` — single source of truth for the kind→status mapping. PATCH/POST bodies deserialize with `serde(deny_unknown_fields)`.

### Filter grammar over URL

Same grammar as `--filter` (Filter.OR boolean: AND/OR/NOT/parens). Concrete encoding rules:

| Want to express | URL form |
|---|---|
| `g>=50` | `?filter=g%3E%3D50` |
| `g>=50 OR a>=50` | `?filter=g%3E%3D50%20OR%20a%3E%3D50` |
| `(g>=30 AND a>=30) OR p>=80` | encode parens as `%28` / `%29` |
| Multiple filters AND'd | `?filter=g>=50&filter=hits>=200` (both kept) |
| Empty filter | `?filter=` — ignored, not an error |

**Critical**: axum default `Query<HashMap<String,String>>` collapses repeated keys → silent data loss. Handlers MUST use `Query<Vec<(String, String)>>` or a custom `Filters` extractor. Frozen golden test `l1_filter_url_repeats_anded`.

`+` is forbidden in filter expressions today (no stat key contains `+`); reserved for future safety. The `+/-` plus-minus alias is rewritten to `plus-minus` BEFORE URL parsing.

For complex filters exceeding ~2 KB URL length: `POST /api/v1/leaders/query` accepts a JSON body. King.2 ships GET-only; POST falls in King.10. URL length cap at 2 KB; over-long filters reject with 414.

### Per-route parameter contract

Every stat-loading route accepts these query params:

| Param | Routes | Default |
|---|---|---|
| `?season=YYYYZZZZ` | `/leaders`, `/player`, `/goalies`, `/compare`, `/rank`, `/class/:year`, `/team/:abbrev`, `/depth` | active-season from config |
| `?type=regular\|playoff` | same set | `regular` |
| `?seasons=N` (1-38 aggregate) | `/leaders`, `/player`, `/compare`, `/rank` | 1 |
| `?filter=...` (repeatable) | `/leaders`, `/goalies`, `/player`, `/compare`, `/class/:year`, `/depth` | none |
| `?sort=cli_key\|alias` | `/leaders`, `/goalies`, `/rank`, `/depth` | `pts-pace` (skaters), `save-pct` (goalies) |
| `?top=N` | `/leaders`, `/goalies`, `/rank` | 20 |
| `?limit=N`, `?offset=N` | all list routes | 50 / 0 |
| `?preset=NAME` | `/player/:id` | `default` (position-aware via `StatId::default_in_career_table`) |
| `?rank_by=cli_key` | `/player/:id` | `pts-pace` |
| `?include_below_threshold=true` | `/leaders`, `/goalies`, `/rank` | false |

**Conflicts**:
- `?seasons=N` AND `?season=YYYYZZZZ` → 400 `kind: "ConflictingParams"`. CLI parity.
- `?type=playoff` on a season without bundled playoff data → 400 `kind: "UnknownSeason"`, hint listing seasons with playoff data.
- Unknown `?sort=` → 400 `kind: "UnknownSort"`, hint with closest 3 keys by edit distance.
- Unknown `?preset=` → 400 with the canonical preset list inline.

**Goalie filter rewrite**: `/api/v1/goalies?filter=...` runs `goalie_filter_rewrite_expr` BEFORE `parse_filter`. `?filter=gp>=20` rewrites to `goalie-games>=20`. L0 fence `l0_api_goalies_filter_rewrite_gp_to_goalie_games`.

**Default rate-stat floor**: when `?sort=` is a per-game rate (`points-per-game`, `goals-per-game`, `save-pct`, `goals-against-avg`), an implicit `gp_min` floor applies (skaters: 10, goalies: 5) UNLESS `?include_below_threshold=true`. Surfaced in `meta.implicit_filters`. Avoids the "4-GP call-up at the top" foot-gun.

### CORS + DNS rebinding
- Default bind `127.0.0.1`: no CORS headers; `Host:` must match `localhost`, `127.0.0.1`, or the bind addr — others reject with 421.
- `--bind 0.0.0.0`: prints `WARNING: LAN mode — no auth, no TLS. Anyone on your network can read your data.` `--cors-origin URL` flag for explicit allowlist.

## HTML rendering + UX patterns

### Templates
`askama` (compile-time, type-safe). Templates live in `icelines-web/templates/*.html`. Build-time check ensures every route's `Template` impl exists.

### Partial-fragment routes (HTMX)
HTMX-driven swaps return HTML fragments. Routes accept `?partial=1` (or `?partial=NAME` for named regions):

- `GET /leaders?partial=1` → returns `<tbody>...</tbody>` only
- `GET /player/:id?partial=tab&tab=career&preset=two-way` → returns the career-table panel only
- `GET /reports?partial=1` → returns the form section only

Frozen rule: `?partial=*` returns ONLY the targeted fragment; bare route returns full page with nav + footer. Idempotent: same target hit twice produces byte-identical HTML (`l1_htmx_swap_idempotent`).

### CSS class contract
Color contract from glass.md propagates as fixed classes:

| Class | Hex | Usage |
|---|---|---|
| `.fit-elite` | `#2e7d32` (green) | Top-tier player fit |
| `.fit-solid` | `#1565c0` (blue) | Solid contributor |
| `.fit-fringe` | `#f9a825` (yellow) | Roster bubble |
| `.fit-buried` | `#b71c1c` (red) | Below replacement |
| `.score-leading` | `#2e7d32` | Game-state leading |
| `.score-trailing` | `#b71c1c` | Game-state trailing |
| `.score-tied` | `#616161` | Tied |

Every color-encoded fit class also carries text or icon ("Elite" / "Solid" / "Fringe" / "Buried"). No information conveyed by color alone (a11y). L1 fence `l1_html_no_color_only_encoding`.

### Active (season, season_type) header
Every page renders a sticky header showing `2025-26 · Regular` with a clickable link to `/seasons`. Without this, time-traveling via PATCH is silent. L1 fence `l1_html_each_route_has_active_season_header`.

### Sort picker UI
HTMX-driven combobox with search-as-you-type, mirroring TUI `/`. Server filters `StatId::all_keys()` by substring; renders `<li>` per match; click sets the form's `?sort=` value. Falls back to `<select>` with all options when JS disabled.

### Career-table preset selector
Visible tab strip on `/player/:id` showing all `CareerTablePreset::ALL`. Active preset highlighted via `.preset-active`. Click swaps via HTMX (`?partial=tab&preset=NAME`) without page reload. `GET /api/v1/presets` enumerates valid names.

### Loading states
HTMX `hx-indicator=".spinner"` on every interactive form. Skeleton row pattern for `/player/:id` first-open (lazy career fan-out). Home page shows "loading…" footer until repo is ready.

### Mobile / narrow viewport
- `<meta name="viewport" content="width=device-width, initial-scale=1">` on every page.
- One CSS breakpoint at `600px`: dual-pane layouts collapse to single-column scroll. Tables get horizontal scroll inside their container, never the page.
- Logo + nav collapse to a hamburger menu under 480px (CSS-only `<details>`, no JS).

### Accessibility
- Semantic HTML: real `<table>` for stat tables, `<nav>` for navigation, `<main>` for content.
- ARIA labels on color-only encodings (covered above).
- Keyboard nav: focus rings preserved; skip-to-content link.
- `prefers-reduced-motion` respected on HTMX swaps.
- King.10 adds an `axe-core`-baseline L1 fence per page.

### Empty + error states
- Player/goalie/team detail for unknown ID: 404 page with a search box and "did you mean…" top-3 results.
- Season-route requests outside the bundle: 400 `kind: "UnknownSeason"`, hint listing bundled range.
- Empty leaderboards: "No matches — try removing one of: g>=50, hits>=200" with each filter as a clickable remove link.

## Reuse map

| Web surface | Reuses |
|---|---|
| `/leaders`, `/api/leaders` | `StatsRepository::skaters` + `PlayerFilter::apply_views` + `parse_filter_expr` (already shipped) |
| `/player/:id` | `load_player_career_into_repo` (UX.1, already shipped) |
| `/team/:abbrev` | `TeamView` + cross-team fit (already shipped via TUI) |
| `/goalies` | `query::run_goalies` data path |
| `/scores`, `/schedule`, `/playoffs` | Existing `tonight`, `schedule`, `playoffs` data layers |
| `/reports` | `Config::reports` + `ReportToggles::set` + `Config::save_reports` (Reports.1) |
| `/docs` | `include_str!("../../COMMANDS.md")` rendered through a markdown crate |
| `/fantasy`, `/api/v1/fantasy/gaps`, `/api/v1/fantasy/simulate` | Fantasy read/product ViewModels in the main dashboard; legacy `fantasy serve` mutation routes stay separate |

The data-loading code stays in `icelines-fetch`; the projection logic stays in `icelines-core`; the web surface is a thin handler layer in `icelines-web/src/`.

## Concurrency & state

### State shape — split, not shared
The TUI's `App` holds `StatsRepository` which is `!Send + !Sync` (intentionally — Phase Hart). It cannot move to a multi-threaded axum runtime. The web server therefore does NOT share `App` with a running TUI process — they run in different processes; the only shared state is `~/.icelines/config.toml` and `~/.icelines/icelines.db`.

```rust
pub struct WebState {
    repo: Arc<RwLock<StatsRepository>>,
    config: Arc<RwLock<Config>>,
    fantasy_db: Arc<FantasyDb>,        // already Send + Sync
    group_db: Arc<GroupDb>,
    cache: Arc<ResponseCache>,         // moka, internally locked
}
```

`StatsRepository` becomes `Send + Sync` for the web variant by wrapping `LruCache` access in interior locking. Phase Hart's `!Send` bound was a CLI/TUI ergonomic; web runs on tokio's multi-threaded runtime and needs real send-ability. **Fallback** if Send-conversion proves invasive: `LocalSet` + `Rc<RefCell<App>>` on a single-threaded runtime — slower but compiles today. King.1 plan must measure and pick.

### Lock discipline
- **Reads** take `repo.read().await` — concurrent reads do not block each other.
- **Lazy career loads** (UX.1 fan-out) load into a *temp* `StatsRepository`, then take `repo.write().await` only for the brief LRU swap. The 50ms fan-out never blocks readers.
- **Config writes** (PATCH `/api/v1/reports`, PATCH `/api/v1/active-season`) hold the config write lock for the file flush only (single ms), not the response render.
- **No mutex held across `.await`** — use `tokio::sync::RwLock` (parking_lot doesn't support async fairness).
- **`tokio::sync::RwLock` is NOT reentrant** — a handler holding a read lock that calls a helper which also takes the read lock can deadlock when a pending writer is queued. Either pass the guard down or release+reacquire. Audit all helper functions.
- **`PlayerView<'_>` lifetime**: handlers must not return a view; derive view inside the lock scope, render to HTML/JSON, drop, release lock.
- **No `unwrap()` / `expect()` in `icelines-web` handler bodies** — every `Result` flows through `WebError`. Library-code unwrap ban from `forge.md` applies to the new crate. Tests are the only exception.

### Response cache
Per-(season, season_type, route, sort, filter-hash, type, preset) memoization with 30-second TTL. Invalidated on any `PATCH /api/v1/reports` or `PATCH /api/v1/active-season`. Backed by `moka::sync::Cache`.

| Route | Cache key | Reason |
|---|---|---|
| `/api/v1/leaders` | (season, type, sort, filter_hash, top, preset) | Most expensive — full skater scan + sort |
| `/api/v1/goalies` | (season, type, sort, filter_hash, top) | Same |
| `/api/v1/team/:abbrev` | (abbrev, season, type) | Cross-team fit recompute |
| `/api/v1/player/:id` | NOT cached | First open is uncached fan-out; subsequent are cheap |
| `/api/v1/scores/:date` | (date), 60s TTL | Live data |
| `/admin/snapshots` | NOT cached | Filesystem walk |

### Compression + caching headers
- All `/api/v1/*`: compact JSON (`serde_json::to_string`, never pretty), `tower-http::compression` gzip.
- All `/static/*`: `Cache-Control: public, max-age=31536000, immutable`, ETag from binary version (`env!("CARGO_PKG_VERSION")`); `Content-Type` per asset (`text/css`, `application/javascript`, `image/svg+xml`).
- HTML: `Cache-Control: no-store` (small files, dynamic).

### Fantasy DB pool
King.9's fold-in MUST reuse the existing `FantasyDb::open()` pool, not create a second pool. One process = one DB pool. Verified by L1 asserting `Arc::strong_count` invariants.

### Cold-start budget
`icelines serve` to listening socket: <500ms target. Bundle decode is once at startup; no per-request bundle reads. L0 fence `l0_serve_cold_start_under_500ms`.

### `--no-cache` mode
Bypasses the response cache; does NOT bypass `SnapshotMeta::integrity` on disk reads (those always run). Useful for development; not for production "headless" use because every request pays full cost.

## Migration mechanics

### `serve` rename — staged migration

Three steps, ordered for compileability:

1. **Pre-King.1**: rename `Commands::Build/Serve/Deploy` → group under `Commands::Site(SiteSubcommand)`. CLI surfaces `icelines site {build,serve,deploy}`.
2. **King.1**: add new `Commands::Serve` (web dashboard). Hidden top-level deprecated aliases `Commands::Build`, `Commands::DeprecatedServe` (mkdocs path), `Commands::Deploy` print to **stderr**:
   ```
   WARNING: 'icelines build' moved to 'icelines site build' in v0.13.
            The old alias is removed in v0.14. Run 'icelines site build' instead.
   ```
   Then dispatch to the new path. stderr (not stdout) so pipeline consumers of `icelines build` aren't broken.
3. **v0.14**: deprecated aliases removed.

### `fantasy serve` boundary
As-built after the Selke/Ted Lindsay follow-up, `icelines serve` owns the parity
read/product views (`/fantasy`, `/api/v1/fantasy/gaps`,
`/api/v1/fantasy/simulate`). `icelines fantasy serve --port N` remains the
legacy local fantasy-server workflow for older standings/team/trade mutation
routes documented in `fantasy-leagues.md`.

```
INFO: fantasy read/product views are available from `icelines serve`.
      Legacy fantasy mutation routes remain on `icelines fantasy serve`.
```

### Browser auto-open
1. Print URL to stdout BEFORE attempting to open (user always has the URL).
2. Honor `BROWSER` env var.
3. Swallow open errors silently — never fail `serve` because a browser couldn't launch (headless, WSL).
4. `--no-open` skips 2-3.

L0 fences: `l0_serve_prints_url_before_open`, `l0_serve_continues_on_open_failure`.

### Port collision policy
Fail loud, no auto-bump. `error: port 8000 already in use\nhint: try '--port 8001'`. Auto-bumping silently changes the printed URL and breaks scripts.

### `--bind` vs `--port` precedence
- `--port N` is shorthand for `--bind 127.0.0.1:N`.
- `--bind ADDR[:PORT]` overrides. `--bind` without port → `--port` fills it. Both with port + mismatch → 400 with hint.
- Default: `127.0.0.1:8000`.

### Exit codes
- `0`: clean shutdown (Ctrl-C, signal).
- `1`: startup error (port in use, bind error, config parse).
- `2`: invalid CLI args (clap-handled).

### `--reload` → `--no-cache`
Renamed. New name describes what it does (skip the response cache); old name confused users coming from uvicorn/django. Old name kept as a hidden alias that prints a deprecation note.

### COMMANDS.md fence
L1 test `l1_commands_md_lists_every_web_route`: walks the axum `Router`, reads `COMMANDS.md` "Web routes" section, asserts each path appears (ignoring trailing-slash and `:id`-vs-`:player_id` differences). Mirrors the StatId catalog grep precedent.

### Per-sub-phase DoD
Every King.N plan file ends with:
- [ ] cargo build / clippy / fmt clean
- [ ] L0 + L1 tests for new routes (counts per Testing strategy)
- [ ] COMMANDS.md updated; `l1_commands_md_lists_every_web_route` passes
- [ ] `--help` long_about updated for any new flag
- [ ] Concurrent swarm test passes (after King.2)
- [ ] One screenshot in the plan file (or "TUI parity confirmed")

## Data integrity contracts

### PATCH `/api/v1/active-season` validation
Handler:
1. Parse season string; reject unparseable with 400 `kind: "BadParam"`.
2. Verify season is in `BUNDLED_SEASONS` (compile-time list) OR is an installed snapshot under `~/.icelines/snapshots/` with passing integrity hash.
3. Only THEN write via `Config::save_active_season()`.

Failure modes: unbundled+uninstalled → 400 `UnknownSeason` (hint with bundled range); installed but integrity check fails → 400 `CorruptSnapshot` (hint to re-fetch).

L1 fences: `l1_patch_active_season_rejects_unbundled`, `l1_patch_active_season_round_trips_through_config`.

### `/api/v1/seasons` source of truth

```json
{
  "schema_version": 1, "route": "seasons",
  "data": {
    "bundled": ["19871988", "...", "20252026"],
    "installed": [{ "season": "19891990", "integrity_ok": true, "fetched_at": "..." }],
    "active": "20252026", "active_type": "regular"
  }
}
```

Bundled from `BUNDLED_SEASONS`. Installed from a `SnapshotStore::list()` walk that includes integrity verification per entry. Failed-integrity seasons listed with `integrity_ok: false` (visible to user, not silently dropped).

### PATCH `/api/v1/reports` write path
ALL config writes go through `Config::save_reports()`. Handlers MUST NOT hand-roll TOML. L1 fence `l1_reports_round_trip_tui_to_web_to_tui`: PATCH from web, reload TUI, assert state changed; revert via TUI keypress, GET from web, assert reverted.

### Provenance fields
Routes that mix data sources include `source` per row:
- `/api/v1/transactions` rows: `"source": "espn" | "nhl-bundle"`.
- `/api/v1/seasons.installed[i]`: `"source": "user-fetch"`.

### Self-describing player rows
Every player row in any list response carries: `nhl_id`, `season` (`"20252026"`), `season_type` (`"regular"|"playoff"`), `team` (current as of season-end). Bookmarked URLs return self-describing JSON.

### Peer-pool definition surfaced
`/api/v1/player/:id/comps` and `/api/v1/player/:id/peers` include a `peer_pool` block:
```json
"peer_pool": {
  "position_band": "C",
  "age_min": 20, "age_max": 24,
  "gp_min": 20,
  "matched_count": 47
}
```

A user comparing a 22-year-old C to "peers" knows the pool is 20-24 year-old centers with ≥20 GP.

## Testing strategy

### Tier discipline
- **L0**: unit tests inside source files; axum handlers via `tower::ServiceExt::oneshot` against an in-memory router. <1ms each.
- **L1**: `icelines-cli/tests/web_*.rs` (or `icelines-web/tests/`). Real axum on `127.0.0.1:0` (kernel-assigned port), hit with `reqwest`. No live network.
- **L2**: persona scenarios in `persona_wave5.rs`. Drive query patterns through the JSON API; assert response equivalence with `query leaders --json`.

### Test-count floor per sub-phase

| Sub-phase | L0 floor | L1 floor | Cumulative wave5 |
|---|---|---|---|
| King.1 | 10 | 5 | 0 |
| King.2 | 30 | 15 | 30 |
| King.3 | 25 | 10 | 60 |
| King.4 | 20 | 10 | 90 |
| King.5 | 10 | 5 | 110 |
| King.6 | 15 | 10 | 140 |
| King.7 | 25 | 10 | 180 |
| King.8 | 30 | 15 | 220 |
| King.9 | 15 | 8 | 240 |
| King.10 | 10 | 5 | **≥260** |

`260` is the cumulative King.10 floor (per the table); a sub-phase can exceed its row's floor but cannot ship under. Add tests rather than skim.

### Snapshot tooling
`insta` (already used for ratatui buffers). HTML snapshots in `icelines-web/tests/snapshots/<route>/<scenario>.snap`. Reviewed via `cargo insta review`. HTMX fragments snapshotted separately from full pages.

### Required fences (each is BLOCKING per CHECKPOINT)

1. **`l0_filter_url_parity`** — parameterized golden: each filter expression parsed twice (CLI string and URL-decoded form), assert resulting `FilterExpr` AST equal.
2. **`l1_keel_b1_cross_surface_json_keys`** — fire `query leaders --json` and `GET /api/v1/leaders` against same fixture; assert identical key sets. Mirrors L.5.6 precedent.
3. **`l1_filter_url_repeats_anded`** — `?filter=g>=50&filter=a>=20` applies both (no silent drop).
4. **`l1_concurrent_request_swarm`** — `tokio::join!` 16 concurrent `/api/v1/leaders` + 4 `/player/:id` cold opens; no deadlock + correct results. Wrap the `join!` in `tokio::time::timeout(Duration::from_secs(10), ...)` so a true deadlock fails the test as a Timeout error rather than hanging out to CI's outer timeout.
5. **`l1_patch_isolates_via_tempdir`** — every PATCH/POST/DELETE uses `Config::with_root(tempdir)`. Stomping the dev's real config = CI-failing.
6. **`l0_schema_version_present_on_every_api_response`** — walk router, fire each GET, assert envelope has `schema_version: 1`.
7. **`l1_reports_round_trip_tui_to_web_to_tui`** — described above.
8. **`l1_commands_md_lists_every_web_route`** — described in Migration mechanics.
9. **`l1_html_each_route_has_active_season_header`** — described in UX patterns.
10. **`l1_html_no_color_only_encoding`** — described in UX patterns (a11y).
11. **`l1_api_keys_are_snake_case`** — non-stat keys (bio/identity) follow `snake_case` contract.
12. **`l1_htmx_swap_idempotent`** — same `?partial=` target hit twice produces byte-identical HTML.
13. **`l0_peer_pool_block_present_on_comps_response`** — `/api/v1/player/:id/comps` and `/api/v1/player/:id/peers` always include the `peer_pool` block (transparency contract).

### Test seam: `Config::with_root(tempdir)`
New constructor on `Config` that points all reads/writes at a tempdir. Every PATCH-touching test uses it. `XDG_CONFIG_HOME` env override is the fallback.

### Bundled fixture cost
L1 tests share a single `OnceLock<WebState>` per binary via `tests/common/mod.rs`. Loads ~20 canonical players into a stripped repo. Don't load all 38 seasons per test — that's 12+ s of overhead per binary.

### CI gate
Web tests behind `--features web-tests` workspace feature. Default `cargo test` runs them; matrix includes `--no-default-features` row. Bind `127.0.0.1:0`, never hardcoded port.

### Required L1 patterns
- **Diacritics**: `GET /player/by-name/Slafkovsk%C3%BD` round-trips through resolver.
- **Historical names**: `GET /player/by-name/Wayne%20Gretzky` resolves without active season; bounded ≤500ms.
- **Markdown export parity**: `GET /api/v1/leaders?format=md` byte-equal to `icelines export md leaders` for the same args.
- **CRUD round-trips**: each write endpoint has POST→GET, PATCH→GET, DELETE→GET sequences.
- **Deprecation warnings**: `icelines build` (after rename) writes deprecation to stderr but exits 0.

## Phase plan (sub-phases)

The full implementation is broken into sub-phases. Each is shippable independently:

- **King.1** — `icelines serve` skeleton: top-level command, axum boots on `:8000`, mkdocs `serve` renamed to `site serve` with one-release alias, single `/` page, vendored static-assets pipeline (`include_bytes!` for CSS / HTMX / logo). Auto-open browser. `--no-open` opt-out.
- **King.2** — Leaderboards. `/leaders` HTML + `/api/leaders` JSON. Filter form (full grammar incl. AND/OR/NOT). Pagination. `?sort=`, `?type=` (season-type toggle), `?preset=` parity. `/rank` route. Sort picker UI parity with TUI `/`.
- **King.3** — Player surfaces. `/player/:id` HTML + JSON with career table + preset cycle. `/player/by-name/:name` redirect. `/player/:id/comps` (peers / similarity). `/compare?p1=&p2=`. Goalie detail at `/goalie/:id`.
- **King.4** — Team + advanced player. `/team/:abbrev`, `/depth` league-wide. `/class/:year`. `/player/:id/{mates,project,scouting}`. `/trade?out=&in=`.
- **King.5** — Goalie leaderboard. `/goalies` HTML + `/api/goalies` JSON. (Detail card already in King.3.)
- **King.6** — Reports overlay + season picker. `/reports` form + PATCH `/api/reports`. `/seasons` + PATCH `/api/active-season`. Persists to `~/.icelines/config.toml`.
- **King.7** — Live-data pages. `/scores`, `/scores/:date`, `/schedule`, `/schedule/:team`, `/schedule/:a/:b`. `/playoffs`, `/playoffs/series/:letter`, `/game/:id` boxscore.
- **King.8** — Transactions + search + docs + groups + games. `/transactions` (with kind/player/team/since filters). `/search`. `/docs` (markdown→HTML). `/groups` + `/group/:name` (CRUD). `/games` attended-tracker (CRUD).
- **King.9 / Selke-Ted follow-up** — Fantasy read/product routes folded in under the main dashboard: `/fantasy`, `/api/v1/fantasy/gaps`, and `/api/v1/fantasy/simulate`. Legacy `fantasy serve` mutation routes remain separate.
- **King.10** — Hardening + closeout. `--bind 0.0.0.0` LAN mode + warning banner. `/admin/snapshots` (read-only list). Persona Wave 5 (50+ web scenarios). Performance review. Old `serve`/`build`/`deploy` deprecation warnings.

Each sub-phase has its own plan file (`design/plans/2026-05-XX-phaseKingClancy-N-<topic>.md`).

## Open questions for review

(All seven original opens were resolved 2026-05-03 — see the **Decisions locked** table at the top. New questions for reviewers may surface during the role review and will land here.)

## Risks (residual, after contract sections)

The forge / pace / wire / edge / glass / keel / bench / scout+tape review pass converted most original risks into hard contracts (see Concurrency & state, URL & API contract, Migration mechanics, Data integrity contracts, Testing strategy). Residual risks worth tracking through implementation:

- **`StatsRepository` Send-conversion scope**: making the repo `Send + Sync` may touch more than the LRU layer. Fallback (LocalSet + Rc) compiles today but caps throughput. King.1 plan must measure and decide.
- **Compile-time regression**: adding axum + tower-http + askama + moka likely adds 30–60 s to a clean build on top of an already 56 MB binary. King.1 records the delta and gates if intolerable.
- **TUI/web state-bleed**: `~/.icelines/config.toml` is shared. Reports overlay change on web → TUI sees it next launch. Documented as a feature; the round-trip fence (`l1_reports_round_trip_tui_to_web_to_tui`) catches divergence.
- **Scope creep**: each sub-phase must stay tightly scoped. King.10 is hardening only — admin write surfaces stay deferred.
- **`broadcast` role coverage**: `.roles/broadcast.md` (created 2026-05-04) owns web-specific concerns from King.2 onward.

## Out of scope (future King Clancy successors)

- WebSockets for live game updates (would need `axum::extract::ws`)
- User accounts + multi-user fantasy leagues (auth + persistence)
- Public deployment (TLS, rate limiting, abuse protection)
- Mobile-optimized layout
- PWA / offline service worker

These are real ideas but each is its own future trophy phase.
