# Code Rigor

## Scope

Repo or feature: `icelines` repo-baseline VTRACE adoption.

Risk level: high for data trust, source-state disclosure, public report claims,
cross-surface parity, and dependency/build claims; medium for terminal/browser
presentation and local user-state flows.

Language/toolchain: Rust workspace with CLI, TUI, web, fetch, query, core,
report/export, local SQLite/cache state, bundled data, Markdown/JSON/CSV output,
and VTRACE documentation validated by `proof`.

This file defines the coding constraints that implementation waves must satisfy
before the Design decisions can be claimed as code-complete. Current docs-first
VTRACE work may record target-not-met rows, but implementation PRs must not use
target language as evidence.

## Coding Constraints

| ID | Constraint | Applies To | Verification | Exception Rule |
|---|---|---|---|---|
| CR-001 | Hand-authored functions should stay reviewable; default soft cap is 60 logical lines and one primary responsibility. | Critical Rust code, render adapters, fetch state machines, query planners | size/complexity check and review | Larger units require a comment or review note naming the invariant that keeps the larger unit safer than splitting. |
| CR-002 | Complex control flow must be bounded by typed states, tests, or documented invariants. | TUI state, fetch/sync, parser/planner, route handlers, report generation | design inspection, unit tests, fixture tests | Waive only when the branch structure mirrors an external protocol or format and has fixture coverage. |
| CR-003 | Invalid inputs and errors must be explicit and typed at public boundaries. | CLI args, TUI command bar, web params, JSON APIs, fetch inputs, file/cache reads | interface tests, route tests, command tests | Impossible states may use assertions only after construction paths enforce them. |
| CR-004 | Critical invariants require assertions, type structure, tests, property tests, or inspection evidence. | key shapes, source state, query intent, report disclosure, feature gates | tests and review | If enforced by a shared builder, link to that builder and a test that covers it. |
| CR-005 | Formatter, compiler warnings, clippy, and affected test slices must be clean before code-complete status, using the verification command matrix below unless an implementation PR records a narrower affected-slice rationale. | Whole changed implementation scope | workspace command matrix or documented affected-slice equivalent | Waivers need owner, exact diagnostic, risk, and revisit trigger. |
| CR-006 | ViewModel builders own hockey semantics; renderers only adapt already-built semantics to a surface. | `icelines-core/src/view_model/*`, CLI/TUI/web/report adapters | review plus parity fixtures | Renderer-local ranking/filtering/scoring is allowed only for purely visual ordering explicitly marked non-semantic. |
| CR-007 | `ViewContext` must include active window, query/filter/sort signature, source/completeness state, and relevant source generation before rendering analytical output. | ViewModels, reports, JSON twins, CLI/TUI/web rendering | ViewModel tests, JSON snapshot, report snapshot, route inspection | A view may omit fields only if the omitted axis cannot change the meaning of the result. |
| CR-008 | `MissingSource`, stale, partial, and unavailable states must not become zeroes, empty success, omitted JSON, or silent fallbacks. | loaders, repositories, ViewModels, renderers, reports | source-state propagation tests and review | Explicit empty states are allowed only when source availability is known and the result is truly empty. |
| CR-009 | Query-time reads must not call live APIs to hide missing local or bundled data. | CLI queries, TUI workbench, web routes, JSON APIs, report/export | code search/review, offline smoke, route tests | Live access is allowed only through explicit fetch/sync/write commands or a later approved design change. |
| CR-010 | Surface inputs must lower through shared parser/planner or typed adapters before execution. | CLI args, TUI commands, web params, AI fallback output, URLs | parser fixtures, command/route tests | A local adapter may normalize syntax but must not implement a competing hockey planner. |
| CR-011 | Domain reads and semantic caches must preserve the key shapes from `DESIGN.md`. | repositories, ViewModel caches, TUI state, route handlers, reports | key-shape tests, cache invalidation tests, review | Cache-key omissions require a comment or test proving the omitted axis is identity-stable or source-stable. |
| CR-012 | Long-lived semantic state belongs to TUI workbench state unless a later design defines another cache key and invalidation contract. | TUI `App`/workbench state, web server state, CLI/report commands | review, TUI cache tests, web route tests | Web/server shared immutable config/assets are allowed; hidden semantic hockey caches are not. |
| CR-013 | TUI state changes must invalidate or rebuild derived ViewModels when active window, typed query, selected source, or source generation changes. | `icelines-cli/src/tui/*`, `icelines-core/src/workbench.rs` | targeted state tests or demo transcript | Temporary monolithic `App` state is acceptable if new semantic logic lands behind named helper/state slices. |
| CR-014 | Web GET routes are read-only and bookmarkable; mutations are POST-backed or explicitly deferred to CLI/TUI. | web handlers, dashboard command parser, JSON APIs | route tests, no-JS/browser inspection | A GET route may trigger idempotent framework/static work but not local hockey/user-state mutation. |
| CR-015 | Browser-visible state must keep active context, source/status, recovery, and non-color state carriers visible. | HTML templates, CSS/status badges, JSON twins, no-JS states | browser/no-JS inspection, route snapshots, accessibility review | Color-only or icon-only state is not acceptable without adjacent text/aria/report labels. |
| CR-016 | Reports and exports must disclose scope, source/completeness state, filters/sort/scoring scheme, warnings, omissions, and generated context near the top. | Markdown, JSON, CSV, social/report commands | snapshot tests and text review | Machine formats may use metadata fields instead of prose but must carry equivalent facts. |
| CR-017 | Public historical/perspective copy must stay descriptive and avoid era-adjusted, predictive, betting, deployment-adjusted, or linemate-adjusted claims unless a later requirement and evidence authorize them. | reports, web copy, CLI explanations, docs examples | snapshot/text review | Comparison labels may be used if they are explicitly scoped to available data and fixture evidence. |
| CR-018 | Fetch, snapshot, and cache data must pass integrity/version checks before deserialization-dependent use. | `icelines-fetch`, installed snapshots, local caches, bundled data loaders | integrity/schema fixture tests | Best-effort parsing is allowed only for explicitly quarantined diagnostic commands that do not feed user-facing answers. |
| CR-019 | Upstream failure handling must distinguish absence, rate limit, unavailable service, schema drift, integrity mismatch, newer schema, and partial write. | NHL/ESPN/MoneyPuck/API clients, sync engine, fetch commands | httpmock/tempdir fixtures, error snapshots | Grouping errors is allowed only at final display if structured details remain available in logs/result state. |
| CR-020 | Local state changes must preserve existing `~/.icelines/` config, snapshots, FantasyDb, favorites, watch rules, named layouts, reports, and cache directories unless a migration plan is recorded. | data install, fetch/sync, fantasy, favorites, watch, layout, report storage | tempdir migration tests, review | Destructive migrations require explicit backup/rollback steps and a VTRACE review entry. |
| CR-021 | Lean CLI and standalone dependency claims require build and dependency evidence before status changes. | Cargo features, workspace manifests, release notes, docs | dependency inspection, `cargo build --no-default-features --features cli` after feature work | Until evidence passes, rows remain `target-not-met`, not `passed`. |
| CR-022 | FLETCH/SLICE removal must account for every affected command or selector by replacement, explicit refusal, compatibility shim, or rollback note. | `icelines-fetch`, `icelines-query`, command surfaces, docs | dependency inspection plus command-surface verification | Removing dependency lines alone is not sufficient evidence. |
| CR-023 | Evidence must be linked to commands, fixtures, snapshots, route tests, demos, or explicit target-not-met rows. | VTRACE `VALIDATION.md`, `VERIFICATION.md`, `REVIEW.md` | evidence-led review | Prose-only review can approve design/docs but cannot close implementation evidence. |
| CR-024 | Rust model-safety boundaries must stay explicit: `StatsRepository` remains intentionally local/non-shared unless redesigned, async UI/web handoffs use `spawn_local`/`LocalSet` or typed owned DTOs as appropriate, `PlayerView<'_>` does not escape repository lifetimes, `LoadOutcome` is not hidden behind broad `Arc<Mutex<_>>` sharing, and `icelines-core` remains I/O-free. | `icelines-core`, query/web/TUI adapters, async handoffs, repository/ViewModel builders | compile-time bounds, targeted tests, code review, dependency review | Any Send/Sync or shared-state relaxation requires a design update naming cache key, lifetime, mutation, and source-state consequences. |
| CR-025 | Data-edge behavior must be fixture-backed for season ID leakage, ESPN/team abbreviation drift, Unicode and duplicate-name disambiguation, games-played threshold boundaries, active/current season rollover, lockout/skeleton seasons, and trade multi-stint preservation. | parsers, identity, source loaders, stats repository, ViewModels, reports | data-edge fixtures and snapshot review | Missing fixture coverage must be recorded as pending evidence, not closed. |
| CR-026 | Formula and methodology code must expose uncertainty in types and labels: pace helpers return optional values when games played are insufficient, `BelowThreshold` or equivalent status survives to renderers, `MIN_GP` and fit thresholds have boundary tests, known-value examples document intent, and complexity claims are measured or clearly labeled. | scoring, pace, fit, report/perspective methods, public copy | unit/property tests, known-value fixtures, text review | A formula may be simpler than the constraint if the output is explicitly labeled descriptive and unsupported axes are disclosed. |
| CR-027 | Browser HTTP and local-operability defaults must be safe and inspectable: host-header/DNS-rebinding posture is explicit, CORS defaults closed unless configured, assets use correct MIME/cache/ETag behavior where served, the URL is printed before auto-open, `0.0.0.0` binds warn, and viewport/touch/focus states are checked. | `icelines-web`, CLI web launch command, templates/assets, route tests | route tests, browser/no-JS inspection, launch smoke | Public network exposure requires explicit user opt-in and review notes. |
| CR-028 | Deployment, line chemistry, special-teams role, injury state, and linemate context are annotations or limits unless separately modeled and evidenced. | reports, depth charts, web/CLI narrative copy, social/public artifacts | snapshot/text review and model trace | Such context may appear as caveat language or source annotations, not as causal explanation. |
| CR-029 | Rust boundary hygiene is mandatory: library code avoids bare `unwrap()`, invariants use explicit `expect()` messages, public crate errors use typed error enums rather than broad `Box<dyn Error>` return positions, external API response structs fail loudly on schema drift, and the workspace dependency graph keeps `icelines-core` below I/O/async crates. | Rust libraries, API clients, workspace manifests, public crate boundaries | clippy/review, error-path tests, serde drift fixtures, dependency inspection | CLI binaries may use `anyhow` at the final user-facing boundary if structured lower-level errors remain intact. |
| CR-030 | Canonical model invariants are code-rigor gates: stint sums equal totals, stint ordering is monotonic, roster upserts preserve last-stint/all-stint indexes, LRU maps are bijective, `eligible_pos` remains singular unless redesigned, and goalie identity uses stats-backed goalie data rather than position text alone. | `StatsRepository`, identity, season stats, loaders, roster/depth builders | invariant tests, compile-fail doctests, fixture snapshots | Any new model shape requires HART/KEEL design review before implementation closure. |
| CR-031 | External-source reliability policy must be testable: 429 honors `Retry-After` or bounded backoff, 503 and maintenance windows surface actionable errors, partial upstream writes are rejected or explicitly flagged resumable, newer snapshot/schema versions refuse clearly, and MoneyPuck/CSV encoding or column drift fails at the boundary. | fetch clients, snapshot store, MoneyPuck/ESPN/NHL loaders, sync commands | httpmock/tempdir fixtures, error snapshots, schema/encoding tests | Final display may simplify language only if structured error state remains available for review/debug output. |
| CR-032 | Shared visual and accessibility contracts are implementation constraints: fit/color tokens come from one contract, renderer-local threshold/color tables are prohibited, active `(season, season_type)` stays visible, terminal output remains readable at standard widths where claimed, browser/report layouts keep semantic structure, and empty/loading/partial states have designed recovery paths. | TUI, CLI tables, web HTML, reports/exports, CSS/templates | renderer snapshots, accessibility/no-color review, browser/terminal inspection | Surface-specific presentation may vary, but semantic colors, labels, and context cannot drift. |
| CR-033 | Major analytics cache records are canonical evidence, not convenience blobs: they must be versioned, scope-keyed, source-windowed, provenance/freshness/quality annotated, invalidation-aware, no-live on read, and consumed without renderer-local recomputation of source-state, confidence, or methodology meaning. | `icelines-core::analytics_cache`, `icelines-fetch::analytics_cache_store`, `icelines-core::view_model::analytics_cache_consumer`, `icelines-web::handlers::analytics_cache_report`, future cache builders/readers, dashboard/report/card/line/goalie/practice/postgame consumers, agent surfaces | schema fixtures, tempdir source-state fixtures, invalidation/rebuild fixtures, consumer contract demos, route fixtures, text review | Initial core schema/source/consumer fixtures passed in WP-009 pulse 02; strict store/read/invalidation fixtures passed in pulse 03; internal downstream consumer ViewModel fixtures passed in pulse 04; first named-cache Web report fixtures passed in pulse 05; first coach dashboard route fixtures passed in pulse 06; first opponent scout route fixtures passed in pulse 07; first player evidence-card route fixtures passed in pulse 08; first line-combination explorer route fixtures passed in pulse 09; broader shipped downstream consumers stay pending, and no consumer may claim cache-backed analytics from ad hoc local calculations. |

