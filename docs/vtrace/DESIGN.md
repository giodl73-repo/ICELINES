# Detailed Design

## Scope

Repo or feature: `icelines` repo-baseline VTRACE adoption.

This file turns `ARCHITECTURE.md` into implementation-facing design decisions for
the current IceLines workspace. It allocates requirements to modules, ViewModel
builders, renderer adapters, data-source state, validation hooks, and target
dependency work. It does not claim that target-only work is implemented.

## Design Decision Summary

| ID | Decision | Requirement IDs | Rationale | Alternatives | Evidence |
|---|---|---|---|---|---|
| DES-001 | Treat ViewModels/envelopes as the only semantic rendering boundary for cross-surface features. | REQ-PARITY-001; REQ-DATA-001; REQ-REPORT-001 | Prevents renderer-local ranking, filtering, source-state, or report-fact drift. | Per-surface DTOs or string renderers. | `design/specs/viewmodels.md`; `design/specs/platform-contracts.md` |
| DES-002 | Build `ViewContext` from active `season`, `season_type`, query/window signature, `LoadOutcome`, and source-generation metadata before rendering. | REQ-WB-002; REQ-DATA-001; REQ-OFFLINE-001 | Every surface must know what data it is showing and how complete it is. | Renderer-specific context badges. | IF-DATA-001; IF-VIEW-001 |
| DES-003 | Route CLI args, TUI command bar phrases, web query params, and AI fallback output through `icelines-query` parser/planner or typed adapters before execution. | REQ-QUERY-001; REQ-WB-001; REQ-PARITY-001 | Preserves one query meaning across surfaces. | Duplicate CLI/TUI/web parsers. | `icelines-query/src/parser.rs`; `icelines-query/src/planner.rs` |
| DES-004 | Keep source fallback source-specific and project absence through `MissingSource`/source-state records. | REQ-DATA-001; REQ-OFFLINE-001; REQ-FRESH-001 | Avoids false zeroes and fake universal fallback chains. | Query-time live fallback or empty-success defaults. | `ARCHITECTURE.md` Source obligations |
| DES-005 | Treat `fetch` commands and cache/install flows as write paths into local state; queries read local/bundled state and do not call live APIs opportunistically. | REQ-OFFLINE-001; REQ-FRESH-001; REQ-DATA-DEPTH-001 | Offline behavior stays deterministic and upstream failures are explicit. | Live fetch on query miss. | IF-FETCH-001 |
| DES-006 | Split long-lived TUI state from one-shot CLI/web/export reads in design and tests. | REQ-WB-001; REQ-WB-002; REQ-PARITY-001 | Only TUI owns persistent pane, focus, and season-switch invalidation state. | One generic renderer state object. | `design/ARCHITECTURE.md` Surfaces |
| DES-007 | Web read state is bookmarkable and allowlisted in GET params; mutations are POST-backed or explicitly deferred to CLI/TUI. | REQ-WEB-001; REQ-WEB-002; REQ-FANTASY-001 | Keeps browser behavior safe, shareable, and recoverable. | Hidden localStorage truth or GET mutations. | IF-WEB-001; surface parity matrix |
| DES-008 | Reports and exports render from ViewModels or report-specific ViewModel projections and disclose scope/source state near the top. | REQ-STAT-001; REQ-REPORT-001; REQ-PARITY-001 | Public artifacts need reproducible facts and limitation language. | Formatter-only report code. | IF-REPORT-001 |
| DES-009 | Parity validation compares canonical row identity, filters, sort, warnings, source state, and empty-state semantics; layout is excluded. | REQ-PARITY-001; REQ-CODE-001 | Tests should fail on semantic drift, not harmless presentation differences. | Screenshot or text-only comparison. | VAL-004; `design/specs/surface-parity.md` |
| DES-010 | Target Cargo features are `cli`, `tui`, `web`, `net`, and `reports`; current `Cargo.toml` is all-surface and not yet compliant. | REQ-LEAN-001; REQ-DEP-001 | Gives implementation a concrete feature map while preserving current-state honesty. | Claim lean support before feature surgery. | `Cargo.toml`; IF-BUILD-001 |
| DES-011 | FLETCH and SLICE removal requires replacement, refusal, or rollback notes for affected command surfaces before standalone is claimed. | REQ-DEP-001; REQ-CODE-001 | Avoids silent feature loss during dependency independence work. | Delete deps and discover missing commands later. | `Cargo.toml`; VERIFICATION.md |
| DES-012 | Integrity/schema/freshness checks run before deserialization-dependent use, and failures become hard errors or typed unavailable state. | REQ-FRESH-001; REQ-DATA-DEPTH-001 | Upstream and cache corruption must not become plausible hockey output. | Best-effort parse with warnings after the fact. | IF-FETCH-001 |
| DES-013 | Historical perspective fixtures are split into lockout, rollover, ambiguous-name, trade-continuity, active-streak, and skeleton-completeness observations. | REQ-STAT-001; REQ-STAT-002; REQ-REPORT-001 | One broad public-post scenario is too coarse to verify. | One golden social-post snapshot. | VAL-002 |
| DES-014 | Visual/source/status tokens are shared semantics; CLI/TUI/web/report adapters choose representation and must provide non-color carriers. | REQ-WEB-002; REQ-WB-002; REQ-DATA-001 | GLASS/CREST concerns become implementable without pushing layout into core. | Renderer-only CSS/terminal color conventions. | Contract 6; IF-VIEW-001 |
| DES-015 | Named layouts use a shared versioned layout model for workspace/pane semantics; TUI/Web may add surface-specific display hints. | REQ-WB-003 | Carries the mission personalization target into design without making renderer layout a hockey semantic cache. | Save only terminal/browser-local state and call it portable. | IF-LAYOUT-001; VAL-010 |

