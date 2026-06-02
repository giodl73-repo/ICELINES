# Integration Plan

## Scope

Repo or feature: `icelines` repo-baseline VTRACE implementation.

Integration is required for work packages that cross CLI, TUI, Web, JSON,
report/export, local-state, fetch, or Cargo feature boundaries. This plan does
not replace package-level verification; it records when package outputs become
an integrated product.

## Integration Items

| ID | Product / Component | Producer | Consumer | Interface | Verification | Status |
|---|---|---|---|---|---|---|
| INT-001 | Source-state and ViewModel parity envelope | `icelines-core`; `icelines-query`; `icelines-fetch` | CLI, TUI, Web HTML, Web JSON, reports | IF-DATA-001; IF-VIEW-001; IF-QUERY-001; IF-WEB-001; IF-REPORT-001 | WP-001 L0/L1 plus `VAL-004` parity evidence | closed_with_risk for WP-001: pulses 01-34 prove selected leaders parity through CLI JSON/text/CSV, Web JSON/HTML, Markdown export, and selected TUI snapshots; broader browser/accessibility, full report/export, broader provenance, query-planner, and final integration-rehearsal risks route to WP-003, WP-004, WP-005, and WP-008 |
| INT-002 | Named layout state model | Shared layout/local-state implementation | TUI workbench, Web dashboard, local config/state | IF-LAYOUT-001; IF-WEB-001 | WP-002 schema tests plus `VAL-010` TUI/Web restore demo | proposed |
| INT-003 | Browser route and URL state behavior | `icelines-web`; CLI serve command | Browser users, bookmarks, automation, JSON clients | IF-WEB-001; IF-VIEW-001 | WP-003 route tests and browser/no-JS inspection | closed_with_risk; pulses 01-07 selected season-type, favorites, streaks, scoring, Admin data-status, no-JS/viewport/recovery shell, and serve launch/bind boundaries passed_with_risk; live-browser/touch-focus/full-JSON-matrix residual risk routes to WP-008 |
| INT-004 | Historical report/export artifacts | Report ViewModels/export commands | Markdown, JSON, CSV, public-copy reviewers | IF-REPORT-001; IF-VIEW-001 | WP-004 snapshots and text review | closed_with_risk; pulses 01-08 selected Markdown export public-copy disclosure guardrail, active-streak status label, completeness/skeleton disclosure, GP-threshold evidence, duplicate/Unicode-name evidence, trade-continuity evidence, lockout/October rollover season-window evidence, and full-lockout season skip evidence passed_with_risk; full report/export matrix residual risk routes to WP-008 |
| INT-005 | Offline/fetch/source reliability path | `icelines-fetch`; CLI data/fetch commands | `StatsRepository`, ViewModels, user commands | IF-FETCH-001; IF-DATA-001 | WP-005 tempdir/httpmock fixtures and command transcript | closed_with_risk; pulses 01-13 selected snapshot seal/refusal, offline/query smoke, shift capability refusal, upstream retry/failure, data/fetch command transcript, snapshot integrity/missing-file, chunked snapshot schema, MoneyPuck CSV drift, FLETCH cache/refresh fallback, player landing schema-drift, abbreviation-drift, missing-source, and partial-refresh resume/flag evidence passed_with_risk; broader transcript breadth routes to WP-008 |
| INT-006 | Fantasy read/local-state flows | Fantasy ViewModels and local-state adapters | CLI, TUI, Web, local SQLite/FantasyDb | IF-VIEW-001; IF-WEB-001 | WP-006 read/mutation-deferral evidence | closed_with_risk; pulses 01-04 selected fantasy JSON local-state/cache-read, existing-FantasyDb read-only, poach imported-availability read-only, and final VAL-007 transcript boundaries passed_with_risk |
| INT-007 | Dependency and lean build boundary | Cargo workspace and package manifests | Maintainers, release scripts, downstream users | IF-BUILD-001 | WP-007 dependency inspection and lean build smoke | target-not-met_dispositioned; pulse 01 records blockers and release-owner revisit trigger |
| INT-008 | End-to-end validation rehearsal | Completed or dispositioned work packages | VTRACE release/readiness gate | All touched interfaces | WP-008 validation rehearsal and trace/review alignment | closed_with_risk; pulse 01 passed broad workspace format/clippy/test gates, refreshed stale Lindsay L3 goldens, aligned VTRACE closeout rows, and keeps dependency/lean support target-not-met |
| INT-009 | Major analytics cache evidence layer | `icelines-core::analytics_cache` initial schema/consumer contract; `icelines-fetch::analytics_cache_store` strict store/read path; internal consumer ViewModel fixture; `icelines-web` named-cache report, coach dashboard, opponent scout, player evidence-card, line-combination explorer, goalie readiness, practice focus, postgame review, and postgame adjustment-review routes | Coach dashboard, opponent scout report, player evidence card, line explorer, goalie view, practice/postgame reports, and agent surfaces | IF-CACHE-001; IF-DATA-001; IF-VIEW-001; IF-REPORT-001 | WP-009 schema/source-state/invalidation/consumer contract and Web route evidence | partial; pulses 02-13 pass the initial core contract, strict store/read path, internal dashboard-style consumer fixture, first named-cache Web report/JSON twin, first coach dashboard route/JSON twin, first opponent scout route/JSON twin, first player evidence-card route/JSON twin, first line-combination explorer route/JSON twin, first goalie readiness route/JSON twin, first practice focus route/JSON twin, first postgame review route/JSON twin, and first postgame adjustment-review route/JSON twin, while broader downstream cache-backed product claims remain pending |

