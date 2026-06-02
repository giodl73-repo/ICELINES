# Architecture

## Scope

Repo or feature: `icelines` repo-baseline VTRACE adoption.

This file is the VTRACE architecture bridge. It summarizes the load-bearing
system shape needed to satisfy `MISSION.md`, `CONOPS.md`, `REQUIREMENTS.md`, and
`INTERFACES.md`; it does not replace the deeper historical architecture record in
`design/ARCHITECTURE.md`.

## Architecture Summary

IceLines is a local, single-operator NHL analytics platform delivered as one Rust
workspace and one primary binary. The active user surfaces are CLI, TUI workbench,
and Web dashboard, with report/export artifacts as durable output renderers. The
mkdocs static site remains a deferred workspace member, not an active user
surface for this VTRACE baseline.

The architecture satisfies the mission through seven invariants:

1. One domain spine: `DataStore`/loader output populates `StatsRepository`, which
   exposes `(player_id, season, season_type)` reads through `PlayerView` and typed
   ViewModels.
2. One query intent: CLI args, TUI command bar, web params, and future AI fallback
   lower through `icelines-query` before execution.
3. One ViewModel boundary: renderers choose layout, not hockey meaning; row
   identity, context, warnings, source state, and empty state originate upstream.
4. Per-source data honesty: bundled, installed, snapshot, cache, and live fetch
   write paths carry typed completeness/provenance instead of silent zeroes.
5. Surface parity by artifact: any capability on more than one surface compares a
   canonical ViewModel/envelope across CLI, TUI, Web HTML, Web JSON, and exports
   where applicable.
6. Cache as evidence layer: future coach/scout/report/card/line/goalie/practice
   surfaces consume a versioned analytics cache contract rather than inventing
   their own hockey semantics.
7. Explicit targets: standalone/no-FLETCH-SLICE and lean CLI-only builds are
   target architecture, not current-state claims.

## Current vs Target Posture

| Claim Area | Current Baseline | Target / Deferred State | Requirement IDs |
|---|---|---|---|
| Active surfaces | CLI, TUI workbench, Web dashboard, Web JSON twins, and report/export artifacts are active review surfaces. | mkdocs/static-site generation remains deferred for this VTRACE baseline unless a later artifact reactivates it. | REQ-WB-001; REQ-WEB-001; REQ-PARITY-001; REQ-REPORT-001 |
| Data source truth | Bundled, snapshot, installed, cache, and live-fetch write paths are source-specific; no universal query-time fallback chain exists. | Design must preserve a per-source fallback table and prevent renderer-local source-state inference. | REQ-DATA-001; REQ-OFFLINE-001; REQ-FRESH-001 |
| Surface parity | Shared ViewModels/envelopes are the comparison artifact; layout differences are allowed. | Gate 3 requires evidence that canonical rows, warnings, filters, sort, and source state match for chosen parity fixtures. | REQ-PARITY-001 |
| Named layout persistence | TUI bindings and Web URL state exist, but durable named layouts are not yet proven. | Persisted layouts require a versioned shared layout model, migration/refusal behavior, and TUI/Web restore evidence before personalization is claimed complete. | REQ-WB-003 |
| Standalone dependency posture | FLETCH/SLICE dependency seams still exist and block standalone compliance claims. | Remove or replace FLETCH/SLICE seams before any standalone claim. | REQ-DEP-001 |
| Lean build posture | Default build remains all-surface; web/TUI/network crates are not yet gated out of a lean CLI path. | `cargo build --no-default-features --features cli` must compile and run offline before lean support is claimed. | REQ-LEAN-001 |
| Major analytics cache posture | The initial core schema, strict store/read path, internal downstream consumer ViewModel fixture, first named-cache Web report, first coach dashboard route, first opponent scout route, first player evidence-card route, first line-combination explorer route, and first goalie readiness route exist, but broader downstream surfaces are not claimed. | A versioned cache builder/read model supplies canonical analytics, provenance, freshness/staleness, quality/completeness, invalidation keys, and consumer contracts for future hockey decision surfaces. | REQ-CACHE-001..004 |
| Evidence maturity | Architecture is traceable review evidence; most validation and verification rows remain pending. | Future gates must attach command, fixture, snapshot, route, or demo evidence to the ledger. | REQ-CODE-001; VAL-001..VAL-010 |