## Module Allocation

| Area | Primary Modules / Files | Design Responsibility | Constraints |
|---|---|---|---|
| Domain repository | `icelines-core/src/stats_repository.rs`; `season_stats.rs`; `identity.rs`; `model.rs` | Store and expose `(player_id, season, season_type)` domain reads; enforce accepted row invariants. | No network, filesystem, renderer layout, or query-time live fetch. |
| ViewModels | `icelines-core/src/view_model/*`; `workbench.rs`; `favorites.rs`; scoring/report projections | Build typed `context`, rows, warnings, semantic tokens, empty states, source state, and stable row IDs. | Renderer adapters may not recompute hockey semantics. |
| Query intent | `icelines-query/src/parser.rs`; `planner.rs`; `executor.rs`; `url.rs`; `data_provider.rs` | Normalize filters, sorts, route params, command-bar phrases, and AI fallback output into one typed intent. | Bad input returns typed errors with recovery hints. |
| Fetch/data writes | `icelines-fetch/src/stats_loader.rs`; `snapshot.rs`; `manifest.rs`; `sync_engine.rs`; `nhl_api.rs`; `moneypuck.rs`; `transactions.rs`; `fletch.rs` | Load local/bundled state, write fetched snapshots, verify integrity/schema, and emit missing-source records. | Live APIs are write paths only; failures are loud or typed. |
| CLI adapters | `icelines-cli/src/commands/*`; `render/*`; `cli.rs` | Adapt CLI args to query/fetch/ViewModel builders and render text/JSON/CSV/report output. | Preserve context/warnings in machine and human output. |
| TUI workbench | `icelines-cli/src/tui/*`; `icelines-core/src/workbench.rs` | Manage long-lived focus, pane, command, active season/type, and cache invalidation state. | State switches clear derived caches tied to `(season, season_type)`. |
| Web adapters | `icelines-web/src/handlers/*`; `api.rs`; `dashboard_command.rs`; `templates.rs`; `workbench.rs`; `state.rs` | Render no-JS HTML, JSON twins, bookmarkable read state, empty/recovery states, and safe mutations. | GET is read-only; user-controlled read state is URL-visible and allowlisted. |
| Layout state target | `icelines-core/src/workbench_layout.rs`; `icelines-cli/src/commands/layout.rs`; `icelines-cli/src/tui/mdi.rs`; `icelines-web/src/handlers/dashboard.rs` | Persist and restore named TUI/Web workbench layouts through a shared versioned record and CLI-owned store path. | Semantic layout choices are versioned and portable; display-only hints stay surface-specific. |
| Reports/exports | `icelines-cli/src/commands/export*`; report ViewModel builders; `icelines-site` where still exercised | Produce Markdown/JSON/CSV artifacts from ViewModels or report projections. | Disclosure appears near top; no predictive or era-adjusted overclaiming. |
| Build boundary | workspace `Cargo.toml`; member `Cargo.toml` files | Current all-surface build plus target feature map for lean CLI and standalone dependency work. | No compliance claim until dependency inspection and lean build evidence pass. |

