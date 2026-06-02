# Work Packages

## Scope

Repo or feature: `icelines` repo-baseline VTRACE implementation.

Work packages convert the accepted VTRACE baseline into implementation slices.
Status is evidence posture, not a promise that the behavior is already shipped.

## Work Package Table

| ID | Objective | Parent IDs | Affected Surfaces | Entry Criteria | Exit Criteria | L0 / L1 / L2 | Status |
|---|---|---|---|---|---|---|---|
| WP-001 | Build source-state, query, and ViewModel parity foundation. | REQ-DATA-001; REQ-QUERY-001; REQ-PARITY-001; REQ-WB-002; REQ-CODE-001 | core ViewModels, query planner, CLI/TUI/Web/report adapters, docs | Baseline VTRACE files accepted; affected module inventory complete. | Shared source/context/query semantics proven by tests or inspection; trace/evidence rows updated. | L0: parser/ViewModel tests; L1: workspace checks; L2: parity fixture/demo | closed_with_risk; pulses 01-34 leaders JSON/Web HTML identity, JSON source-state, JSON active-context, JSON result-state, JSON empty/warning parity, selected Web HTML recovery parity, selected Web HTML active-context parity, selected Web HTML query-result parity, selected Web HTML source-state parity, selected Web HTML empty/warning metadata parity, selected Web HTML goalie-chip recovery affordance, selected Web HTML active-chip accessibility state, selected Web HTML active-chip route-level parity evidence, selected Web HTML active-filter route-level parity evidence, selected Web HTML active-filter UI route-level parity evidence, selected TUI context/source-state presentation, selected Markdown export context/source-state presentation, selected Markdown export front-matter context/source metadata, selected CLI text query context/source-state presentation, selected CLI CSV identity/context/source-state metadata, selected CLI CSV query-result metadata, selected CLI text query-result metadata, selected CLI text empty/warning recovery guidance, selected TUI query-result metadata, selected TUI active-filter result evidence, selected TUI active-filter L2 snapshot evidence, selected default CLI text active-filter result evidence, selected Markdown export report-body query-result metadata, selected Markdown export front-matter query-result metadata, selected Markdown export active-filter evidence, selected Markdown export empty/warning recovery guidance, selected Markdown export front-matter empty/warning recovery guidance, and selected TUI empty/warning recovery guidance passed_with_risk; remaining broad risks route to WP-003, WP-004, WP-005, and WP-008 |
| WP-002 | Implement named workbench layout persistence. | REQ-WB-003; REQ-WB-001; REQ-WB-002; REQ-WEB-001 | shared layout model, local state, TUI, Web URL/bookmark state, docs | `IF-LAYOUT-001` accepted; storage/migration rule selected; local-state backup/refusal path known. | Named layout save/restore/update works in TUI and Web where supported; schema/version evidence recorded. | L0: layout/state tests; L1: affected-slice/workspace posture; L2: VAL-010 demo | closed_with_risk |
| WP-003 | Harden Web dashboard route and browser safety behavior. | REQ-WEB-001; REQ-WEB-002; REQ-WB-002; REQ-PARITY-001 | web handlers/templates/CSS/assets, JSON twins, launch command | Shared ViewModel/context behavior is available or explicitly stubbed. | No-JS shell, active context, URL state, GET-read-only, recovery, viewport, and host/bind checks recorded. | L0: route tests; L1: workspace checks; L2: browser/no-JS inspection | closed_with_risk; pulses 01-07 season-type GET-read-only, favorites cache-read-only, streaks missing-cache, scoring missing-cache, Admin data-status missing-cache, browser shell/recovery, and serve launch/bind route boundaries passed_with_risk; residual live-browser/touch-focus/full-JSON-matrix risk routed to WP-008 |
| WP-004 | Add historical perspective and report/export evidence. | REQ-STAT-001; REQ-STAT-002; REQ-REPORT-001; REQ-PARITY-001 | report/export commands, scoring/perspective models, fixtures, snapshots | Fixture list covers lockout, rollover, ambiguous names, trades, active streaks, GP thresholds, and completeness. | Report/export snapshots disclose scope and avoid unsupported claims; edge observations are separate. | L0: fixture/snapshot tests; L1: workspace checks; L2: VAL-002/VAL-004 review | closed_with_risk; pulses 01-08 selected Markdown export disclosure guardrail, active-streak status label, completeness/skeleton disclosure, GP-threshold export evidence, duplicate/Unicode-name evidence, trade-continuity evidence, lockout/October rollover season-window evidence, and full-lockout season skip evidence passed_with_risk |
| WP-005 | Implement offline, fetch, and data-depth reliability evidence. | REQ-OFFLINE-001; REQ-DATA-DEPTH-001; REQ-FRESH-001; REQ-DATA-001 | fetch clients, snapshot/cache/manifest state, CLI data commands, core source-state path | Source-state foundation is available; tempdir/httpmock fixture approach selected. | Offline smoke, install/fetch/status/snapshot, failure/drift, and shift-refusal evidence recorded. | L0: tempdir/httpmock tests; L1: workspace checks; L2: command transcript | closed_with_risk; pulses 01-13 selected snapshot, offline, shift-refusal, retry/failure, transcript, integrity/schema, CSV/cache/upstream-schema, abbreviation-drift, missing-source, and partial-refresh resume/flag evidence passed_with_risk; broader transcript breadth routes to WP-008 |
| WP-006 | Close fantasy read-model and local-state mutation safety. | REQ-FANTASY-001; REQ-WEB-001; REQ-CODE-001 | fantasy ViewModels, local SQLite state, CLI/TUI/Web read flows, mutation deferrals | Local-state preservation rule accepted; read ViewModel surfaces inventoried. | Shared fantasy read ViewModels render consistently; web mutation deferrals and local-state preservation evidence recorded. | L0: ViewModel/local-state tests; L1: workspace checks; L2: VAL-007 demo | closed_with_risk; pulses 01-04 selected fantasy JSON local-state/cache-read, existing-FantasyDb read-only, poach imported-availability read-only, and VAL-007 transcript boundaries passed_with_risk |
| WP-007 | Remove dependency seams and add lean CLI build. | REQ-DEP-001; REQ-LEAN-001; REQ-CODE-001 | workspace manifests, feature gates, command surfaces, release docs | FLETCH/SLICE command-surface inventory complete; replacement/refusal/rollback plan accepted. | Dependency graph has no FLETCH/SLICE path/git seams; lean CLI build and offline smoke pass. | L0: manifest/dependency inspection; L1: workspace checks; L2: lean build smoke | target-not-met_dispositioned; pulse 01 inventory identifies current blockers and keeps standalone/lean claims unpromoted |
| WP-008 | Run integration and validation rehearsal. | REQ-CODE-001; VAL-001..VAL-010; TRACE.md | CLI, TUI, Web, JSON, reports, local state, build/deps, docs | Package-level evidence exists or is dispositioned. | Integration evidence, validation rehearsal, trace, and review gate are complete or accepted with risk. | L0: docs/trace checks; L1: workspace checks; L2: validation rehearsal | closed_with_risk; pulse 01 refreshed stale Lindsay L3 golden outputs, retired the broad clippy MDI test initializer lint, passed broad workspace format/clippy/test gates, and aligned VTRACE closeout rows while preserving WP-007 dependency/lean target-not-met posture |
| WP-009 | Build the major analytics cache foundation. | REQ-CACHE-001; REQ-CACHE-002; REQ-CACHE-003; REQ-CACHE-004; REQ-CODE-001 | analytics cache records/envelopes, cache store/read path, downstream consumer ViewModels, future dashboard/report surfaces | CHG-072 target-spec baseline accepted; storage, schema compatibility, metric family, and first consumer fixture selected. | Versioned schema, no-live read behavior, invalidation/degraded-state behavior, downstream consumer preservation, first named-cache Web report evidence, first coach dashboard route evidence, first opponent scout route evidence, and first player evidence-card route evidence recorded. | L0: schema/contract fixtures; L1: store/source-state fixtures; L2: consumer preservation and Web report fixtures | partial; pulses 02-08 passed selected core schema/envelope, strict JSON store/read/invalidation, internal consumer ViewModel, first product-facing named-cache Web report, first coach dashboard route, first opponent scout route, and first player evidence-card route evidence; broader hockey product surfaces remain pending |