## Tailoring

| Area | Rule | Rationale |
|---|---|---|
| Review size | Prefer small implementation waves by surface or data boundary; do not refactor CLI, TUI, web, fetch, reports, and Cargo features in one PR unless mechanically generated and separately evidenced. | Cross-surface behavior can regress silently when many adapters change at once. |
| ViewModel purity | New hockey semantics must land in core/query/report ViewModel builders or typed projections, not renderer string-formatting code. | Keeps parity and source-state disclosure enforceable. |
| Key-shape discipline | Any new repository read, cache, or ViewModel result that depends on season/type must include `(season, season_type)` or prove it is stable without them. | Prevents regular/playoff/current-season drift. |
| Source-state discipline | New data-source code must return typed unavailable/partial/missing/freshness state at the boundary that first knows it. | Avoids false confidence and zero-shaped failures. |
| Query discipline | Surface adapters may normalize syntax and user-friendly aliases; shared parser/planner owns meaning. | Prevents CLI/TUI/web divergence. |
| TUI state | New persistent semantic state belongs in named workbench/App state slices or helpers with reset/invalidation behavior. | Reduces risk from a concentrated event-loop state object. |
| Web state | User-controllable read state is URL-visible and allowlisted; user/local-state mutations are POST-backed or deferred. | Preserves bookmarkability, safety, and no-JS behavior. |
| Reports | Public artifacts must begin with scope/source disclosure before ranking or narrative claims. | Public sharing amplifies stale or partial-data mistakes. |
| Fetch/integrity | Validate bytes/schema/version before use; distinguish retryable, unavailable, incompatible, and partial-write states. | Upstream drift must not produce plausible hockey output. |
| Evidence tiers | Use L0 unit/property tests for pure invariants, L1 tempdir/httpmock fixtures for I/O/fetch, L2 subprocess/route/browser/snapshot evidence for surfaces. | Matches the evidence tier allocation in `DESIGN.md`. |
| Cargo claims | Treat `REQ-DEP-001` and `REQ-LEAN-001` as target-not-met until dependency graph and lean build evidence pass. | Prevents overclaiming standalone or lean CLI support. |
| Static site | Do not treat `icelines-site` as active VTRACE surface evidence unless a later design reactivates static site generation. | Architecture/design deferred mkdocs/static-site for this baseline. |
| Rust safety | Treat repository lifetime, Send/Sync, async handoff, and I/O-free core boundaries as design-level invariants, not incidental compiler details. | Prevents unsafe shared-state fixes during web/TUI integration. |
| Data edges | Every known historical/upstream edge gets a fixture, snapshot, or explicit pending-evidence row before closure. | Avoids broad "works on current season" evidence. |
| Methodology | Formulas carry threshold and uncertainty state through types, labels, and tests. | Prevents overconfident pace/fit/report output. |
| Web operability | Browser launch and HTTP serving behavior must be safe on local machines and understandable without JS. | Prevents accidental exposure and inaccessible recovery states. |
| Rust API hygiene | Error, schema, and dependency boundaries are reviewed as part of correctness, not style. | Prevents panic-shaped failures and hidden upstream drift. |
| Model invariants | HART invariants are named as gates even when the changed file is not `StatsRepository`. | Prevents model drift through adapters, reports, or loaders. |
| Upstream reliability | Retry, resumability, schema, encoding, and version policy are fixture-backed. | Prevents network/API drift from becoming plausible wrong hockey output. |
| Visual contract | Shared color/context/accessibility rules are centralized and tested across active surfaces. | Prevents readable-but-divergent UI claims. |
| Analytics cache | Cache-backed analytics must carry one source-state and methodology contract from builder to consumer. | Prevents future hockey screens from overclaiming stale, partial, unsupported, or locally recomputed evidence. |