## Key-Shape and Surface Lifetime Design

### Domain key-shape design

| State / Record | Key Shape | Stable Across Season Switch? | Design Rule |
|---|---|---|---|
| Player identity, preferred display name, handedness, birth data | `player_id` plus source generation where source-sensitive | Usually yes | Access through identity/ViewModel accessors; do not duplicate display-name matching in renderers. |
| Season skater/goalie stats | `(player_id, season, season_type)` | No | Reads must keep the active window explicit; playoff and regular rows are not interchangeable. |
| Team stints and transaction-derived team display | `(player_id, season, season_type, stint date/order)` | No | Preserve multi-stint history; do not collapse traded players into one team except through an explicitly named last-stint view. |
| Team rosters | `(team, season, season_type, roster mode)` | No | Distinguish last-stint roster from all-stints roster; cache keys must include the window and roster mode. |
| Schedule, scores, playoffs, and game detail | `(season, season_type, date/team/game_id as applicable)` | No | A game cache keyed only by team or date is incomplete if the active window can change. |
| Query results and leaderboard rows | `(view family, season, season_type, query/filter/sort/window signature, source generation)` | No | Rebuild when active window, typed query, or source generation changes. |
| Source/completeness notices | `(source domain, season/window when applicable, snapshot/cache generation)` | Sometimes | Notices follow the data they describe; renderer-local notices cannot outlive the ViewModel context. |
| User preferences, saved groups, favorites, watch rules | local user ID/config key plus explicit referenced domain IDs | Yes, unless a preference encodes a window | Browser GET reads may display them; mutations require POST-backed paths or CLI/TUI action. |

Rules:

- `PlayerView` or ViewModel accessors are the preferred read boundary for
  renderer-facing facts. Direct reads from raw totals should be limited to core
  builders and tests that are asserting the model itself.
- If state changes meaning when `(season, season_type)` changes, that axis is in
  the key, cache invalidation trigger, or typed construction argument.
- If a cache key intentionally omits an axis, the owning code must state why the
  data is identity-stable or source-stable.

### Surface lifetime and cache placement

| Surface / Adapter | Lifetime | Allowed Long-Lived Semantic State | Disallowed State |
|---|---|---|---|
| TUI workbench | Long-lived event loop | `App`/workbench state slices, active window, focus, selected row, command state, source notices, and ViewModel caches keyed by active window/source generation. | Renderer-only caches that survive `repo_swap` without the right key; hidden season/type globals. |
| CLI command | One invocation | Parsed args, one loaded repository/outcome, one ViewModel/report projection, exit status. | Cross-command semantic cache, live query fallback, renderer-local rankings. |
| Web HTML handler | One request for semantic data | Request context, allowlisted query params, loaded repository/outcome, ViewModel, response-local notices. | Hidden server-side semantic cache that changes results without URL/source context; GET mutation. |
| Web JSON twin | One request for semantic data | Same typed intent and ViewModel context as HTML where applicable. | JSON-only planning, omitted warnings/source state, display-name row identity. |
| Reports/exports | One fixed generation | Fixed intent, fixed fixture/clock when evidence requires it, disclosure block, stable sections. | Formatter-only facts, omitted source/scope, predictive/era-adjusted overclaiming. |
| Static site | Deferred for this baseline | None claimed as an active surface. | Using `icelines-site` presence as evidence of active static-site compliance. |

The web server may keep immutable assets, configuration, or non-semantic
framework state. It must not keep a hidden semantic hockey cache unless a later
design updates the cache key, invalidation, and source-state contract.

## Algorithms / Logic

### Query-to-render flow

```text
surface input
  -> typed adapter
  -> icelines-query parser/planner
  -> data provider/repository read
  -> ViewModel builder with ViewContext
  -> renderer adapter
  -> text / TUI / HTML / JSON / CSV / Markdown
```

Rules:

- Adapters may translate syntax into typed intent; they may not implement a
  second hockey query planner.
- Query errors preserve spans or field names so each surface can show recovery
  without changing semantics.
- Row identity is a domain ID, never a display name.

### Source-state projection

