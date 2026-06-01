# Concept of Operations

## Scope

Repo or feature: `icelines` — operational scenarios for the workbench platform
across CLI, TUI, Web, and export/site renderers. This CONOPS turns the MISSION's
users and First Validation Scenarios into concrete operating flows with normal
paths, failure/degraded paths, outputs, and handoffs. Scenario IDs (`CON-###`)
are referenced by `REQUIREMENTS.md` and `TRACE.md`. See `MISSION.md` for intent
and the six platform contracts. Each scenario names `VAL-###` validation
evidence: where an automated fence already exists it is cited inline (e.g.
CON-004); otherwise the `VAL-###` is demonstration/inspection evidence to be
defined in `VALIDATION.md`, **not** an existing test (BENCH).

## Actors

| Actor | Responsibility | Needs |
|---|---|---|
| Power user / analyst | Drive analysis fast on the TUI workbench by command bar | Keyboard speed, precise filters, season time-travel, honest data |
| Stats blogger / social poster | Produce citable historical stat/streak artifacts | Perspective queries, completeness disclosure, reproducible export |
| New / casual user | Browse via the web dashboard with no setup | Discoverable UI, bookmarkable URL state, always-visible context |
| Fantasy manager | Make roster/trade/poach decisions | Gaps, simulation, poach board; offline, repeatable |
| Data tracker | Acquire and verify depth data | Easy `fetch`/`data install`/`snapshot`; honest capability matrix |
| Coach / analyst | Consume prepared evidence for roster, matchup, game-day, and postgame decisions | Fast canonical analytics, provenance/freshness disclosure, no black-box coaching authority |
| Operator / maintainer | Build, run, and ship the binary | Standalone build, feature-gated lean builds, green gates |
| Future agent | Resume work from intent + trace | Readable `docs/vtrace/`, truthful status |

## Scenarios

### CON-001: Power-user workbench reshape (TUI)

Trigger: analyst wants to answer a precise question and rearrange context fast.

Inputs: `icelines` (default workbench); cmdbar `:stats`; filter
`pos=C age<=23 AND p.last20g>=20`; pane cycle; `y` season picker.

Normal path: workbench launches (activity rail + scores ribbon + left/right
panes + workspace + cmdbar). Operator swaps the workspace to Stats, applies the
filter through `icelines-query`, cycles left/right panes via
`WORKBENCH_PANE_BINDINGS`, time-travels with `y`, and opens a player card — all
by keyboard, in interactive time (a **target to be measured**, not a guaranteed
bound). Active `(season, season_type)` stays visible.

Failure or degraded path: bad filter → typed `ParseError` with span + hint, not
a crash; the example `p.last20g>=20` is a **sliding-window atom** that
`needs_provider()` (boxscore manifest) — if that data isn't cached the query
degrades with an explicit "needs boxscore data" notice + fetch hint, never a
silent empty result; requested historical season not installed → dim
"[not installed]" with a `data install` hint; LRU pressure after multi-season
travel stays bounded (cap 80).

Outputs: rendered `LeadersView` / `PlayerCardView`; updated pane state.

Handoffs: same filter feeds CON-004 (parity) and the export in CON-002.

Validation evidence: VAL-001 (`VALIDATION.md`).

### CON-002: Stat-in-perspective post (Reports / social)

Trigger: blogger spots a stat or streak and wants to post it with context.

Inputs: a historical query — e.g. Art Ross `g.streak>=15 EVER`, or
`streaks`/`records` — followed by `export md` / `--json`.

Normal path: the query returns the streak **with its ranking/context**. Streaks
are **intra-season, axis-typed**: a run does not span the offseason, and `EVER`
returns the *best intra-season* streak across seasons — not one continuous
cross-season run; trade continuity is *within* a season across `team_stints`. The
answer correctly skips the **2004-05 lockout**, honors the October
`CURRENT_SEASON` rollover, resolves name collisions (the Sebastian Aho problem)
via team context, and **discloses data completeness** over the range — including
the **per-source asymmetry** (only ~5 modern seasons carry full Tier-1 detail;
33 are skeleton; some domains are snapshot-only). `export md` / `--json` writes a
reproducible artifact (fixed fixture + clock).

Failure or degraded path: range includes skeleton or snapshot-only seasons →
result is labeled `partial`/`stale` with the affected span and source, never
silently presented as complete; an **in-progress (active) streak** in the live
season is marked "ongoing" with its current length, not truncated or presented
as final; ambiguous name → disambiguation prompt or both candidates, not an
arbitrary pick.

Outputs: a citable markdown/JSON artifact with ranking + completeness disclosure.

Handoffs: artifact is the blogger's post; becomes reproducibility evidence.

Validation evidence: VAL-002 (`VALIDATION.md`); risks in `MISSION.md` (EDGE rows).

### CON-003: New-user web browse (Web)

Trigger: a first-time visitor opens the web dashboard cold, no CLI knowledge.

Inputs: `icelines serve` (prints the URL before auto-open, handles port
collision, warns on `0.0.0.0` bind), then `GET /dashboard` and navigation to
`/leaders`, `/scores`, `/player/:id`, `/playoffs`.