## Required Review Checklist

Use this checklist for implementation PRs or review entries that claim closure of
any VTRACE row.

| Check | Required Question |
|---|---|
| Requirement trace | Which `REQ-*`, `IF-*`, `DES-*`, and `VAL-*` rows does this change satisfy or affect? |
| Surface parity | Does CLI, TUI, web HTML, web JSON, and report/export behavior still share row identity, context, warnings, source state, and empty-state semantics where applicable? |
| Source state | Can missing/stale/partial/unavailable data still be seen by the final user or machine consumer? |
| Active context | Is `(season, season_type)` visible or encoded where the result depends on it? |
| Key shape | Did every cache/repository/read model include the axes needed to avoid stale or cross-window answers? |
| Query path | Does the change reuse parser/planner/typed adapters rather than reimplementing query meaning? |
| Browser safety | Are GET routes read-only, URL state allowlisted, mutation paths POST-backed, and no-JS states understandable? |
| Report safety | Does public copy disclose scope/source limits and avoid unsupported claims? |
| Fetch safety | Are integrity/schema/version and upstream failure states checked before use? |
| Local data | Are existing user config, snapshots, FantasyDb, favorites, watch rules, reports, and cache state preserved? |
| Build/deps | Are feature/dependency claims backed by dependency inspection and build evidence, or still marked target-not-met? |
| Rust model safety | Did repository lifetimes, Send/Sync boundaries, async handoffs, and I/O-free core constraints remain intact? |
| Data edges | Did the change touch any edge covered by lockout, rollover, team-abbrev drift, duplicate names, Unicode, GP thresholds, or multi-stint trades? |
| Formula/methodology | Are thresholds, insufficient-data states, known-value examples, and measured-vs-labeled complexity claims preserved? |
| Browser operability | Are host binding, CORS, MIME/cache behavior, auto-open, 0.0.0.0 warnings, viewport, touch, and focus states safe and inspectable where applicable? |
| Rust API hygiene | Are library panics, broad error erasure, external schema drift, and dependency-direction violations avoided? |
| Model invariants | Do HART invariants, singular eligibility, goalie discrimination, roster indexes, LRU bijection, and compile-time borrow/Send fences still hold? |
| Upstream reliability | Are 429/503, Retry-After/backoff, partial writes, newer schemas, CSV encoding, and source absence tested or explicitly pending? |
| Visual contract | Do fit colors/tokens, active context, semantic HTML/terminal readability, no-color cues, and recovery states stay consistent? |
| Analytics cache | If cache-backed analytics are touched, are version, scope, source window, provenance, freshness, quality, invalidation, warnings, disclosures, and consumer-contract behavior tested without query-time live fetch? |
| Evidence | Is there repeatable command, fixture, snapshot, route/browser, demo, or explicit target-not-met evidence? |