## Components

| Component | Responsibility | Requirement IDs | Interfaces | Evidence |
|---|---|---|---|---|
| `icelines-core` | Pure domain model, `StatsRepository`, `PlayerView`, workbench catalog, ViewModels, semantic tokens, scoring/report projections. No network or file I/O should live here. | REQ-WB-002; REQ-PARITY-001; REQ-DATA-001; REQ-FANTASY-001 | IF-DATA-001; IF-VIEW-001 | `design/specs/viewmodels.md`; `design/specs/platform-contracts.md` |
| `icelines-query` | Deterministic Art Ross grammar, parser, planner, filters, sorts, and typed query errors. | REQ-QUERY-001; REQ-WB-001; REQ-STAT-001 | IF-QUERY-001 | `COMMANDS.md`; CON-001; CON-004 |
| `icelines-fetch` | External API clients, snapshot/cache readers and writers, manifest/install flows, integrity/schema checks, missing-source population. | REQ-DATA-001; REQ-OFFLINE-001; REQ-DATA-DEPTH-001; REQ-FRESH-001 | IF-FETCH-001; IF-DATA-001 | CON-005; CON-006; CON-008 |
| Major analytics cache (target) | Versioned producer/read model for prepared hockey analytics with source window, provenance, freshness/staleness, quality/completeness, invalidation, warnings, and consumer envelopes. | REQ-CACHE-001; REQ-CACHE-002; REQ-CACHE-003; REQ-CACHE-004 | IF-CACHE-001; IF-DATA-001; IF-VIEW-001; IF-REPORT-001 | CON-010; CHG-072 |
| `icelines-cli` | Binary entry point, CLI commands, TUI workbench, report/export commands, local state mutation surfaces. It adapts user intent to shared core/query/fetch boundaries. | REQ-WB-001; REQ-WB-003; REQ-STAT-001; REQ-REPORT-001; REQ-CODE-001 | IF-QUERY-001; IF-LAYOUT-001; IF-REPORT-001; IF-BUILD-001 | `COMMANDS.md`; VAL-001; VAL-002; VAL-009; VAL-010 |
| `icelines-web` | Axum HTML dashboard, JSON twins, bookmarkable URL state, browser recovery/empty states, safe POST-backed mutation boundaries. | REQ-WEB-001; REQ-WEB-002; REQ-WB-003; REQ-PARITY-001; REQ-FANTASY-001 | IF-WEB-001; IF-VIEW-001; IF-LAYOUT-001 | VAL-003; VAL-004; VAL-010 |
| `icelines-site` | Deferred mkdocs/static-site renderer. It remains in the workspace but is not the active surface baseline; exports remain active report artifacts. | REQ-REPORT-001; REQ-PARITY-001 | IF-REPORT-001; IF-VIEW-001 | surface-parity matrix; REVIEW.md |
| Report/export writers | Markdown, JSON, CSV, and public report artifacts from ViewModels or report ViewModel projections. | REQ-STAT-001; REQ-REPORT-001; REQ-PARITY-001 | IF-REPORT-001; IF-VIEW-001 | VAL-002; VAL-004 |
| Local state under `~/.icelines/` | Snapshots, installed data, config, SQLite groups/fantasy/watch state, named layouts, reports, and cached assets. | REQ-OFFLINE-001; REQ-DATA-DEPTH-001; REQ-FANTASY-001; REQ-FRESH-001; REQ-WB-003 | IF-FETCH-001; IF-DATA-001; IF-LAYOUT-001 | VAL-005; VAL-006; VAL-007; VAL-008; VAL-010 |
| Cargo workspace/features | Default all-surface build today; target feature-gated web/TUI/net boundaries and lean offline CLI. | REQ-DEP-001; REQ-LEAN-001; REQ-CODE-001 | IF-BUILD-001 | VERIFICATION.md EVID-DEP-001; EVID-LEAN-001 |

## Data Flow

```text
user intent
  -> CLI flags / TUI cmdbar / web params / export command
  -> icelines-query parser + planner
  -> icelines-fetch loader or local snapshot/install/cache read
  -> LoadOutcome { repo, missing, provenance, freshness }
  -> icelines-core StatsRepository keyed by (player_id, season, season_type)
  -> ViewModel with ViewContext, source_state, warnings, rows, empty_state
  -> CLI / TUI / Web HTML / Web JSON / report-export renderer
  -> user-visible result plus evidence-ready warnings and disclosures
```