```text
LoadOutcome + source metadata + active window
  -> SourceState entries
  -> ViewContext.completeness
  -> ViewModel warnings / empty_state
  -> renderer badges, labels, aria text, report disclosure, JSON fields
```

Rules:

- `MissingSource` is a typed unavailable/partial state, not `0`, an empty vector,
  or omitted JSON.
- Source state names the affected domain when known: roster, schedule, game logs,
  shifts, injuries, fantasy import, realtime, MoneyPuck, contracts, or upstream
  API.
- Renderers may change wording, density, glyph, or badge style, but not the
  state meaning.

### Source fallback design

| Source / Domain | Design Path | Failure / Absence Handling |
|---|---|---|
| Bios and skater stats | Prefer valid snapshot tiers where present, then embedded bundled data according to loader rules. | Missing/stale data creates source-state disclosure. |
| Goalies | Use legacy/embedded/installed support where implemented. | Missing goalie domain becomes unavailable/partial state, not a skater zero. |
| Transactions | Use legacy/embedded/installed or fetched local state; preserve season-aware team mapping. | Unknown team maps to `LEAGUE` with warning. |
| Playoffs | Use installed/bundled bracket/game-log state where available. | Missing bracket/game detail yields explicit empty or unavailable state. |
| Realtime, MoneyPuck, contracts | Read local snapshot/cache produced by opt-in fetch. | Absence becomes `MissingSource`; query does not call live APIs. |
| NHL API / ESPN / MoneyPuck upstream | `fetch` command write path with retry/backoff/integrity/schema behavior. | 429/503/schema drift/integrity mismatch fail loud or degrade explicitly. |
| Local SQLite | Groups, fantasy, watch, roster snapshots, and local user state. | Missing setup yields actionable empty state; browser GET does not mutate it. |

### Fetch boundary state machine

| Stage | Success Output | Failure Output | Design Constraint |
|---|---|---|---|
| Locate local snapshot/cache/bundle | Candidate source plus metadata | `MissingSource` or actionable setup message | Absence is state, not a zero-value data set. |
| Verify integrity/version before use | Accepted bytes and source generation | Hard refusal for hash mismatch or newer unsupported schema | Do not deserialize or partially use unverified bytes. |
| Fetch upstream on explicit command | Validated upstream payload | Typed HTTP/schema/rate-limit/unavailable error | Query-time paths do not perform this stage. |
| Retry/backoff | Completed request or exhausted retry result | Error carrying retry/unavailable context | Honor `Retry-After` where present; distinguish 429 from 503 and schema drift. |
| Validate schema/content | Typed records and source metadata | Loud schema/content error or explicit skipped-source result | Unknown or incompatible shape cannot become plausible output. |
| Write snapshot/cache | Durable local state and metadata | Write failure with no success-shaped query result | Partial writes are rejected or marked resumable/partial before exposure. |
| Build `LoadOutcome` | Repository-ready data plus `missing`/warnings | Unavailable result with recovery guidance | Every caller receives source state to project through ViewModels. |

### TUI state split

| TUI State Slice | Owns | Reset / Invalidation Trigger | Evidence Hook |
|---|---|---|---|
| `ActiveWindowState` | active `season`, `season_type`, selected source window | Season/type switch | VAL-001 active-context demo |
| `PaneBindingState` | left/right pane IDs, focused pane, bound experience | URL/workbench binding change or explicit user action | VAL-001; VAL-004 |
| `CommandState` | command bar text, parse result, recovery hint | Submitted command, escape/cancel, screen switch | REQ-QUERY-001 parser tests |
| `ViewCacheState` | derived ViewModel cache keyed by window/query/source generation | Any active window, filter/sort, or source generation change | REQ-PARITY-001 fixtures |
| `SelectionState` | selected row/player/team/game, drilldown target | Dataset identity changes or selected item disappears | VAL-001 edge observation |
| `SourceNoticeState` | current warnings, missing-source notices, stale indicators | New `LoadOutcome` or ViewModel context | VAL-005; VAL-008 |

This split is a design contract, not necessarily a current file split. If the
implementation keeps a larger `App` struct temporarily, `CODE_RIGOR.md` must
forbid new semantic logic from being added directly to renderer code.

### Web state and route design