Normal path: the server-rendered no-JS shell loads (grouped catalog rail, bound
experience tabs, scores ribbon, left/right pane chips, workspace). The visitor
finds league leaders, tonight's slate, a player card, and the bracket; every page
shows active `(season, season_type)`; the workspace+panes view is bookmarkable
via `?workspace=&left=&right=` URL state.

Failure or degraded path: overconstrained filter → empty-state page with
one-click filter removal, not a blank table; unknown route → recovery page, not a
stack trace; mutation attempted via GET → rejected (writes stay POST-backed).

Outputs: rendered HTML pages; a shareable URL.

Handoffs: bookmarked URL reopens identical state; JSON twins feed automation.

Validation evidence: VAL-003 (`VALIDATION.md`), including a first-impression /
screenshot check (CREST's 5-second test) for the cold new-user landing.

### CON-004: Cross-surface query parity

Trigger: operator (or test) checks that one question yields one answer
everywhere.

Inputs: one non-trivial filter run on `icelines query leaders --filter "…"`,
TUI `:stats`, `GET /leaders` + `/api/v1/leaders`, and `export md leaders`.

Normal path: all renderers lower the same `LeadersView` and produce identical
rows, identical applied filters/sort, and identical completeness/source warnings.

Failure or degraded path: a renderer that re-derives logic locally diverges →
caught by parity fences (e.g. `l2_query_goalies_cli_and_web_row_identity_match`);
the divergence is a defect, not a tolerated difference.

Outputs: matching rows across surfaces; a parity test result.

Handoffs: feeds `VERIFICATION.md` parity fences and the surface-parity matrix.

Validation evidence: VAL-004 (`VALIDATION.md`); `../specs/surface-parity.md`.

### CON-005: Offline cold launch

Trigger: a user on a machine with no network opens IceLines.

Inputs: `icelines` (workbench) and a historical-season query, no `fetch` run.

Normal path: the default workspace and a historical query over **bundled-backed
domains** (bios/stats, plus goalies/transactions/playoffs where an installed
bundle exists) succeed with **zero network calls**; freshness/provenance show
"bundled."

Failure or degraded path: persistence is **per-source asymmetric** — bios/stats
and installed goalies/transactions/playoffs serve offline, but **snapshot-only
domains (realtime, MoneyPuck, contracts) have no bundled fallback** and surface
an explicit `MissingSource`/unavailable state; tonight's live scores show
"offline / live unavailable," not a hang or a silent zero; `[live]
enabled=false` honored.

Outputs: rendered views sourced entirely from the bundle.

Handoffs: when network returns, `icelines fetch` (CON-008) refreshes opt-in.

Validation evidence: VAL-005 (`VALIDATION.md`).

### CON-006: Data-depth acquisition (boxscore / shift)

Trigger: a data tracker wants deeper detail than the bundle carries.

Inputs: `icelines data install <season>`; `icelines fetch boxscore
--for-favorites`; `icelines data status`; `snapshot verify`.

Normal path: a season installs; boxscore-level detail loads for favorites into
the manifest/`EventStream`; `data status` shows freshness/provenance per kind;
integrity is verified.

Failure or degraded path: **shift-level** tracking is requested → the locked
`shifts=off` capability returns the explicit BENCH-H3 refusal, not a silent
no-op; partial fetch saves with `partial:true` and is resumable.

Outputs: installed/cached artifacts; an honest capability/status report.

Handoffs: cached boxscores feed records/streaks (CON-002) and scoring reports.

Validation evidence: VAL-006 (`VALIDATION.md`).

### CON-007: Fantasy manager decision loop

Trigger: a manager evaluates roster moves before a deadline.

Inputs: `:poach rw cats=hits,blocks free top=12`; `fantasy gaps`;
`fantasy simulate add=… drop=…`; `fantasy import-yahoo --file …`.

Normal path: poach board, roster gaps, and add/drop simulation render shared
ViewModels (`PoachBoardView` / `FantasyRosterGapView` / `FantasySimulationView`)
on CLI/TUI/Web read surfaces; the manager reshapes to a fantasy layout and acts.

Failure or degraded path: league/roster mutations on the web dashboard are
**deferred** → routed to CLI/TUI with an explicit message, never a GET-mutation;
invalid drop → explicit scenario-resolution error.

Outputs: ranked candidates, gap board, projected scenario deltas.

Handoffs: CLI/TUI remain the canonical mutation surface; reports via CON-002.

Validation evidence: VAL-007 (`VALIDATION.md`); surface-parity fantasy rows.

### CON-008: Fresh-data refresh and upstream failure (WIRE)

Trigger: a user refreshes live/fresh data from upstream.

Inputs: `icelines fetch all` / `fetch sync`; live NHL API, optional MoneyPuck,
ESPN transactions.

Normal path: cache-first protocol — snapshot read (**integrity hash verified
before deserialization**; `--refresh` skips the cache and refetches) or fetch
with retry; schema validated (`deny_unknown_fields`); write to snapshot/manifest
with integrity hash; `LoadOutcome.missing` populated for absent silos.

Failure or degraded path: HTTP 429 → backoff honoring `Retry-After`; 503 → clear
"API unavailable, try again" message; **schema drift → loud deserialization
failure**, never a silent dropped field; a snapshot written by a **newer binary**
(`bundle_schema_version > MAX_KNOWN_BUNDLE_SCHEMA`) is **refused with a clear
upgrade message**, not silently corrupted; an **integrity-hash mismatch is a hard
error**, not a silent read; MoneyPuck/Realtime/Contracts absent → `MissingSource`
flag surfaced, not zeroed; ESPN abbrev drift (PHX→ARI→UTA) mapped season-aware,
unknown → `LEAGUE` synthetic + WARN.

Outputs: refreshed snapshots/manifests; explicit `source_state`/warnings.

Handoffs: refreshed data feeds every read scenario; staleness surfaces per
Contract 1.

Validation evidence: VAL-008 (`VALIDATION.md`); `INTERFACES.md` (external APIs).

### CON-009: Build, gate, and lean/standalone run (operator / maintainer)

Trigger: a maintainer builds the binary or ships a release.

Inputs: `cargo build/test`, `cargo clippy -- -D warnings`, `cargo fmt --check`;
`cargo build --no-default-features --features cli` (target).

Normal path: the workspace builds and all gates pass with **no cross-repo
dependencies** (post FLETCH/SLICE removal). A `--no-default-features --features
cli` build (target) yields a lean, offline CLI without web/TUI/network crates.

Failure or degraded path: **today** the lean/standalone build does **not** hold —
FLETCH/SLICE are still present and `icelines-cli` hard-depends on
`icelines-web`/`ratatui`/`axum`; tracked as `[target]` in `MISSION.md` and
`REQUIREMENTS.md` (REQ-DEP), implemented and verified before the criterion is
claimed.

Outputs: a release binary; green CI; (target) a lean offline binary.

Handoffs: build evidence feeds `VERIFICATION.md` and `REVIEW.md`.

Validation evidence: VAL-009 (`VALIDATION.md`).

### CON-010: Major analytics cache for hockey decision surfaces

Trigger: a coach, analyst, or future report/screen wants the same prepared
evidence for game-day, opponent scout, player-card, line-combination, goalie,
practice-focus, and postgame review workflows without recomputing or inventing
analytics in each front end.

Inputs: explicit cache build/read request with season/type, source window,
query/view family, team/player/game/line keys where applicable, source manifest
or snapshot generation, and requested consumer contract.

Normal path: a cache builder reads only validated local/bundled/snapshot source
state, computes canonical analytics records, attaches provenance, freshness,
source-window, quality/completeness, and invalidation metadata, and exposes a
read model for downstream screens/reports. Consumers can ask for prepared facts
and explanation fields, but may not silently recompute rankings, confidence, or
source-state meaning. Cache records are evidence for human judgment, not
autonomous coaching authority.

Failure or degraded path: missing source, stale generation, partial window,
invalid key shape, unsupported metric, or cache/schema mismatch returns typed
unavailable/stale/partial/refusal state. The cache does not call live APIs during
read paths, does not zero-fill missing hockey facts, and does not claim
prediction accuracy, injury certainty, betting insight, line chemistry causality,
or complete-world truth unless a later controlled requirement proves it.

Outputs: versioned analytics cache records and consumer-ready envelopes carrying
metric values, context, provenance, freshness/staleness, quality/completeness,
warnings, invalidation keys, and disclosure text.

Handoffs: future Coach Game-Day Dashboard, Opponent Scout Report, Player
Evidence Card, Line Combination Explorer, Goalie Readiness & Workload View,
Practice Focus Report, Postgame Review Report, and agent-facing summaries
consume the cache through `IF-CACHE-001`.

Validation evidence: VAL-011 (`VALIDATION.md`).

## Operational Assumptions

- Single operator on one machine; no auth, no multi-tenant; state under
  `~/.icelines/`.
- The bundle (38 seasons; ~5 modern fully detailed, 33 skeleton) is present in
  the binary; offline flows depend on it.
- Live/fresh data depends on the public NHL API (and optional MoneyPuck/ESPN),
  which have no SLA; degraded paths are the norm, not the exception.
- `shifts=off` is a locked capability today; shift-level tracking is a stated
  target with a known constraint.
- The lean/standalone build is a target; the default build includes all surfaces.

## Open Questions

- Named/saved workbench layouts (state-of-the-art persistence): where do they
  live — `~/.icelines/config.toml`, a dedicated layouts file, or the SQLite db —
  and how do TUI and Web round-trip them? (Tracked as a `[target]` in MISSION.)
- For perspective answers over skeleton seasons, what is the minimum completeness
  disclosure that is both honest and not noisy? (CON-002.)
- Lean-build feature matrix: exact `web`/`tui`/`net` boundaries and whether a
  `cli`-only build still wants `net` for opt-in fetch. (CON-009; `DESIGN.md`.)
- Does removing the FLETCH/SLICE surfaces (CON-006/CON-008 inputs) drop any
  command users currently rely on? (REQ-DEP; `DESIGN.md` rollback.)