## Verification Command Matrix

Implementation PRs that claim code-complete status should run the broad command
set below or record an affected-slice rationale that is at least as strict for
the files touched. Documentation-only changes may use `proof check` and
`git diff --check`.

| Scope | Default Command / Evidence | When Required |
|---|---|---|
| Formatting | `cargo fmt --check` | Rust source or manifest-adjacent code changes. |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Rust implementation changes unless the workspace has a documented narrower package target for the wave. |
| Unit/integration tests | `cargo test --workspace --all-targets` or affected package/test commands with rationale. | Core, query, fetch, web, TUI, report/export, or data changes. |
| CLI subprocess evidence | Named CLI commands with fixed fixtures/tempdirs and captured output. | CLI behavior, reports, exports, exit status, or JSON/CSV changes. |
| Fetch/upstream evidence | Tempdir and mock-server fixtures covering success, absence, 429, 503, schema drift, integrity mismatch, and partial write where touched. | Fetch, snapshot, cache, upstream, or install changes. |
| Major analytics cache | Schema fixtures, source-state/no-live fixtures, invalidation/rebuild fixtures, consumer envelope demos, and public-copy text review. | Cache record, build/read, invalidation, or downstream consumer changes. |
| Web evidence | Route tests plus no-JS/browser inspection for active context, recovery, focus/touch, CORS/host posture, MIME/cache behavior, and JSON twins where touched. | Web handlers, assets, templates, browser state, launch command. |
| TUI evidence | Targeted state/cache tests or transcript/snapshot demo with active-window switch and source notice survival. | TUI/workbench state, command bar, panes, cache behavior. |
| Docs/VTRACE | `C:\src\proof\target\debug\proof.exe check <docs\vtrace> --errors-only`; `git diff --check`. | VTRACE documentation updates. |