| Route Family | Read State | Mutation Boundary | Required States |
|---|---|---|---|
| `/dashboard` | `workspace`, `left`, `right`, `experience`, active season/type where supported | No GET mutation; command parser rejects deferred writes or routes to POST paths. | no-JS shell, active context, pane recovery |
| `/leaders`, `/goalies`, `/career`, `/compare` | filter/sort/window params allowlisted through typed adapters | n/a | overconstrained filter empty state, source warnings |
| `/scores`, `/schedule`, `/playoffs`, `/game/:id` | date/team/game/window params where applicable | POST-backed cache loading for missing game/scoring details where implemented. | missing-cache recovery, loaded-zero distinction |
| `/favorites`, `/fantasy`, `/watchlist` | selected group/team/player/read filters | POST-backed add/remove/create/toggle paths or CLI/TUI deferral. | local-state missing/setup empty state |
| `/api/v1/*` JSON twins | Same typed intent as HTML twin where applicable | Mutation twins return typed mutation result. | context, warnings, empty_state, source_state |

### Surface context and visual hierarchy

| Surface | Context Placement | Primary Decision Path | Source/Status Carrier | Next Action / Recovery |
|---|---|---|---|---|
| CLI text/table | Header or first lines before rows. | Stable row order, compact columns, explicit filter/sort labels. | Text labels plus optional color/glyphs. | Exit text or footer hint. |
| CLI JSON/CSV | Top-level context fields or companion metadata fields. | Stable row IDs and typed metric fields. | Machine-readable `source_state`, warnings, and omissions. | Error object or empty-state record. |
| TUI | Header/status band visible on every screen. | Focused pane, selected row, and bounded table/card density. | Text/glyph/color with non-color carrier. | Command hint, recovery row, or status notice. |
| Web HTML | Above-the-fold context near page title/dashboard chrome. | Clear visual hierarchy: context, primary result, supporting evidence, action. | Text/badge/aria labels; color is secondary. | Empty, stale, partial, loading, and 404 states are designed surfaces. |
| Web JSON | Same semantic context as HTML twin. | Stable IDs, typed metrics, typed empty state. | Structured warnings/source state. | Structured recovery/error fields. |
| Markdown/report | Disclosure block near top, before claims. | Stable sections and headings. | Plain-text source/scope disclosure. | Omission/recovery notes where useful. |

### Report/export design

Reports follow this sequence:

```text
fixed intent + fixed fixture/clock
  -> ViewModel or ReportView projection
  -> disclosure block
  -> stable sections with IDs
  -> human artifact
  -> optional machine-readable equivalent
```

The disclosure block includes report kind, generated time, season/type, data
source/completeness state, filters/sort/scoring scheme, warnings, and omissions.
Historical perspective copy must remain descriptive and must not imply
era-adjusted, predictive, betting, deployment-adjusted, or linemate-adjusted
meaning.

### Parity comparison design

VAL-004 parity evidence compares:

- ViewModel family and version.
- Active `season` and `season_type`.
- Applied filters and sort.
- Stable row IDs and row order where order is semantic.
- Primary metric and typed numeric display policy.
- Warnings, omissions, empty state, and source/completeness state.
- JSON and human adapter preservation of the same semantic tokens.

Parity evidence does not require identical text wrapping, color, pagination,
terminal width, CSS, table borders, or card layout.

### Major analytics cache target design

The major analytics cache is a target design baseline for future hockey decision
surfaces. The first implementation slices add the versioned
`icelines-core::analytics_cache` record/consumer-envelope types and strict
`icelines-fetch::analytics_cache_store` JSON store/read path plus the internal
`icelines-core::view_model::analytics_cache_consumer` fixture and first
named-cache Web report/JSON twin. Broader shipped product surfaces remain
pending.

Required record envelope:

```text
AnalyticsCacheRecord
  cache_schema_version
  producer { crate/version/git/source_manifest_id }
  scope { season, season_type, source_window, entity keys }
  metric_family + metric_id + value + unit + methodology_version
  provenance { sources, snapshot/install/cache generation, fixture/source IDs }
  freshness { generated_at, source_as_of, stale_after, status }
  quality { completeness, confidence_label, warnings, omissions }
  invalidation { source_generation, dependency_keys, rebuild_reason }
  consumer_contract_version
  disclosures
```