## Work Package Details

### WP-001: Source-state, query, and ViewModel parity foundation

Objective: prove that shared query intent, source/completeness state, active
context, and ViewModel/envelope semantics survive to active renderers without
renderer-local hockey meaning.

Parent requirement IDs: REQ-DATA-001; REQ-QUERY-001; REQ-PARITY-001;
REQ-WB-002; REQ-CODE-001.

Design/interface/code-rigor IDs: DES-001; DES-002; DES-003; DES-004; DES-009;
DES-014; IF-DATA-001; IF-VIEW-001; IF-QUERY-001; CR-003; CR-006; CR-007;
CR-008; CR-010; CR-011; CR-023; CR-024; CR-032.

Validation scenario IDs: VAL-001; VAL-004; VAL-005; VAL-008; VAL-009.

Affected files/modules: `icelines-core/src/view_model/*`,
`icelines-query/src/*`, CLI/TUI/Web/report adapter paths touched by parity
evidence, and VTRACE evidence rows.

Entry criteria:

- Baseline requirements/interfaces/design rows are accepted.
- Affected adapters are inventoried before code changes.
- Parity fixture or demo target is named.

Exit criteria:

- Source-state and active context survive into the chosen ViewModel/envelope.
- CLI/TUI/Web/report behavior touched by the slice does not re-plan or recompute
  hockey semantics locally.
- `VERIFICATION.md`, `TRACE.md`, and `REVIEW.md` record evidence or pending
  disposition.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | Parser/ViewModel/parity affected tests or documented inspection. | pulses 01 through 34 passed: Web leaders ViewModel projection covers JSON rows, HTML `data-*` row identity, complete roster source state, active context, empty/warning state, HTML recovery rendering, HTML active-context attributes, HTML query-result attributes, HTML source-state attributes, HTML empty/warning attributes, visible goalie recovery position chip, active chip `aria-current` state, route-level active-chip parity with CLI JSON `position_filter`, route-level active-filter parity with CLI JSON `active_filters`, and route-level active-filter UI parity with CLI JSON `active_filters`; CLI leaders JSON export tests passed with result-state and envelope empty/warning fields; CLI query text tests assert context/source-state, query-result, active-filter result rows, and selected empty/warning recovery rendering; TUI leaders tests assert context/source-state rendering, selected query-result metadata rendering, selected active-filter result rendering, selected empty/warning recovery guidance, and the hidden snapshot parser/render seam; Markdown export tests assert context/source-state rendering, front-matter context/source metadata, report-body query-result metadata, front-matter query-result metadata, selected active-filter row/result evidence, selected empty/warning recovery guidance, and front-matter empty/warning state; CLI CSV tests assert identity/context/source-state and query-result metadata columns |
| L1 | yes | Workspace format, lint, and test matrix or justified affected-slice equivalent. | pulses 01-34 passed_with_risk: `cargo fmt --check`, affected Web clippy, affected CLI bin/test clippy passed; pulses 33 and 34 reran affected CLI system-test clippy. Full workspace clippy remains blocked by unrelated `icelines-fetch` lint debt and broader Web all-targets clippy remains blocked by unrelated existing `icelines-web\tests\l1_router.rs` lint debt |
| L2 | yes | Cross-surface parity fixture or demo tied to `VAL-004`. | pulses 01-08, 10-15, 17-24, 28-30, 32, 33, and 34 passed: CLI subprocess matched Web route JSON and Web HTML leaders stable identity fixtures, plus CLI/Web JSON source-state, active-context, result-state, empty/warning parity, CLI JSON envelope/Web HTML recovery parity, CLI JSON/Web HTML active-context parity, CLI JSON/Web HTML query-result parity, CLI JSON/Web HTML source-state parity, CLI JSON envelope/Web HTML empty/warning metadata parity, CLI JSON/Web HTML active-chip parity, CLI JSON/Web HTML active-filter parity, CLI JSON/Web HTML active-filter UI parity, Markdown export context/source-state stdout evidence, Markdown export front-matter metadata stdout evidence, CLI query text context/source-state stdout evidence, CLI CSV identity/context/source-state stdout evidence, CLI CSV query-result stdout evidence, CLI query text result metadata stdout evidence, CLI query text active-filter result stdout evidence, CLI query text empty/warning recovery stdout evidence, Markdown export query-result stdout evidence, Markdown export front-matter query-result stdout evidence, Markdown export active-filter stdout evidence, Markdown export empty/warning stdout evidence, Markdown export front-matter empty/warning stdout evidence, and hidden TUI snapshot active-filter stdout evidence; pulses 09, 16, 25, and 31 add L0 TUI evidence only and keep full interactive TUI parity pending overall |

V closure:

| V Area | IDs / Evidence | Status | Notes |
|---|---|---|---|
| Need / CONOPS | CON-001; CON-004; CON-005; CON-008 | pending | Supports same-answer and source-honesty needs. |
| Requirements | REQ-DATA-001; REQ-QUERY-001; REQ-PARITY-001; REQ-WB-002 | pending | Parent rows remain pending until evidence passes. |
| Architecture / Interface | IF-DATA-001; IF-VIEW-001; IF-QUERY-001 | pending | Interface semantics must remain additive/compatible. |
| Design / Code Rigor | DES-001..004; DES-009; DES-014; CR-003; CR-006..011; CR-023; CR-024; CR-032 | pending | No renderer-local semantic drift. |
| Implementation | Paths listed above | closed_with_risk | Selected leaders parity foundation is accepted with residual risks routed to successor packages. |
| Verification | EVID-WP001-L0; EVID-WP001-L1; EVID-WP001-L2; EVID-WP001-HTML-L2; EVID-WP001-SOURCE-L2; EVID-WP001-CONTEXT-L2; EVID-WP001-RESULT-L2; EVID-WP001-EMPTY-WARNING-L2; EVID-WP001-HTML-RECOVERY-L2; EVID-WP001-HTML-CONTEXT-L2; EVID-WP001-HTML-RESULT-L2; EVID-WP001-HTML-SOURCE-L2; EVID-WP001-HTML-EMPTY-WARNING-L2; EVID-WP001-HTML-POS-CHIP-L0; EVID-WP001-HTML-POS-ARIA-L0; EVID-WP001-HTML-POS-ARIA-L2; EVID-WP001-HTML-FILTER-ACTIVE-L2; EVID-WP001-HTML-FILTER-UI-L2; EVID-WP001-TUI-CONTEXT-L0; EVID-WP001-TUI-RESULT-L0; EVID-WP001-TUI-ACTIVE-FILTER-L0; EVID-WP001-TUI-ACTIVE-FILTER-L2; EVID-WP001-TUI-EMPTY-WARNING-L0; EVID-WP001-EXPORT-CONTEXT-L0; EVID-WP001-EXPORT-CONTEXT-L2; EVID-WP001-EXPORT-METADATA-L0; EVID-WP001-EXPORT-METADATA-L2; EVID-WP001-EXPORT-RESULT-L0; EVID-WP001-EXPORT-RESULT-L2; EVID-WP001-EXPORT-FM-RESULT-L0; EVID-WP001-EXPORT-FM-RESULT-L2; EVID-WP001-EXPORT-EMPTY-WARNING-L0; EVID-WP001-EXPORT-EMPTY-WARNING-L2; EVID-WP001-EXPORT-FM-EMPTY-WARNING-L0; EVID-WP001-QUERY-TEXT-CONTEXT-L0; EVID-WP001-QUERY-TEXT-CONTEXT-L2; EVID-WP001-QUERY-CSV-METADATA-L0; EVID-WP001-QUERY-CSV-METADATA-L2; EVID-WP001-QUERY-CSV-RESULT-L0; EVID-WP001-QUERY-CSV-RESULT-L2; EVID-WP001-QUERY-TEXT-RESULT-L0; EVID-WP001-QUERY-TEXT-RESULT-L2; EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L0; EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L2; EVID-CR-003; EVID-CR-011; EVID-CR-018 | partial | Leaders identity, JSON source-state, JSON active-context, JSON result-state, JSON empty/warning parity, selected Web HTML recovery parity, selected Web HTML active-context parity, selected Web HTML query-result parity, selected Web HTML source-state parity, selected Web HTML empty/warning metadata parity, selected Web HTML goalie-chip recovery affordance, selected Web HTML active-chip accessibility state, selected Web HTML active-chip route-level parity evidence, selected Web HTML active-filter route-level parity evidence, selected Web HTML active-filter UI route-level parity evidence, selected TUI context/source-state, selected TUI query-result metadata, selected TUI active-filter result evidence, selected TUI active-filter L2 snapshot evidence, selected TUI empty/warning recovery guidance, selected Markdown export context/source-state, selected Markdown export front-matter metadata, selected CLI text query context/source-state, selected CLI CSV identity/context/source-state, selected CLI CSV query-result metadata, selected CLI text query-result metadata, selected CLI text empty/warning recovery guidance, selected Markdown export query-result metadata, selected Markdown export empty/warning recovery guidance, and selected Markdown export front-matter empty/warning recovery guidance have evidence; broader evidence rows remain pending overall. |
| Validation | VAL-004 primarily; VAL-001/005/008 impacts; selected VAL-002C report/export disclosure | partial | CLI/Web JSON stable row identity, Web HTML row identity, CLI/Web JSON source-state, CLI/Web JSON active context, CLI/Web JSON result state, CLI/Web JSON empty/warning state, selected CLI envelope/Web HTML recovery, selected CLI JSON/Web HTML active context, selected CLI JSON/Web HTML query-result state, selected CLI JSON/Web HTML source state, selected CLI JSON envelope/Web HTML empty/warning metadata, selected CLI JSON/Web HTML active-filter route state, selected CLI JSON/Web HTML active-filter UI state, selected TUI context/source state, selected TUI query-result state, selected TUI active-filter result state, selected TUI active-filter L2 snapshot state, selected TUI empty/warning recovery state, selected Markdown export context/source state, selected Markdown export front-matter metadata, selected CLI text query context/source state, selected CLI CSV identity/context/source state, selected CLI CSV query-result state, selected CLI text query-result state, selected CLI text empty/warning recovery state, selected Markdown export query-result state, selected Markdown export empty/warning recovery state, and selected Markdown export front-matter empty/warning state matched for leaders; full interactive TUI, full report/export, broader browser route/accessibility proof, broader provenance, and broader active context remain open. |
| Trace | `TRACE.md` requirement and work-package trace | closed_with_risk | Evidence IDs and residual-risk routing are recorded before closure. |
| Gate | Work Package Close Review | closed_with_risk | Decision recorded in `REVIEW.md`. |

Validation impact: enables later cross-surface validation claims.

Risks: a narrow adapter fix could hide drift in untouched surfaces; record
affected-slice rationale if not running full parity evidence.

Assurance/security classification:

| Lane | Required | Reviewer / Role | Decision | Evidence / Rationale |
|---|---|---|---|---|
| Systems engineering | yes | Campbell / HART | pending | Shared semantic boundary. |
| Requirements traceability | yes | BENCH | pending | Parent requirements map to evidence. |
| V&V | yes | BENCH / EDGE | pending | Parity and source-state checks. |
| Software assurance | yes | FORGE | pending | Rust boundaries and tests. |
| Security/privacy | no | CREST | not_required | No new exposure expected unless Web paths change. |
| Safety/mission impact | yes | TAPE / PACE | pending | Wrong-but-confident output risk. |
| Source custody | no | Foster | not_required | No upstream data writes expected. |
| Configuration/change control | yes | Jim Gregory | pending | Interface/evidence changes may trigger change control. |

Review gate: Work Package Close Review passed with risk on 2026-05-31.

Git execution:

- Branch/worktree: child-repo branch in `C:\src\ICELINES`.
- Commit plan: implementation, tests/evidence, and VTRACE row updates together.
- Push/PR condition: L0/L1 pass or affected-slice rationale is recorded.
- Agent stop condition: stop if interface semantics or validation claims change
  without `CHANGE_CONTROL.md`.

Status: in_progress. Pulses 01-32 leaders JSON/Web HTML identity, JSON
source-state, JSON active-context, JSON result-state, JSON empty/warning parity,
selected Web HTML recovery parity, selected Web HTML active-context parity,
selected Web HTML query-result parity, selected Web HTML source-state parity,
selected Web HTML empty/warning metadata parity, selected TUI context/source-state presentation, selected Markdown export
context/source-state presentation, selected Markdown export front-matter
context/source metadata, selected CLI text query context/source-state
presentation, selected CLI CSV identity/context/source-state metadata, selected
CLI CSV query-result metadata, selected CLI text query-result metadata, selected
CLI text empty/warning recovery guidance, selected TUI query-result metadata, selected Markdown export report-body query-result
metadata, selected Markdown export front-matter query-result metadata, selected
Markdown export empty/warning recovery guidance, selected Markdown export
front-matter empty/warning recovery guidance, selected TUI empty/warning
recovery guidance, selected Web HTML goalie-chip recovery affordance, selected
Web HTML active-chip accessibility state, selected Web HTML active-chip
route-level parity evidence, and selected Web HTML active-filter route-level
parity evidence, selected Web HTML active-filter UI route-level parity evidence,
selected TUI active-filter result evidence, and selected default CLI text
active-filter result evidence are `passed_with_risk`; the
broad work package remains open.