### Source flow

```text
bundled data ----------\
installed bundles ------> source-specific loader -> LoadOutcome.missing/source_state
snapshots/cache --------/
live NHL/MoneyPuck/ESPN fetch -> validated snapshot/write path -> later read
```

There is no single universal fallback chain. Each source has its own allowed
path:

- bios/skater stats: snapshot tiers and embedded bundled data.
- goalies, transactions, playoffs: legacy/embedded/installed fallback where
  implemented.
- realtime, MoneyPuck, contracts: snapshot or opt-in fetch write path only;
  absence is `MissingSource`/unavailable state.
- live APIs: write path for `fetch` commands, not a query-time escape hatch.

### Target cache flow

```text
validated bundled/installed/snapshot source state
  -> major analytics cache builder
  -> versioned cache records with provenance/freshness/quality/invalidation
  -> IF-CACHE-001 consumer envelope
  -> dashboard / scout report / player card / line explorer / goalie view /
     practice focus / postgame review / agent summary
```

The target cache is not a second truth source. It is a reproducible evidence
layer over explicit source windows. Cache reads may return unavailable, stale,
partial, schema-incompatible, or unsupported-metric states, and consumer surfaces
must render those states instead of recomputing or zero-filling.

### Source obligations

| Source / Domain | Current Read Path | Absent / Failed Source Behavior | Design Obligation |
|---|---|---|---|
| Bios and skater stats | Snapshot tiers and embedded bundled data. | Missing or stale data is disclosed through source/completeness state. | Keep `(season, season_type)` in loader keys, cache keys, and ViewContext. |
| Goalies, transactions, playoffs | Legacy/embedded/installed fallback where implemented. | Absence is domain-specific unavailable or `MissingSource`, not a zero-shaped success. | Do not generalize this fallback to all sources without design evidence. |
| Realtime, MoneyPuck, contracts | Snapshot or opt-in fetch write path only. | Missing silo is a typed `MissingSource` or unavailable state. | Preserve missing-source identity through ViewModels, JSON, reports, and browser states. |
| External NHL API | `fetch` write path into validated local state. | 429/503/schema drift/integrity mismatch fail loudly or degrade explicitly. | No query path may silently call live APIs to fill missing data. |
| ESPN transactions | Fetch/write path with season-aware team abbreviation mapping. | Unknown team maps to `LEAGUE` with warning. | Carry warning and mapped team state into transactions surfaces. |
| Local SQLite state | Groups, fantasy, watch, and roster-local state under `~/.icelines/`. | Missing local state yields explicit setup/empty state. | Browser GET routes remain read-only; mutations require POST-backed or CLI/TUI paths. |
| Major analytics cache (target) | Explicit build over validated source state and explicit read contract. | Missing/stale/partial/unsupported/schema mismatch returns typed cache state. | Consumers use canonical cache envelopes and disclosure fields; no autonomous coaching authority or prediction claims. |

### Surface flow

```text
StatsRepository + query/result intent
  -> shared ViewModel/envelope
      -> CLI text / JSON / CSV adapter
      -> TUI workbench adapter
      -> Web HTML template
      -> Web JSON route
      -> Markdown/JSON/CSV report-export adapter
```

Renderer differences are allowed for density, pagination, styling, and
navigation. Renderer-local recomputation of ranking, filtering, source state,
classification, or report facts is an architecture defect.

## Boundary Rules