Build path: cache builders read only explicit local/bundled/snapshot source state
and fail with typed unavailable/stale/partial/schema/unsupported states when
inputs are not sufficient. Builders do not call live APIs except through an
explicit fetch/write command that produces validated source state before a cache
build.

Read path: consumers request a cache envelope by consumer contract and typed
keys. The envelope carries prepared analytics and disclosure fields; downstream
screens and reports must not recompute rankings, confidence, freshness,
quality/completeness, or source-state meaning locally. Renderer-local sorting is
allowed only for visual presentation when the canonical rank/order remains
available.

Consumer families:

- Coach Game-Day Dashboard.
- Opponent Scout Report.
- Player Evidence Card.
- Line Combination Explorer.
- Goalie Readiness & Workload View.
- Practice Focus Report.
- Postgame Review Report.
- Agent-facing summary prompts.

Non-claims: the cache is decision support for humans. It does not provide
autonomous coaching authority, validated prediction accuracy, betting value,
injury certainty, line-chemistry causality, or complete-world truth unless a
later controlled requirement and validation row prove that narrower claim.

### Evidence tier allocation

| Design Area | Evidence Tier | Evidence Shape |
|---|---|---|
| Key-shape and model invariants | L0 unit/property/compile-fail where applicable | Pure core tests for `(player_id, season, season_type)`, team stints, roster modes, and borrow/send constraints. |
| Query parser/planner | L0 unit and fixture tests | Aliases, duplicate filters, invalid filters, span/field recovery, route param normalization, and command-bar handoff. |
| Fetch boundary and upstream failures | L1 tempdir/httpmock/integrity fixtures | 429, 503, schema drift, unknown fields, partial results, integrity mismatch, newer snapshot schema, ESPN abbrev drift. |
| CLI parity and reports | L2 subprocess plus snapshots | Stable command output, exit status, JSON/CSV/Markdown disclosure, anti-overclaim copy. |
| TUI state/cache/visual context | Snapshot/demo evidence plus targeted cache tests | Active context, season switch invalidation, source notice survival, selected-row recovery, non-color state carrier. |
| Web routes and JSON twins | Route tests plus browser/no-JS inspection | GET read-only, allowlisted params, no-JS shell, active context, 404/recovery, narrow viewport, JSON context/source state. |
| Major analytics cache | L0 schema/contract/unit fixtures plus L1 tempdir source-state fixtures and L2 consumer demos | Cache record compatibility, provenance/freshness/invalidation, stale/partial/refusal states, no query-time live fetch, and consumer envelope preservation. |
| Lean/standalone targets | Build/dependency inspection evidence | `cargo build --no-default-features --features cli` and dependency graph checks only after feature work exists. |
| Cross-surface parity | Comparative fixture evidence | ViewModel version/context, row IDs/order, filters/sort, metrics, warnings, omissions, source state, empty state. |

### Target Cargo feature design

Current state: the workspace has no implemented lean feature split. The target
feature map is:

| Feature | Includes | Must Exclude | Notes |
|---|---|---|---|
| `cli` | `icelines-core`, `icelines-query`, offline CLI renderers, bundled/local reads, report text/JSON/CSV where no network/web/TUI dependency is needed. | `axum`, `tower`, `tower-http`, `askama`, `ratatui`, `crossterm`, `reqwest`, live `tokio` network paths unless separately selected. | Target for `cargo build --no-default-features --features cli`. |
| `tui` | TUI workbench, ratatui/crossterm adapters, keyboard/focus state. | Web server/router/template crates. | Depends on `cli` or shared core/query. |
| `web` | Axum handlers, templates, JSON twins, static assets, browser command parser. | TUI crates. | May require a web-safe core feature such as current `send-sync`. |
| `net` | `reqwest`, API clients, fetch/write commands, retry/backoff support. | No requirement to include TUI or web. | Opt-in for fresh-data writes. |
| `reports` | Markdown/JSON/CSV report/export projections. | Web/TUI/network unless a report explicitly opts into fetch. | Can be included by default CLI if dependency-light. |

Standalone target work removes or replaces:

- `fletch-core` path dependency in `icelines-fetch`.
- `slice-core` git dependency in `icelines-query`.

Every affected command or selector must choose one path: native replacement,
explicit refusal message, compatibility shim with no cross-repo dependency, or
rollback note. The dependency removal is not complete until dependency
inspection and command-surface verification pass.

## Invariants