## Exceptions / Waivers

No exceptions are approved for the VTRACE docs baseline.

| ID | Constraint | Exception | Rationale | Owner | Revisit Trigger |
|---|---|---|---|---|---|
| None | n/a | n/a | n/a | n/a | n/a |

Future waivers must be recorded with:

- the exact constraint ID,
- affected files/commands,
- why the safer path is not available,
- user-visible risk,
- owner,
- expiration or revisit trigger,
- evidence that the waiver does not invalidate a requirement claim.

## Verification Evidence

| Evidence ID | Constraint IDs | Command / Review | Result | Evidence Pointer |
|---|---|---|---|---|
| EVID-CR-001 | CR-001; CR-002; CR-004 | Implementation review for size, control flow, and invariants. | pending | REVIEW.md |
| EVID-CR-002 | CR-005 | Verification command matrix or documented affected-slice equivalent. | passed_with_risk: WP-008 pulse 01 passed `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-targets` after refreshing stale Lindsay L3 golden outputs, fixing the MDI layout test initializer lint, and retiring the no-live/date and plain no-live/dry-run persona regressions while preserving `--for-favorites` no-live refusal | VERIFICATION.md; context/waves/2026-06-01-vtrace-wp008-integration/pulses/pulse-01.md |
| EVID-CR-003 | CR-006; CR-007; CR-008; CR-010; CR-011 | ViewModel/query/key-shape unit and fixture tests. | pending | VERIFICATION.md; VAL-004 |
| EVID-CR-004 | CR-009; CR-018; CR-019 | Offline/fetch/integrity/upstream failure fixtures or mocks. | passed_with_risk: WP-005 pulses 01-13 cover selected snapshot seal/refusal, no-live/offline smoke, upstream retry/failure, data/fetch command transcript, snapshot integrity/missing-file, chunked snapshot schema, MoneyPuck CSV drift, FLETCH cache/refresh fallback, player landing schema-drift, abbreviation-drift, missing-source, and partial-refresh resume/flag boundaries | VAL-005; VAL-006; VAL-008 |
| EVID-CR-005 | CR-012; CR-013 | TUI state/cache invalidation tests or demo transcript. | passed_with_risk: selected non-interactive TUI leaders/layout evidence and the WP-008 MDI layout test initializer fix support closure; full interactive TUI transcript remains residual risk | VAL-001; EVID-WP008-REHEARSAL-L1 |
| EVID-CR-006 | CR-014; CR-015 | Web route/no-JS/browser checks and JSON twin snapshots. | pending | VAL-003; VAL-004 |
| EVID-CR-007 | CR-016; CR-017 | Report/export snapshots and text review. | pending | VAL-002; VAL-004 |
| EVID-CR-008 | CR-020 | Local-state migration/tempdir tests when storage changes. | pending | VAL-006; VAL-007 |
| EVID-CR-009 | CR-021; CR-022 | Dependency inspection and lean CLI build after feature work. | target-not-met_dispositioned: WP-007 pulse 01 records FLETCH path dependency, direct/transitive SLICE git dependencies, affected FLETCH command surfaces, affected SLICE selector surface, and missing `cli` feature; no standalone/lean claim is made | EVID-DEP-001; EVID-LEAN-001; EVID-WP007-DEP-INVENTORY-L0 |
| EVID-CR-010 | CR-023 | VTRACE review entry linking evidence to claims. | passed_with_risk: WP-008 closeout review links broad gates, golden refresh, MDI lint fix, validation dispositions, and WP-007 target-not-met dependency/lean separation | REVIEW.md; context/waves/2026-06-01-vtrace-wp008-integration/pulses/pulse-01.md |
| EVID-CR-011 | CR-024 | Compile-time/lifetime review for repository sharing, `PlayerView<'_>`, async handoff, broad mutex avoidance, and core I/O-free dependency review. | pending | VERIFICATION.md |
| EVID-CR-012 | CR-025 | Data-edge fixtures for season ID leakage, team-abbrev drift, Unicode/duplicate names, GP thresholds, rollover, lockout/skeleton seasons, and multi-stint trades. | partial: WP-004 pulses 04-08 cover selected duplicate/Unicode, GP threshold, rollover/lockout, and multi-stint trade boundaries; WP-005 pulse 11 covers selected team-abbrev drift boundaries | VAL-002; VAL-008 |
| EVID-CR-013 | CR-026; CR-028 | Formula/methodology and report-copy snapshots with threshold, insufficient-data, annotation, and unsupported-context checks. | pending | VAL-002; VAL-004 |
| EVID-CR-014 | CR-027 | Browser HTTP/local-operability route and launch evidence. | pending | VAL-003 |
| EVID-CR-015 | CR-029 | Error-path tests, schema-drift fixtures, dependency inspection, and review of library panic boundaries. | pending | VERIFICATION.md |
| EVID-CR-016 | CR-030 | HART invariant tests and compile-fail doctests for borrow/Send fences where feasible. | pending | VERIFICATION.md; VAL-002 |
| EVID-CR-017 | CR-031 | HTTP/mock, tempdir, schema-version, CSV encoding, and partial-write/resume fixtures. | passed_with_risk: WP-005 pulse 04 covers selected HTTP/mock retry failure behavior, pulse 05 covers selected no-live fetch command refusals before live client construction, pulse 06 covers selected snapshot integrity/missing-file behavior, pulse 07 covers selected chunked snapshot schema-version behavior, pulse 08 covers selected MoneyPuck CSV required-column and malformed-row drift behavior, pulse 09 covers selected generic FLETCH HTTP cache/refresh fallback behavior, pulse 10 covers selected player landing schema-drift behavior, pulse 11 covers selected abbreviation-drift behavior, pulse 12 covers selected player landing missing-source behavior, and pulse 13 covers selected partial-refresh resume/flag behavior | VAL-005; VAL-006; VAL-008 |
| EVID-CR-018 | CR-032 | TUI/CLI/web/report visual-token snapshots and accessibility/no-color/context review. | pending | VAL-001; VAL-003; VAL-004 |
| EVID-CR-033 | CR-033 | Major analytics cache schema/source-state/invalidation/consumer contract evidence. | partial: WP-009 pulse 02 adds selected core schema serde, local snapshot source-state preservation, live-fetch-source refusal, newer-schema refusal, unsupported metric refusal, consumer-envelope preservation, contract mismatch refusal, and unsupported-consumer refusal. Pulse 03 adds strict store/read tempdir fixtures for missing, stale, partial, missing-source, schema, unsupported-metric, invalidation, failed-rebuild rollback, and store-backed consumer-envelope behavior. Pulse 04 adds an internal downstream consumer ViewModel fixture and store-backed coach-dashboard feed fixture that preserve envelope semantics without recomputation. Pulse 05 adds named-cache Web report fixtures that preserve cache evidence through HTML and JSON while rendering missing-cache unavailable state explicitly. Broader shipped downstream surfaces remain pending. | VAL-011; IF-CACHE-001; EVID-WP009-CACHE-SPEC-L0; EVID-WP009-CACHE-SCHEMA-L0; EVID-WP009-CACHE-STORE-L1; EVID-WP009-CACHE-CONSUMER-L2; EVID-WP009-CACHE-WEB-L2 |