## Integration Sequence

| Order | Item | Prerequisites | Verification |
|---:|---|---|---|
| 1 | INT-001 | Baseline controls accepted; WP-001 implementation slice ready. | ViewModel/query/source-state tests and parity fixture/demo. |
| 2 | INT-002 | INT-001 context rules available where layouts include active context policy. | Layout schema tests, local-state preservation, TUI/Web restore demo. |
| 3 | INT-003 | INT-001 ready; INT-002 if dashboard layouts are in scope. | Route/browser/no-JS/URL-state checks. |
| 4 | INT-004 | INT-001 source/report semantics stable. | Report/export snapshots and text review. |
| 5 | INT-005 | INT-001 source-state vocabulary stable. | Offline/fetch/tempdir/httpmock evidence. |
| 6 | INT-006 | INT-001 shared ViewModel pattern stable; local-state preservation known. | Fantasy read and mutation-deferral transcript. |
| 7 | INT-007 | Command-surface inventory and change-control decision complete. | Dependency inspection, lean CLI build, offline CLI smoke. |
| 8 | INT-008 | INT-001..INT-007 passed, deferred, blocked, or target-not-met with rationale. | L2 validation rehearsal and readiness review. |
| 9 | INT-009 | CHG-072 target specification baseline accepted; cache storage/schema and first consumer fixture selected. | Schema fixture, source-state/invalidation fixture, no-live read check, and consumer envelope demo. |

## Test Readiness

| Check | Status | Evidence |
|---|---|---|
| Required fixtures exist | passed_with_risk | Named by WP-001 through WP-007 before package closure or target-not-met disposition; WP-008 refreshed Lindsay L3 goldens when source/result context output changed. |
| Expected results are documented | passed_with_risk | Expected output/disclosure recorded in `VALIDATION.md`, `VERIFICATION.md`, package details, snapshots, and WP-008 pulse 01. |
| Verification commands are runnable | passed | Package-specific command blocks plus `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-targets` pass in the final rehearsal state. |
| Rollback path is known | passed_with_risk | Local-state layout, fantasy/local state, and fetch/cache risks are recorded in package closeouts; dependency surgery remains target-not-met with owner/revisit trigger; major analytics cache storage/rebuild rollback has selected failed-rebuild preservation evidence, while broader consumer-surface rollback remains pending beyond the first named-cache report and coach dashboard route. |

## Integration Risks

| Risk | Mitigation | Owner |
|---|---|---|
| Package-level tests pass but cross-surface semantics drift. | INT-001 and INT-008 require parity evidence before readiness claims. | Campbell / BENCH |
| Named layouts persist state that one surface cannot safely restore. | INT-002 requires schema versioning, migration/refusal behavior, and TUI/Web restore evidence. | Jack Adams / GLASS |
| Browser URL state or GET behavior regresses during integration. | INT-003 requires route/no-JS/read-only checks and change-control watch. | CREST / broadcast |
| Upstream/fetch errors appear only in CLI but not ViewModels/reports. | INT-005 requires source-state propagation evidence through consumers. | WIRE / TAPE |
| Dependency feature work breaks command availability silently. | INT-007 requires command-surface inventory, replacement/refusal/rollback evidence, and change control. | KEEL / FORGE |
| A cache amplifies stale, partial, unsupported, or mismatched hockey data across future screens. | INT-009 requires source-state, freshness, invalidation, quality/completeness, warning, disclosure, and consumer-contract evidence before any downstream surface claim. | HART / WIRE / TAPE / BENCH |