- `icelines-core` remains free of network I/O, filesystem I/O, terminal layout,
  HTML/CSS, and web route concerns.
- `StatsRepository` reads are keyed by `(player_id, season, season_type)` when
  season/type affects the data.
- Long-lived semantic caches belong only to TUI state unless a later design
  explicitly defines another cache key/invalidation contract.
- Analytical cache keys include active window, typed query/filter/sort/window
  signature, and source/data generation where known.
- `PlayerView`/ViewModel accessors are the renderer-facing read boundary for
  hockey facts.
- `LoadOutcome.missing` and source/completeness state survive into ViewModels and
  applicable renderers.
- Query-time reads do not call live APIs to hide missing local data.
- GET routes are read-only; browser mutation is POST-backed or deferred.
- Reports disclose source/completeness state near the top.
- Color is never the only carrier of state.
- Static site generation is deferred for this VTRACE baseline unless later
  artifacts explicitly reactivate it.
- Standalone and lean CLI are target states until `Cargo.toml` and build evidence
  prove them.
- Major analytics cache has partial implementation evidence through initial core
  schema/consumer fixtures, strict fetch-layer store/read fixtures, and an
  internal dashboard-style ViewModel fixture plus the first named-cache Web
  report. Future screens still cannot claim broad cache-backed analytics before
  shipped-surface evidence and copy review exist.
- Future cache-backed consumers must carry provenance, freshness/staleness,
  quality/completeness, warnings, and disclosures through to the user-visible or
  machine-readable output.

## Edge Cases

| Edge Case | Expected Behavior | Verification |
|---|---|---|
| 2004-05 lockout season in historical query | Omit or label according to available data without pretending games existed. | VAL-002 fixture observation |
| October `CURRENT_SEASON` rollover | Active season/type remains visible and source state discloses stale or unavailable current data. | VAL-002; VAL-003 |
| Ambiguous player names | Parser/planner or ViewModel gives disambiguation by stable ID/context, not first display-name match. | VAL-002; REQ-QUERY-001 tests |
| Player traded mid-season | Season totals and team stints preserve continuity; report copy avoids single-team overclaim. | VAL-002 fixture observation |
| Active streak still ongoing | Label ongoing state explicitly; do not render as a completed streak. | VAL-002 report inspection |
| Skeleton/bundled-only season | Disclose partial/skeleton completeness near the result and report top. | VAL-002; VAL-005 |
| Realtime/MoneyPuck/contracts absent offline | Render `MissingSource`/unavailable state; do not show zero-shaped success. | VAL-005; VAL-008 |
| Upstream 429 or 503 | Honor retry/backoff when available and surface unavailable/retry state. | VAL-008 |
| Snapshot integrity mismatch | Refuse the snapshot before use; no partial parse. | VAL-006; VAL-008 |
| Newer snapshot schema | Refuse with upgrade-oriented message. | VAL-008 |
| Unknown ESPN team abbreviation | Map to season-aware team when possible or `LEAGUE` with warning. | VAL-008 |
| Overconstrained web filter | Show empty-state recovery preserving active context and source state. | VAL-003 |
| Narrow viewport / color-only risk | State remains visible through text/glyph/aria/report labels, not only color. | VAL-003 |
| Web GET mutation phrase | Reject, route to POST-backed handler, or defer to CLI/TUI with explicit guidance. | VAL-003; VAL-007 |
| Cache requested with stale, partial, incompatible, unsupported, or missing source state | Return a typed cache unavailable/stale/partial/refusal state with source-window disclosure; do not emit zero-shaped success. | VAL-011 |
| Cache consumer lacks a supported contract version | Refuse with an upgrade/compatibility message rather than silently dropping fields. | VAL-011 |
| TUI season switch with cached panes | Derived caches tied to old window are cleared or rebuilt before render. | VAL-001 |
| Lean build attempted today | Treat failure as expected target-not-met evidence, not regression against current baseline. | EVID-LEAN-001 |

## Migration / Rollout

This design is a traceability artifact for a docs-first VTRACE pass. Rollout
should proceed in narrow implementation waves:

1. Lock `CODE_RIGOR.md` constraints for ViewModel purity, source-state
   preservation, query intent, web mutation safety, report disclosure, and
   feature/dependency claims.
2. Convert the design tables above into implementation work only when a wave is
   selected; do not refactor all surfaces at once.