### WP-002: Named workbench layout persistence

Objective: implement durable named layout save/restore/update behavior across
TUI and Web where supported, using a shared versioned layout model instead of
renderer-local semantic state.

Parent requirement IDs: REQ-WB-003; REQ-WB-001; REQ-WB-002; REQ-WEB-001.

Design/interface/code-rigor IDs: DES-006; DES-007; DES-015; IF-LAYOUT-001;
IF-WEB-001; IF-VIEW-001; CR-012; CR-013; CR-014; CR-020; CR-032.

Validation scenario IDs: VAL-001; VAL-003; VAL-010.

Affected files/modules: `icelines-core/src/workbench_layout.rs`,
`icelines-core/src/lib.rs`, `icelines-cli/src/commands/layout.rs`,
`icelines-cli/src/commands/mod.rs`, `icelines-cli/src/config.rs`,
`icelines-cli/src/cli.rs`, `icelines-cli/src/main.rs`,
`icelines-cli/src/tui/mod.rs`, `icelines-cli/src/tui/mdi.rs`,
`icelines-cli/src/commands/menu.rs`, `icelines-cli/src/commands/serve.rs`,
`icelines-web/src/config.rs`, `icelines-web/src/handlers/dashboard.rs`,
`docs/vtrace/*.md`, and
`context/waves/2026-05-30-vtrace-wp002-layout/**`.

Entry criteria:

- Storage location and schema version are selected.
- Migration/refusal behavior for corrupt, unsupported, or incomplete records is
  documented before writes are enabled.
- Local-state preservation and rollback path are known.

Exit criteria:

- Users can name, save, restore, and update a layout in the supported surfaces.
- Stored record includes version, layout name, workspace/pane semantics, active
  context policy, and surface display hints only where appropriate.
- TUI/Web restore evidence and stored-schema inspection close `VAL-010` or record
  an accepted deferral.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | `cargo test -p icelines-core workbench_layout --lib`; `cargo test -p icelines-web dashboard --lib`; CLI bin tests `l0_layout_cli_save_and_show_round_trip` and `l0_mdi_applies_persisted_workbench_layout`. | passed for focused schema/store/CLI/TUI/Web restore checks |
| L1 | yes | Workspace format, lint, and tests or affected-slice equivalent. | `cargo fmt --check` and core clippy passed; full workspace/web/CLI clippy remains blocked by unrelated existing lint debt outside WP-002 |
| L2 | yes | `VAL-010` TUI/Web restore demo plus stored-layout inspection. | passed_with_risk: CLI separate-process reload/store inspection and Web `/dashboard?layout=tonight` restore passed; TUI restore covered by focused bin test |

V closure:

| V Area | IDs / Evidence | Status | Notes |
|---|---|---|---|
| Need / CONOPS | Mission personalized layouts; CON-001; CON-003 | passed_with_risk | Direct mission target for named layouts is implemented; broader workbench validation remains outside WP-002. |
| Requirements | REQ-WB-003; REQ-WB-001; REQ-WB-002; REQ-WEB-001 | passed_with_risk | `REQ-WB-003` has focused L0 and L2 evidence; broader workspace lint debt remains open. |
| Architecture / Interface | IF-LAYOUT-001; IF-WEB-001; IF-VIEW-001; CHG-001 | passed_with_risk | Schema/store/URL decision is controlled and demonstrated for layout restore; `IF-WEB-001` remains broader than WP-002. |
| Design / Code Rigor | DES-006; DES-007; DES-015; CR-012; CR-013; CR-014; CR-020; CR-032 | pass_with_risk | Focused local-state/schema/TUI/Web checks pass; broader code-rigor evidence pending. |
| Implementation | `icelines-core/src/workbench_layout.rs`; CLI layout command/config/TUI restore; Web dashboard layout restore | in_progress | Shared schema, CLI commands, TUI restore, and Web restore are implemented. |
| Verification | EVID-WP002-L0; EVID-WP002-L1; EVID-WP002-L2; EVID-VAL-010; EVID-CR-008 | pass_with_risk | L0 and L2 evidence passed; L1 is partial because broad clippy exposes unrelated existing lint debt. |
| Validation | VAL-010 | passed_with_risk | CLI durable reload/store inspection and Web restore demo passed; TUI restore is covered by bin test evidence. |
| Trace | `TRACE.md` WP-002 row | pass_with_risk | Trace now names L0/L1/L2 evidence and the workspace-clippy risk. |
| Gate | Work Package Close Review and Integration Readiness Review | closed_with_risk | WP-002 close review accepted named-layout risk; WP-008 still owns broader integration rehearsal. |

Validation impact: closes or refines the mission personalization target.

Risks: corrupt local records, destructive migration, hidden renderer-local
semantic state, or inconsistent TUI/Web restore behavior.

Assurance/security classification:

| Lane | Required | Reviewer / Role | Decision | Evidence / Rationale |
|---|---|---|---|---|
| Systems engineering | yes | KEEL / GLASS | pass_with_risk | Cross-surface schema and restore path are coherent; L2 demo passed with TUI covered by bin evidence. |
| Requirements traceability | yes | BENCH | pass_with_risk | `REQ-WB-003`, `IF-LAYOUT-001`, `VAL-010`, `EVID-VAL-010`, and `EVID-CR-008` remain linked with L1 risk named. |
| V&V | yes | BENCH / EDGE | pass_with_risk | Focused L0 and L2 passed; L1 broad clippy remains open due unrelated existing lint debt. |
| Software assurance | yes | FORGE | pass_with_risk | Typed schema/refusal paths and focused tests pass; broad clippy/workspace evidence pending. |
| Security/privacy | yes | CREST / broadcast | pass_with_risk | Local file path and URL restore are explicit and non-secret; Web restore demo passed. |
| Safety/mission impact | yes | GLASS | pass_with_risk | Personalized workflow mission target has L2 restore evidence with workspace-lint risk still named. |
| Source custody | no | Foster | not_required | No external data source expected. |
| Configuration/change control | yes | Jim Gregory | pass_with_risk | `CHG-001`, stage execution, and pulse evidence name the schema/store/URL decision and remaining lint risk. |

Review gate: Implementation Readiness Review before coding, Integration
Readiness Review before TUI/Web integration, then Work Package Close Review.

Git execution:

- Branch/worktree: child-repo branch dedicated to layout persistence.
- Commit plan: schema/model first, surface adapters second, evidence/docs last.
- Push/PR condition: L0/L1 pass and no local-state migration risk is unreviewed.
- Agent stop condition: stop if schema writes could destroy existing
  `~/.icelines/` data without a recorded backup/refusal plan.

Stage / pulse record:

- Stage: S4 Work Package Execution, with INT-002 in S5 partial integration.
- Change control: `CHG-001`.
- Pulse:
  `context/waves/2026-05-30-vtrace-wp002-layout/pulses/pulse-01.md`.

Status: in_progress.

### WP-003: Web dashboard route and browser safety

Objective: prove browser cold-start, no-JS readability, active context, URL
state, recovery paths, host/bind safety, and GET-read-only behavior.

Parent requirement IDs: REQ-WEB-001; REQ-WEB-002; REQ-WB-002; REQ-PARITY-001.

Design/interface/code-rigor IDs: DES-007; DES-014; IF-WEB-001; IF-DATA-001;
IF-VIEW-001; CR-014; CR-015; CR-027; CR-032.

Validation scenario IDs: VAL-003; VAL-004.

Affected files/modules: `icelines-web/src/*`, CLI serve/launch command paths,
templates/assets, JSON twins, and route tests.

Entry criteria:

- Active context and source-state contract from WP-001 is available or explicitly
  bounded for this slice.
- Route list and expected recovery states are named.

Exit criteria:

- Browser route/no-JS/viewport/recovery evidence is recorded.
- GET routes remain read-only and mutation behavior is POST-backed or deferred.
- `0.0.0.0` bind warning and URL-before-auto-open behavior are verified where
  touched.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | Route tests and template/JSON snapshot checks for touched paths. | pulses 01-07 passed: season-type POST route tests, GET read-only/method-not-allowed evidence, favorites cache-read-only/missing-cache evidence, streaks missing-cache no-cache-creation evidence, scoring/outlook/tonight-intel missing-cache no-cache-creation evidence, Admin data-status missing-cache no-cache-creation evidence, selected no-JS/viewport/recovery shell evidence, and serve launch/bind evidence |
| L1 | yes | Workspace checks or affected-slice equivalent. | pulses 01-07 passed: `cargo fmt --check`, focused route/serve tests, and affected Web/CLI clippy |
| L2 | yes | Browser/no-JS/narrow viewport inspection for `VAL-003`. | partial accepted: route-level HTML evidence proves no-JS notice, viewport metadata, skip-link, dashboard URL-addressable copy, and 404 recovery; serve launch helper evidence proves URL/no-open/LAN warning semantics; live browser screenshot/review and touch/focus inspection are residual WP-008 risks |

V closure: see `TRACE.md` WP-003 and `VERIFICATION.md` EVID-CR-006,
EVID-CR-014, and EVID-CR-018. Status is `closed_with_risk` with pulses 01-07
accepted for selected route and launch boundaries.

Validation impact: closes browser first-session and safety portions of `VAL-003`.

Risks: visual-only fixes could miss JSON or no-JS state; route evidence must
include semantic context, not only screenshots.

Assurance/security classification: systems engineering, requirements
traceability, V&V, software assurance, security/privacy, safety/mission impact,
and configuration/change control are required; source custody is not required
unless external data paths change.

Review gate: Work Package Close Review.

Git execution:

- Branch/worktree: child-repo branch for web route/browser slice.
- Commit plan: route/template changes with tests and evidence docs.
- Push/PR condition: L0/L1 pass and browser inspection evidence is recorded or
  explicitly pending.
- Agent stop condition: stop if a GET route mutates local/user state.

Stage / pulse record:

- Stage: S4 Work Package Execution, with INT-003 in S5 partial integration.
- Change control: `CHG-037`; `CHG-038`; `CHG-039`; `CHG-041`; `CHG-042`;
  `CHG-047`; `CHG-048`.
- Pulse:
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-01.md`;
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-02.md`;
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-03.md`;
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-04.md`;
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-05.md`;
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-06.md`;
  `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-07.md`.

Status: `closed_with_risk`. Pulses 01-07 are `passed_with_risk` for selected route
boundaries only: POST owns season-type mutation, GET returns
method-not-allowed, route tests prove active state is preserved, and
`/favorites` stat-line rendering reads only existing cached boxscores without
creating manifest/boxscore state; selected streaks and scoring/outlook/
tonight-intel GET render paths also avoid creating local cache state when
manifest data is absent; selected Admin data-status GET render paths avoid the
same cache-state creation; selected dashboard and unknown-route HTML proves
no-JS notice, viewport metadata, skip-link, navigation, and recovery affordances.
Serve launch tests prove URL-before-open output, `--no-open` gating, LAN warning,
and bind resolution behavior. Live browser screenshot/review, touch/focus, and
full JSON-twin matrix inspection remain residual WP-008 risks before readiness.

### WP-004: Historical perspective and report/export evidence

Objective: make public historical perspective and exported artifacts
fixture-backed, disclosure-forward, and free of unsupported era-adjusted,
predictive, betting, deployment, injury, special-teams, or linemate claims.

Parent requirement IDs: REQ-STAT-001; REQ-STAT-002; REQ-REPORT-001;
REQ-PARITY-001.

Design/interface/code-rigor IDs: DES-008; DES-013; IF-REPORT-001; IF-VIEW-001;
IF-DATA-001; CR-016; CR-017; CR-025; CR-026; CR-028; CR-032.

Validation scenario IDs: VAL-002; VAL-004.

Affected files/modules: report/export commands, report ViewModel builders,
scoring/perspective helpers, fixtures, snapshots, and public-copy docs/examples.

Entry criteria:

- Fixture plan names lockout, October rollover, ambiguous/Unicode names,
  duplicate names, intra-season trades, active streaks, GP thresholds, and
  skeleton/completeness disclosure.

Exit criteria:

- Snapshot/text evidence proves disclosure near the top and no unsupported
  public-copy implication.
- Edge observations are separate and not collapsed into one broad smoke.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | Report/export fixture and snapshot tests for touched paths. | pulses 01-08 passed_with_risk: selected Markdown leaders export disclosure, active-streak status, completeness/skeleton disclosure, GP-threshold, duplicate/Unicode-name, trade-continuity, lockout/October rollover season-window, and full-lockout skip evidence are recorded. |
| L1 | yes | Workspace checks or affected-slice equivalent. | pulses 01-08 affected-slice passed: focused export/streak/data tests, `cargo fmt --check`, affected CLI/core clippy, VTRACE proof, and diff checks were recorded as applicable; broad workspace clippy remains blocked by unrelated known lint debt and is not required for selected package closeout. |
| L2 | yes | `VAL-002` and `VAL-004` report/export review evidence. | passed_with_risk for `VAL-002`: pulses 01-08 selected Markdown export public-copy, active-streak status, completeness/skeleton, GP-threshold, duplicate/Unicode-name, trade-continuity, season-window, and full-lockout skip reviews passed_with_risk; full report/export matrix remains pending overall under `VAL-004` and WP-008. |

V closure: see `TRACE.md` WP-004 and `VERIFICATION.md` EVID-VAL-002A/B/C/D,
EVID-WP004-LOCKOUT-SKIP-L0, EVID-CR-007, EVID-CR-012, EVID-CR-013, and
EVID-CR-016. Status is closed_with_risk with broader report/export matrix,
broader active-streak parity, and ambiguous-name disambiguation breadth routed to
WP-008.