## Current Open Rigor Risks

| Risk | Constraint IDs | Current Disposition |
|---|---|---|
| ViewModel/source-state propagation has not yet been proven across every renderer. | CR-006; CR-007; CR-008; CR-023 | Carry to verification evidence and parity fixtures. |
| TUI state may remain concentrated in large workbench/App structures. | CR-001; CR-002; CR-012; CR-013 | Allow temporarily, but require named helper/state-slice discipline for new semantics. |
| Web browser guarantees are not yet backed by route/browser evidence. | CR-014; CR-015; CR-023 | Carry to VAL-003 and route tests. |
| Fetch and upstream drift behavior needs fixture evidence. | CR-018; CR-019 | Carry to VAL-006 and VAL-008. |
| Rust model-safety boundaries need explicit evidence before web/TUI async refactors claim closure. | CR-024 | Carry to verification and code review evidence. |
| Data-edge fixture coverage is broader than current proof. | CR-025 | Carry to VAL-002 and VAL-008. |
| Formula/methodology thresholds and unsupported-context copy need snapshot evidence. | CR-026; CR-028 | Carry to report/export and parity evidence. |
| Browser HTTP operability needs launch/route/browser evidence. | CR-027 | Carry to VAL-003. |
| Rust error/schema/dependency boundaries need explicit evidence before implementation closure. | CR-029 | Carry to verification command matrix and code review evidence. |
| HART invariants and compile-time fences need named test evidence. | CR-030 | Carry to model fixture and compile-fail evidence. |
| Upstream retry/resume/schema/encoding policy needs fixture evidence. | CR-031 | Carry to fetch and snapshot verification evidence. |
| Shared visual/accessibility contracts need cross-surface snapshots or inspection. | CR-032 | Carry to surface parity evidence. |
| Major analytics cache implementation is partial. | CR-033 | Initial core schema/source/consumer, strict store/read/invalidation, internal consumer ViewModel, and first named-cache Web report fixtures passed; keep broader shipped downstream surfaces and release claims pending until their evidence passes. |
| FLETCH/SLICE and lean CLI are target-not-met_dispositioned. | CR-021; CR-022 | Do not claim standalone or lean compliance until dependency/build evidence passes; owner: maintainer/release lens; revisit when dependency removal and Cargo feature-gating PR is ready. |
| Static site is deferred while workspace files still exist. | CR-023 | Do not use static-site presence as active surface evidence. |