| Rule | Why It Matters | Review Lens |
|---|---|---|
| `icelines-core` stays pure: no network, no filesystem, no renderer-specific CSS/terminal layout. | Keeps domain logic reusable and fixture-testable. | FORGE; KEEL |
| `StatsRepository` is single-threaded and `!Send + !Sync`; async loader patterns must respect `spawn_local`/`LocalSet` constraints when the repo crosses async boundaries. | Prevents unsound thread handoff and accidental `Arc<Mutex<StatsRepository>>` designs. | HART; FORGE |
| Cache keys and invalidation include `(season, season_type)` whenever cached data depends on the active window. | Prevents silent wrong-season results after time travel. | HART; KEEL; EDGE |
| `LoadOutcome.missing` and source/completeness state must survive to renderers and reports. | Prevents wrong-but-confident output. | TAPE; WIRE; GLASS |
| Web routes use bookmarkable GET for reads and POST-backed paths for mutations; no hidden localStorage truth. | Preserves browser safety, recoverability, and shareable state. | broadcast; CREST |
| Public exports are ViewModel-backed and disclose descriptive scope near the top. | Keeps social/report artifacts reproducible and honest. | SCOUT; PACE; BENCH |
| Cache consumers use `IF-CACHE-001` records and may not locally recompute rankings, confidence, freshness, or source-state meaning. | Keeps future hockey decision screens consistent and auditable. | HART; Campbell; BENCH |
| Standalone and lean-build claims require `Cargo.toml` and feature evidence first. | Avoids claiming dependency independence before FLETCH/SLICE and feature gating are resolved. | KEEL; FORGE; BENCH |

## Dependencies

| Dependency | Purpose | Boundary / Risk | Verification |
|---|---|---|---|
| Public NHL API | Fresh bios, stats, schedules, scores, game details, realtime-capable source data. | No SLA; schema drift and season leakage must fail loud or degrade explicitly. | REQ-FRESH-001; VAL-008 |
| MoneyPuck CSVs | Optional advanced stat source. | Optional silo; absence is typed missing state, not zero. | REQ-FRESH-001; IF-FETCH-001 |
| ESPN transactions feed | Transaction source and team abbreviation mapping input. | Abbreviation drift and teamless rows require season-aware mapping or `LEAGUE` warning. | REQ-FRESH-001; VAL-008 |
| Bundled data via binary | Offline default analytics. | Skeleton/detail asymmetry must be disclosed. | REQ-OFFLINE-001; VAL-005 |
| Local snapshots/manifests | Cached/fetched data and installed seasons. | Integrity, schema version, and partial fetch state must be explicit. | REQ-DATA-DEPTH-001; REQ-FRESH-001 |
| SQLite under `~/.icelines/` | Groups, favorites, fantasy, watch/event state. | Local single-user state only; web mutation routes must not turn GET into writes. | REQ-FANTASY-001; REQ-WEB-001 |
| `ratatui` / `crossterm` | TUI workbench rendering and input. | Target lean CLI build must gate this out. | REQ-LEAN-001 |
| `axum` / `tower` / `tower-http` / `askama` | Web dashboard and JSON route serving. | Target lean CLI build must gate this out; browser safety remains explicit. | REQ-WEB-001; REQ-LEAN-001 |
| `reqwest` / `tokio` | Opt-in network fetch/write path. | Target offline CLI build must not require network runtime unless `net` is opted in. | REQ-LEAN-001; REQ-FRESH-001 |
| FLETCH / SLICE | Current cross-repo integration/dependency seams. | Blocks standalone claim until removed or replaced. | REQ-DEP-001; EVID-DEP-001 |

## Failure Modes