Validation impact: pulses 01-08 close_with_risk the public-copy disclosure,
active-streak labeling, completeness/skeleton disclosure, GP-threshold,
duplicate/Unicode-name, trade-continuity, and lockout/October rollover
season-window portions of `VAL-002`, and add full-lockout season skip evidence;
full report/export matrix coverage remains open under `VAL-004`/WP-008.

Risks: social/report copy can overclaim even when raw values are correct; text
review is a required evidence type.

Assurance/security classification: systems engineering, requirements
traceability, V&V, software assurance, safety/mission impact, and
configuration/change control are required; security/privacy and source custody
are not required unless new data sources or user data are introduced.

Review gate: Work Package Close Review.

Git execution:

- Branch/worktree: child-repo branch for report/export evidence.
- Commit plan: fixtures/snapshots before public-copy claim changes.
- Push/PR condition: affected snapshot/text review evidence is recorded.
- Agent stop condition: stop if copy implies unsupported methodology.

Status: closed_with_risk; pulses 01-08 passed_with_risk.

### WP-005: Offline, fetch, and data-depth reliability

Objective: prove offline query behavior, explicit data install/fetch/status
flows, snapshot integrity, upstream failure handling, resumability/partial-write
policy, and locked shift-level refusal.

Parent requirement IDs: REQ-OFFLINE-001; REQ-DATA-DEPTH-001; REQ-FRESH-001;
REQ-DATA-001.

Design/interface/code-rigor IDs: DES-004; DES-005; DES-012; IF-FETCH-001;
IF-DATA-001; CR-008; CR-009; CR-018; CR-019; CR-029; CR-031.

Validation scenario IDs: VAL-005; VAL-006; VAL-008.

Affected files/modules: `icelines-fetch/src/*`, snapshot/manifest/cache code,
CLI data/fetch commands, source-state builders, tempdir/httpmock fixtures.

Entry criteria:

- Fixture/mocking approach covers absence, rate limit, unavailable service,
  schema drift, integrity mismatch, newer schema, abbreviation drift, and partial
  writes.

Exit criteria:

- Query-time paths do not call live APIs to hide missing data.
- Fetch/write paths fail loudly or produce typed missing/unavailable state.
- Command transcripts or tests close `VAL-005`, `VAL-006`, and `VAL-008` scope
  for touched surfaces.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | Tempdir/httpmock/source-state fixtures for touched paths. | pulses 01-02 and 04-13 passed: selected `SnapshotStore::read` evidence refuses named unsealed snapshots with `NotSealed` before trusting existing file bytes; selected no-live schedule and bundled leaders isolated-home smoke evidence avoids data/cache writes; selected httpmock retry evidence covers 429, 503, generic 5xx, non-retryable 4xx, retry budget, and backoff cap; selected no-live fetch refusals avoid data/cache writes; selected snapshot integrity and missing-file fixtures reject changed or incomplete source state; selected chunked snapshot schema fixtures promote v1, round-trip v2, and reject v3/newer manifests; selected MoneyPuck CSV fixtures reject missing required columns and malformed rows; selected FLETCH cache/refresh, player landing schema-drift, abbreviation mapper, missing-source, and partial-refresh resume/flag fixtures pass |
| L1 | yes | Workspace checks or affected-slice equivalent. | pulses 02-03, 05, and 10-13 affected CLI/fetch evidence passed; pulse 01 affected fetch slice passed_with_risk with the existing `icelines-fetch/src/fletch.rs` clippy blocker recorded |
| L2 | yes | Data/fetch command transcript and offline smoke evidence. | partial: pulse 02 selected offline smoke passed, pulse 03 selected shift capability refusal passed, and pulse 05 selected data/fetch/status/snapshot command transcript evidence passed; broader transcript breadth remains pending |

V closure: see `TRACE.md` WP-005 and `VERIFICATION.md` EVID-VAL-005,
EVID-VAL-006, EVID-VAL-008, EVID-WP005-SNAPSHOT-SEAL-L0,
EVID-WP005-OFFLINE-SMOKE-L2, EVID-WP005-SHIFTS-LOCK-L1,
EVID-WP005-FETCH-RETRY-L1, EVID-WP005-DATA-TRANSCRIPT-L2,
EVID-WP005-SNAPSHOT-INTEGRITY-L0, EVID-WP005-CHUNKED-SCHEMA-L0,
EVID-WP005-MONEYPUCK-CSV-L0, EVID-WP005-CACHE-REFRESH-L0,
EVID-WP005-UPSTREAM-SCHEMA-L1, EVID-WP005-ABBREV-DRIFT-L1,
EVID-WP005-MISSING-SOURCE-L1, EVID-WP005-PARTIAL-RESUME-L0, EVID-CR-004, EVID-CR-015, and
EVID-CR-017.
Status is closed_with_risk.

Validation impact: closes offline and upstream reliability claims.

Risks: a successful happy-path fetch can mask drift or partial writes; closure
requires failure fixtures.

Assurance/security classification: systems engineering, requirements
traceability, V&V, software assurance, security/privacy, safety/mission impact,
source custody, and configuration/change control are required.

Review gate: Work Package Close Review.

Git execution:

- Branch/worktree: child-repo branch for fetch/offline reliability.
- Commit plan: failure fixtures with implementation changes.
- Push/PR condition: failure modes are evidenced or explicitly pending.
- Agent stop condition: stop if user-facing reads deserialize unchecked bytes or
  call live APIs opportunistically.

Status: in_progress; pulse 01 selected unsealed snapshot read-refusal evidence
passed, pulse 02 selected offline/query smoke passed_with_risk, pulse 03
selected shift capability lock/refusal evidence passed_with_risk, pulse 04
selected upstream retry/failure evidence passed_with_risk, pulse 05 selected
data/fetch command transcript evidence passed_with_risk, and pulse 06 selected
snapshot integrity mismatch and missing-file evidence passed_with_risk, pulse 07
selected chunked snapshot schema drift/newer-schema evidence passed_with_risk,
pulse 08 selected MoneyPuck CSV drift evidence passed_with_risk, pulse 09
selected FLETCH cache/refresh fallback evidence passed_with_risk, and pulse 10
selected player landing schema-drift evidence passed_with_risk, pulse 11
selected abbreviation-drift evidence passed_with_risk, pulse 12 selected
missing-source evidence passed_with_risk, and pulse 13 selected partial-refresh
resume/flag evidence passed_with_risk. Broader command transcript breadth remains
accepted WP-008 residual risk.

### WP-006: Fantasy read-model and local-state mutation safety

Objective: render fantasy poach, roster gaps, import, and simulation read flows
from shared ViewModels where available, while preserving local-state safety and
deferring unsupported Web mutations away from GET.

Parent requirement IDs: REQ-FANTASY-001; REQ-WEB-001; REQ-CODE-001.

Design/interface/code-rigor IDs: DES-007; IF-VIEW-001; IF-WEB-001; CR-014;
CR-020; CR-023.

Validation scenario IDs: VAL-007.

Affected files/modules: fantasy ViewModel builders, local SQLite/FantasyDb paths,
CLI/TUI/Web fantasy adapters, route mutation boundaries, local-state tests.