3. Add or update evidence rows in `VERIFICATION.md` as commands, fixtures,
   route tests, parity checks, and demos become available.
4. Treat FLETCH/SLICE removal and lean feature gating as target implementation
   waves with rollback/refusal notes before any public compliance claim.
5. Treat the major analytics cache as an incremental implementation wave: the
   initial `icelines-core::analytics_cache` schema/consumer contract,
   `icelines-fetch::analytics_cache_store` store/read path, and internal
   `icelines-core::view_model::analytics_cache_consumer` fixture now feed the
   first named-cache Web report/JSON twin, but broader shipped consumer claims
   remain pending until their evidence passes. Do not let a dashboard/report
   claim cache-backed analytics before the matching evidence exists.
6. Revisit `TRACE.md` after design review closure so design elements point to
   `DESIGN.md` rows rather than only architecture-level contracts.

No user data migration is introduced by this file. Future local-state changes
must preserve existing `~/.icelines/` config, snapshots, FantasyDb, favorites,
watch rules, reports, and cache directories unless a migration plan is recorded.

## Code Rigor Hooks

These are the constraints that the next VTRACE file, `CODE_RIGOR.md`, should
formalize before production changes are made.

| Area | Risk | Required Code Rigor Constraint |
|---|---|---|
| ViewModel builders | Renderer-local semantic recomputation creeps back in. | Builders own ranking/filter/source-state semantics; adapters only render. |
| Source state | Missing source becomes zero, empty success, or omitted JSON. | Tests/assertions preserve `LoadOutcome.missing` into `ViewContext`, warnings, JSON, reports, and browser states. |
| Query intent | CLI/TUI/web parsing diverges. | Shared parser/planner fixtures cover aliases, duplicate filters, bad filters, route params, and command-bar handoffs. |
| TUI state | Long-lived `App` mixes cache, focus, query, source notices, and render code. | New semantic state lands in named state slices or helper modules, with cache-key tests for active window changes. |
| Web routes | GET paths mutate local SQLite state or hide state in localStorage. | Route tests prove GET read-only behavior and allowlisted bookmark state. |
| Reports | Public artifacts omit source/scope or overclaim perspective. | Snapshot tests include disclosure block and anti-overclaim copy checks. |
| Fetch/integrity | Corrupt or unsupported snapshots are parsed before validation. | Integrity/schema/version checks happen before use and have explicit failure tests. |
| Upstream failures | 429/503/schema drift degrade as plausible output. | Integration tests cover retry/backoff, unavailable state, and loud failures. |
| Feature gating | Lean/standalone claim lands before Cargo proof. | Dependency inspection and `cargo build --no-default-features --features cli` evidence are required before status changes. |
| FLETCH/SLICE removal | Command behavior disappears silently. | Every removed dependency surface records replacement, refusal, compatibility shim, or rollback note. |
| Analytics cache | Future screens compute or reinterpret metrics independently. | A cache contract test proves consumers preserve cache envelope semantics and disclosure without local source-state/confidence recomputation. |
| Visual tokens | Color is the only state carrier. | Token adapters include text/glyph/aria/report labels and ASCII fallback where applicable. |
| Evidence ledger | Reviews stay prose-only. | Verification rows link command output, fixture names, route tests, demos, or explicit target-not-met evidence. |

## Open Design Risks

| Risk | Disposition |
|---|---|
| Exact current file split for TUI state may lag behind the logical state slices. | Carry to CODE_RIGOR and implementation waves; forbid adding new semantic logic to renderer-only paths. |
| Cargo feature names are target names, not implemented workspace features. | Keep REQ-LEAN-001 target until build evidence exists. |
| FLETCH/SLICE replacement scope may affect user-visible commands. | Require replacement/refusal/rollback notes before dependency removal is called complete. |
| Validation evidence remains mostly pending. | Move to evidence rows during Gate 3; this file only defines evidence hooks. |
| Static site status is deferred while `icelines-site` remains in the workspace. | Do not advertise static site as active user surface without a later design update. |
| Major analytics cache implementation is partial with only the first named-cache product report surface. | Treat the in-core schema/consumer contract, strict store/read path, internal consumer ViewModel, and named-cache Web report as foundation evidence only; keep broader shipped consumer claims pending until their WP-009 evidence passes. |