| Failure Mode | Impact | Mitigation | Evidence |
|---|---|---|---|
| Renderer recomputes ranking/filter/source state locally. | Surface drift; same question yields different answers. | ViewModel-only rendering rule; parity comparison in VAL-004. | REQ-PARITY-001; IF-VIEW-001 |
| Active season/type hidden or cache not invalidated after time travel. | User reads correct-looking wrong-season data. | Active context visibility and `(season, type)` cache-key review. | REQ-WB-002; VAL-001; VAL-003 |
| Missing source becomes zero or empty success. | Wrong-but-confident analytics and exports. | `MissingSource` and completeness vocabulary in IF-DATA-001. | REQ-DATA-001; VAL-005; VAL-008 |
| External API schema drift or newer snapshot schema is silently accepted. | Corrupt or misread data. | `deny_unknown_fields`, integrity check before deserialize, version refusal. | REQ-FRESH-001; IF-FETCH-001 |
| Historical perspective result overclaims meaning. | Public post implies era-adjusted, betting, predictive, or deployment-adjusted analysis that IceLines did not compute. | Report/export disclosure and anti-overclaim requirement. | REQ-STAT-001; REQ-REPORT-001; VAL-002 |
| Cache record is stale, partial, or from an incompatible schema but renders as confident success. | Future screens amplify plausible wrong decisions. | Versioned cache records with freshness/staleness, source-window, quality/completeness, invalidation, and typed unavailable states. | REQ-CACHE-001; REQ-CACHE-002; IF-CACHE-001; VAL-011 |
| Dashboard/report/card recomputes cache semantics locally. | Consumers disagree and traceability fails. | `IF-CACHE-001` consumer contract and review gate prohibit local recomputation of canonical analytics, confidence, or source-state meaning. | REQ-CACHE-003; REQ-CACHE-004 |
| Broad historical scenario hides edge cases. | Lockout, October rollover, ambiguous names, trades, and active streaks regress unnoticed. | Split VAL-002 evidence into discrete fixture observations. | REQ-STAT-002 |
| Web GET path mutates local state. | Unsafe browser behavior and surprising writes. | GET read-only rule; POST-backed mutation or explicit deferral. | REQ-WEB-001; REQ-FANTASY-001 |
| Lean/standalone target is claimed before dependency work lands. | Misleading release posture; hard-to-debug external coupling. | Target-only status until Cargo inspection and lean build evidence pass. | REQ-DEP-001; REQ-LEAN-001 |
| TUI App remains a god-object. | Slower changes and higher regression risk around workbench state. | Carry risk into `DESIGN.md`; isolate state boundaries and tests. | REVIEW.md; future DESIGN.md |

## Architecture Decisions

| ID | Decision | Status | Consequence |
|---|---|---|---|
| ADR-VT-001 | Treat active surfaces as CLI, TUI, Web, and export/report artifacts; treat mkdocs static site as deferred for this baseline. | accepted | Avoids stale "four active surfaces" claims while preserving export/static trace rows. |
| ADR-VT-002 | Use ViewModels/envelopes as the parity comparison artifact. | accepted | Validation compares semantic output, not screenshots or renderer layout. |
| ADR-VT-003 | Keep live APIs out of query-time fallback. | accepted | Offline/default reads remain deterministic; `fetch` is the write path into local state. |
| ADR-VT-004 | Mark FLETCH/SLICE removal and lean CLI build as target architecture. | accepted target | No standalone or lean compliance claim until dependencies/features prove it. |
| ADR-VT-005 | Keep `ARCHITECTURE.md` as a VTRACE bridge and leave implementation details to `DESIGN.md`. | accepted | Architecture records the shape; design will assign file/module-level decisions. |
| ADR-VT-006 | Treat the major analytics cache as the canonical future evidence layer for hockey decision surfaces. | accepted partial | Initial core cache envelope, strict store/read, internal consumer ViewModel, first named-cache Web report, first coach dashboard route, first opponent scout route, first player evidence-card route, first line-combination explorer route, and first goalie readiness route evidence exists; broader practice/postgame screens still cannot claim cache-backed implementation until their shipped-surface evidence exists. |

## Open Risks

| Risk | Impact | Disposition |
|---|---|---|
| FLETCH/SLICE dependency seams remain in `Cargo.toml`. | Standalone compliance cannot be claimed. | Track under REQ-DEP-001; solve in DESIGN/implementation. |
| Lean feature boundary is not implemented. | `--no-default-features --features cli` remains target evidence, not current evidence. | Track under REQ-LEAN-001; design Cargo feature map next. |
| TUI workbench state concentration remains high. | Harder to reason about cache invalidation, keyboard state, and tests. | Carry to DESIGN as a decomposition/refactor concern. |
| Validation rows are mostly pending. | Architecture is traceable but not yet proven by evidence. | Gate 2/3 must move evidence rows from pending to verified/validated. |
| Named/saved layout persistence is still a product target. | Mission customization promise is only partially met. | Carry as target requirement/design topic before claiming completion. |
| Major analytics cache is partially implemented with only the first named-cache report, first coach dashboard route, first opponent scout route, first player evidence-card route, first line-combination explorer route, and first goalie readiness route. | Future product copy could imply dashboards/reports/cards/line/goalie screens are broadly cache-backed. | Keep broader shipped-surface claims pending and require WP-009 consumer/copy evidence before production practice/postgame claims, broader goalie workflows, broader line-combination workflows, broader player-card workflows, broader scout workflows, and broader coach-dashboard expansion. |