Entry criteria:

- Read surfaces and mutation surfaces are inventoried.
- Existing local state preservation behavior is understood.

Exit criteria:

- Shared fantasy read models render consistently where supported.
- Web mutation deferrals route to CLI/TUI or POST-backed paths; GET does not
  mutate local state.
- Local-state preservation evidence is recorded.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | Fantasy ViewModel, local-state, and route mutation tests. | pulses 01-04 passed: selected fantasy JSON route tests assert missing local FantasyDb state does not create `~/.icelines`, daily/matchup missing-cache reads do not create `~/.icelines/data`, existing-FantasyDb GET reads do not create SQLite WAL/SHM sidecar state, poach imported-availability GET reads do not create SQLite WAL/SHM sidecar state, and focused core/fetch/CLI/Web fantasy transcript tests pass |
| L1 | yes | Workspace checks or affected-slice equivalent. | pulses 01-04 passed affected-slice: `cargo fmt --check`; `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`; focused core/fetch/CLI/Web fantasy transcript tests |
| L2 | yes | `VAL-007` read/mutation-deferral transcript. | passed_with_risk: shared ViewModels, CLI/TUI handoffs, CLI L2 import/gaps/roster-shape/daily/matchup/export commands, Web dashboard mutation deferrals, Web fantasy routes, and Web poach routes covered; active-writer SQLite and full interactive TUI rendering remain accepted risks |

V closure: see `TRACE.md` WP-006 and `VERIFICATION.md` EVID-VAL-007,
EVID-WP006-FANTASY-L1, EVID-WP006-FANTASY-RO-L1,
EVID-WP006-POACH-RO-L1, EVID-WP006-TRANSCRIPT-L0, and EVID-CR-008.
Status is closed_with_risk.

Validation impact: closes fantasy decision-loop read behavior and safe mutation
boundaries.

Risks: local user data loss or accidental Web GET mutation.

Assurance/security classification: systems engineering, requirements
traceability, V&V, software assurance, security/privacy, safety/mission impact,
source custody, and configuration/change control are required because local user
state is in scope.

Review gate: Work Package Close Review.

Git execution:

- Branch/worktree: child-repo branch for fantasy read/local-state slice.
- Commit plan: local-state tests before mutation-boundary changes.
- Push/PR condition: preservation and route safety evidence is recorded.
- Agent stop condition: stop if a path can mutate FantasyDb/favorites/watch state
  through GET or without a preservation plan.

Status: closed_with_risk. Pulses 01-04 selected fantasy JSON
local-state/cache-read, existing-FantasyDb read-only, poach imported-availability
read-only, and `VAL-007` final transcript boundaries are `passed_with_risk`;
active-writer database semantics, full interactive TUI rendering, and broader
local-state preservation evidence remain accepted risks.

### WP-007: Dependency seams and lean CLI target

Objective: remove or replace FLETCH/SLICE dependency seams and implement a lean
offline CLI feature path without silently deleting command behavior.

Parent requirement IDs: REQ-DEP-001; REQ-LEAN-001; REQ-CODE-001.

Design/interface/code-rigor IDs: DES-010; DES-011; IF-BUILD-001; CR-021;
CR-022; CR-029.

Validation scenario IDs: VAL-009.

Affected files/modules: workspace and member `Cargo.toml` files, feature-gated
module boundaries, command surfaces that depend on FLETCH/SLICE, release/build
docs.

Entry criteria:

- FLETCH/SLICE command-surface inventory is complete.
- Replacement, refusal, compatibility shim, or rollback path is accepted for each
  affected surface.
- Feature boundary plan names `cli`, `tui`, `web`, `net`, and `reports` behavior.

Exit criteria:

- Dependency inspection finds no FLETCH/SLICE path/git seams before standalone
  claim.
- `cargo build --no-default-features --features cli` and offline CLI smoke pass
  before lean claim.
- Removed or altered commands have documented replacement/refusal/rollback
  evidence.

Verification commands:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --no-default-features --features cli
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | Manifest/dependency graph inspection and feature compile checks. | target-not-met_dispositioned; pulse 01 records FLETCH path dependency, direct/transitive SLICE git dependencies, affected FLETCH commands, affected SLICE selector, and missing `cli` feature |
| L1 | yes | Workspace checks. | pending |
| L2 | yes | Lean CLI build plus offline smoke and command-surface evidence. | target-not-met |

V closure: see `TRACE.md` WP-007 and `VERIFICATION.md` EVID-DEP-001,
EVID-LEAN-001, EVID-WP007-DEP-INVENTORY-L0, and EVID-CR-009. Status remains
target-not-met_dispositioned with maintainer/release owner and a revisit trigger
when dependency removal and Cargo feature-gating are ready.

Validation impact: closes standalone/lean target claims only after evidence
passes.

Risks: dependency surgery can break hidden command paths or overclaim release
posture.

Assurance/security classification: systems engineering, requirements
traceability, V&V, software assurance, source custody, and configuration/change
control are required; security/privacy is required if dependency changes affect
network or local state boundaries.

Review gate: Implementation Readiness Review before code, Work Package Close
Review before status changes.

Git execution:

- Branch/worktree: child-repo branch dedicated to dependency/feature work.
- Commit plan: inventory and compatibility notes before Cargo surgery.
- Push/PR condition: dependency and lean evidence are recorded or target-not-met.
- Agent stop condition: stop if command behavior disappears without replacement,
  refusal, shim, or rollback note.

Status: target-not-met.

### WP-008: Integration and validation rehearsal

Objective: integrate package outputs into an end-to-end evidence rehearsal across
CLI, TUI, Web, JSON, reports, local state, fetch/offline behavior, and build/deps.

Parent requirement IDs: REQ-CODE-001 plus all requirements with non-deferred
validation impact.

Design/interface/code-rigor IDs: all IDs touched by completed work packages.

Validation scenario IDs: VAL-001 through VAL-010.

Affected files/modules: integration fixtures, validation scripts/transcripts,
route/browser evidence, VTRACE trace/evidence/review rows.

Entry criteria:

- Package-level evidence exists or each missing row has blocked/deferred/target
  disposition with owner and revisit trigger.
- Integration items in `INTEGRATION_PLAN.md` are ready or explicitly blocked.

Exit criteria:

- L2 validation rehearsal records command, fixture, snapshot, route/browser,
  demo, or target-not-met evidence.
- `TRACE.md`, `VERIFICATION.md`, `VALIDATION.md`, and `REVIEW.md` align.
- Release/transition readiness is either passed, blocked, or accepted with risk.

Verification commands:

```powershell
git diff --check
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Validation levels:

| Level | Required | Commands / Evidence | Result |
|---|---|---|---|
| L0 | yes | VTRACE proof and trace consistency checks. | passed_with_risk; `proof check` and `git diff --check` pass after closeout alignment |
| L1 | yes | Workspace checks. | passed; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-targets` pass after the Lindsay L3 golden refresh and MDI test initializer fix |
| L2 | yes | Rehearsal evidence for VAL-001..VAL-010 or explicit disposition. | passed_with_risk; VAL-001..VAL-010 are rehearsed through package evidence, regenerated goldens, and explicit WP-007 target-not-met disposition |

V closure: closed_with_risk. Predecessor packages have evidence or accepted
target-not-met disposition, and the final rehearsal records remaining residual
risks without promoting unsupported standalone/lean claims.

Validation impact: final integrated evidence pass before public readiness claims.

Risks: integrating too early can turn pending evidence into misleading claims.

Assurance/security classification: all lanes required for touched work; lanes
with no affected scope may be marked not required with rationale during the gate.

Review gate: Integration Readiness Review, Test Readiness Review, and
Release/Transition Readiness Review passed_with_risk on 2026-06-01.

Git execution:

- Branch/worktree: child-repo branch or release-prep branch after packages close.
- Commit plan: evidence and trace updates with any rehearsal fixtures.
- Push/PR condition: all open evidence rows are resolved, deferred, blocked, or
  target-not-met.
- Agent stop condition: stop if any validation result is promoted without
  command/artifact identity.

Status: closed_with_risk.

## WP-009: Major analytics cache foundation

Parent IDs:

- Requirements: `REQ-CACHE-001`, `REQ-CACHE-002`, `REQ-CACHE-003`,
  `REQ-CACHE-004`, `REQ-CODE-001`
- Interfaces: `IF-CACHE-001`, `IF-DATA-001`, `IF-VIEW-001`, `IF-REPORT-001`
- Design/architecture: major analytics cache target design, ADR-VT-006
- Validation: `VAL-011`
- Integration: `INT-009`
- Change control: `CHG-072`

Scope:

- Define and implement a versioned major analytics cache record and consumer
  envelope for future hockey decision surfaces.
- Build from explicit local/bundled/snapshot source state only; no cache read path
  may call live APIs.
- Carry provenance, freshness/staleness, source window, quality/completeness,
  warnings, invalidation keys, methodology version, and disclosure fields through
  cache records and consumer envelopes.
- Prove stale, partial, missing, schema-incompatible, unsupported metric, invalid
  key, and consumer-contract mismatch states refuse or degrade explicitly.
- Demonstrate at least one coach/scout/report/card-style consumer envelope without
  renderer-local recomputation of canonical analytics, confidence, or source-state
  meaning.

Out of scope:

- Claiming production cache availability before implementation fixtures pass.
- Autonomous coaching decisions, prediction accuracy, betting value, injury
  certainty, line-chemistry causality, or complete-world truth.
- Building all downstream dashboards/reports in the same package.

Entry criteria:

- CHG-072 target-spec baseline is accepted.
- Cache storage path, schema compatibility rule, first metric families, and first
  consumer fixture are selected before Rust implementation begins.
- Source-state fixtures are identified for complete, stale, partial, missing,
  unsupported, and schema-incompatible cache inputs.

Exit criteria:

- `IF-CACHE-001` record/envelope schema has compatibility fixtures.
- Cache build/read tests prove no query-time live fetch and explicit unavailable,
  stale, partial, unsupported, and invalidation behavior.
- Consumer contract tests prove downstream surfaces preserve prepared analytics,
  provenance, freshness, quality/completeness, warnings, and disclosure.
- `VALIDATION.md`, `VERIFICATION.md`, `TRACE.md`, `REVIEW.md`,
  `INTEGRATION_PLAN.md`, and wave pulse records point to concrete evidence.
- Required docs/source gates pass.

Verification plan:

| Level | Required | Planned Evidence | Status |
|---|---|---|---|
| L0 | yes | Schema/contract fixture tests for cache records, invalidation keys, version compatibility, and consumer envelopes. | partial; pulses 02-04 passed selected core schema serde, version compatibility, invalidation-key/methodology/consumer requirements, consumer-envelope fixtures, and internal consumer-ViewModel projection |
| L1 | yes | Tempdir/source-state fixtures for missing/stale/partial/schema/unsupported/no-live paths plus affected clippy/format tests if code is touched. | partial; pulse 03 adds a strict `icelines-fetch` cache store/read path with tempdir missing, stale/partial/missing-source preservation, schema/metric refusal, invalidation, and rollback fixtures |
| L2 | yes | At least one consumer demo or snapshot showing cache-backed decision-support envelope and non-claim disclosure. | partial; pulses 02-08 passed in-core, store-backed coach-dashboard envelope, internal dashboard-style ViewModel proofs, first named-cache Web report, first coach dashboard route, first opponent scout route, and first player evidence-card route evidence; broader line/goalie/practice/postgame surfaces are not claimed |

V closure: partial. The initial core schema/source/consumer contract slice exists
in `icelines-core::analytics_cache`, and the first strict JSON store/read path
exists in `icelines-fetch::analytics_cache_store`; the first internal consumer
ViewModel fixture exists in
`icelines-core::view_model::analytics_cache_consumer`; the first named-cache Web
report, first coach dashboard route, first opponent scout route, and first player
evidence-card route exist in `icelines-web::handlers::analytics_cache_report`.
Downstream line/goalie/practice/postgame surfaces and broader product-copy
reviews remain pending.

Validation impact: adds `VAL-011` for coach/analyst trust in a shared analytics
evidence layer.

Risks: a cache can amplify stale or partial data across many future screens; the
contract must refuse or disclose degraded state before any consumer claim.

Assurance/security classification: HART, Campbell, WIRE, TAPE, SCOUT, and BENCH
review lanes required before implementation closure.

Review gate: specification baseline accepted 2026-06-01; pulse 02 accepted the
initial core schema/consumer slice; pulse 03 accepted the strict store/read and
invalidation slice; pulse 04 accepted the internal downstream consumer ViewModel
fixture; pulse 05 accepted the first named-cache Web report; pulse 06 accepted
the first coach dashboard route; pulse 07 accepted the first opponent scout
route; pulse 08 accepted the first player evidence-card route. Implementation
closure requires broader shipped-surface consumer and closeout reviews.

Git execution:

- Branch/worktree: child-repo implementation branch or focused docs/spec branch.
- Commit plan: keep cache implementation/evidence in ICELINES before any TRACKER
  submodule pointer update.
- Push/PR condition: no cache consumer claim is promoted without schema,
  source-state, invalidation, and non-claim evidence.
- Agent stop condition: stop if a cache read path fetches live data, zero-fills
  missing hockey facts, or hides stale/partial source state.

Status: partial.

## Orphan Check

Before implementation starts, confirm:

- [x] Every accepted `REQ-*` is assigned to a work package or dispositioned;
  target cache requirements are assigned to WP-009.
- [x] Every interface-changing work package names `IF-*` IDs.
- [x] Every critical-code work package names `CR-*` IDs.
- [x] Every work package has exit criteria and verification commands.
- [x] Every work package lists L0/L1/L2 requirements or explicit non-requirement.
- [x] Every work package has V closure rows completed or marked by referenced
  trace/evidence rows.
- [x] Every required assurance/security review lane is complete or accepted with
  risk.
- [x] No work package is only "cleanup" without parent IDs or discovery status.

Open checklist item: none for this VTRACE wave. Assurance/security lane
decisions are accepted with risk through the WP-008 closeout review.
