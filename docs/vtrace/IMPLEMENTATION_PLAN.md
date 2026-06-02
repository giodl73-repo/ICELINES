# Implementation Plan

## Scope

Repo or feature: `icelines` repo-baseline VTRACE implementation.

Implementation baseline: 2026-05-30 docs-first VTRACE baseline plus the VTRACE
implementation-management update. This plan moves IceLines from accepted
requirements/design controls into controlled work packages, integration gates,
and evidence-bearing implementation slices.

This file does not claim implementation closure. It defines the control loop for
future code, fixture, route, browser, dependency, and validation work.

## Baseline Inputs

| Artifact | Status | Notes |
|---|---|---|
| `MISSION.md` | accepted | Mission targets include personalized workbench layouts, descriptive public reporting, offline confidence, and lean/standalone goals. |
| `CONOPS.md` | accepted | Ten scenarios define the operating workflows and validation intent, including the target major analytics cache workflow. |
| `REQUIREMENTS.md` | accepted with targets | Accepted requirements are implementation inputs; `REQ-WB-003` is passed_with_risk for WP-002, `REQ-DEP-001` and `REQ-LEAN-001` remain target posture, and `REQ-CACHE-001..004` are partial until shipped product-surface evidence passes. |
| `ARCHITECTURE.md` | accepted with risk | Architecture freezes active surfaces, source-state posture, and target/deferred claims. |
| `INTERFACES.md` | accepted with open questions | Interfaces are controlled; `IF-LAYOUT-001` and `IF-BUILD-001` require implementation detail before closure. |
| `DESIGN.md` | accepted with risk | Design allocates ViewModels, source-state propagation, TUI/Web state, reports, and target Cargo features. |
| `CODE_RIGOR.md` | accepted | Implementation PRs must use these constraints as gates, not advisory text. |
| `VERIFICATION.md` | accepted with pending evidence | Command/evidence matrix exists; most implementation evidence remains pending. |
| `VALIDATION.md` | accepted with pending evidence | `VAL-001` through `VAL-009` remain pending; `VAL-010` is passed_with_risk for WP-002 named layout persistence. |
| `TRACE.md` | accepted with pending evidence | End-to-end trace exists and now receives work-package links. |
| `REVIEW.md` | accepted with risk | Prior reviews allow implementation planning without claiming Gate 3 readiness. |
| `STAGE_EXECUTION.md` | active | Stage board records S0-S6 status, gate posture, and WP-002 S4/S5 next actions. |
| `ROLE_RECOMMENDATIONS.md` | active | Maps VTRACE review lanes to ICELINES `.roles` files and WP-002 review order. |

## Implementation Strategy

Implementation will proceed by coherent work packages, not broad repo cleanup.
Each package must close its own small V: parent requirements, interfaces/design,
code-rigor constraints, implementation changes, verification commands,
validation impact, trace updates, and review gate.

Each implementation pulse must be recorded under `context/waves/**/pulses/`
with the active `WP-*`, parent IDs, boundary IDs, allowed/forbidden scope,
L0/L1/L2 evidence, required VTRACE file updates, and gate outcome. The active
WP-001 execution records are under
`context/waves/2026-05-30-vtrace-wp001-parity/pulses/`; the WP-002 layout
execution record is
`context/waves/2026-05-30-vtrace-wp002-layout/pulses/pulse-01.md`.

The current implementation-ready product sequence is WP-001 parity foundation
pulses followed by the already closed WP-002 named workbench layout slice. Pulse
12 adds selected default CLI text query context/source-state presentation only;
it must not be mixed with dependency feature surgery or unrelated fetch/feature
work.

Target-only rows remain honest:

- `REQ-WB-003` is passed_with_risk for WP-002 named layout persistence; broader
  workbench validation and workspace lint debt remain open.
- `REQ-DEP-001` is target-not-met until FLETCH/SLICE seams are removed or
  replaced with command-surface evidence.
- `REQ-LEAN-001` is target-not-met until the lean CLI feature build and offline
  smoke pass.
- `REQ-CACHE-001..004` have partial WP-009 implementation evidence for the
  versioned analytics cache schema, source-state/invalidation behavior, strict
  store/read path, internal consumer ViewModel, and first named-cache Web report;
  broader shipped product surfaces remain pending.
- `REQ-SIGNAL-001` has partial WP-010 core-only implementation evidence for the
  first IceLines Signals descriptors, formulas, and missing-input evidence; no
  stable stat catalog or shipped surface claim is accepted.

## Sequencing

| Order | Work Package | Why This Order |
|---:|---|---|
| 1 | WP-001 Source-state and ViewModel parity foundation | Shared semantics must be stable before surface evidence can close. |
| 2 | WP-002 Named workbench layout persistence | Directly addresses the highest-value pending mission target and exercises TUI/Web/local-state integration. |
| 3 | WP-003 Web dashboard route and browser safety | Builds on shared state and layout/URL contracts; closes first-session browser risks. |
| 4 | WP-004 Historical perspective report/export evidence | Public-copy and fixture evidence should land before external sharing claims. |
| 5 | WP-005 Offline, fetch, and data-depth reliability | Requires source-state discipline plus tempdir/httpmock fixture work. |
| 6 | WP-006 Fantasy read-model and local-state safety | Shares ViewModel/local-state constraints and must preserve mutation boundaries. |
| 7 | WP-007 Standalone dependency and lean CLI target | Higher-risk Cargo/dependency work waits until command-surface inventory is ready. |
| 8 | WP-008 Integration and validation rehearsal | Runs cross-surface L2 evidence after package-level work is closed or explicitly deferred. |
| 9 | WP-009 Major analytics cache foundation | Defines the shared analytics evidence layer before downstream coach/scout/report/card/line/goalie/practice/postgame front ends are built. |
| 10 | WP-010 IceLines Signals core metric family | Starts new descriptive metric methodology behind a core evidence API before any stat catalog, cache, report, or surface promotion. |

## Source-To-Work-Package Mapping

Every accepted or target requirement is assigned to at least one work package or
an explicit target disposition.

| Source IDs | Work Package | Disposition | Notes |
|---|---|---|---|
| REQ-WB-001 / DES-003 / DES-006 / IF-QUERY-001 / IF-VIEW-001 / CR-012 / CR-013 / CR-032 | WP-001; WP-002 | implement | Workbench flow depends on shared query semantics and TUI state discipline. |
| REQ-WB-002 / DES-002 / DES-014 / IF-DATA-001 / IF-VIEW-001 / IF-WEB-001 / CR-007 / CR-015 / CR-032 | WP-001; WP-003 | implement | Active context is a cross-surface invariant. |
| REQ-WB-003 / DES-015 / IF-LAYOUT-001 / CR-020 / VAL-010 | WP-002 | closed_with_risk | Schema, migration/refusal behavior, CLI durable reload, TUI restore test, and Web restore demo pass for named layouts; broader `CR-020` and workspace lint debt remain open. |
| REQ-QUERY-001 / DES-003 / IF-QUERY-001 / CR-003 / CR-010 | WP-001 | implement | Query parser/planner parity is a foundation for later surface work. |
| REQ-STAT-001 / DES-008 / DES-013 / IF-REPORT-001 / CR-017 / CR-026 / CR-028 | WP-004 | implement | Descriptive historical perspective must be fixture-backed before public claims. |
| REQ-STAT-002 / DES-013 / IF-DATA-001 / IF-FETCH-001 / CR-025 / CR-030 | WP-004; WP-005 | implement | Edge fixtures split across report expectations and data/fetch source behavior. |
| REQ-REPORT-001 / DES-001 / DES-008 / IF-REPORT-001 / CR-016 / CR-017 | WP-004 | implement | Report/export snapshots carry disclosure and anti-overclaim checks. |
| REQ-WEB-001 / DES-007 / IF-WEB-001 / IF-VIEW-001 / CR-014 / CR-027 | WP-003 | implement | Browser cold-start, URL state, and no-JS/read-only route behavior. |
| REQ-WEB-002 / DES-007 / DES-014 / IF-WEB-001 / CR-015 / CR-027 / CR-032 | WP-003 | implement | Browser/accessibility/recovery evidence. |
| REQ-PARITY-001 / DES-001 / DES-009 / IF-VIEW-001 / IF-DATA-001 / IF-QUERY-001 / CR-006 / CR-010 / CR-011 | WP-001; WP-008 | implement | Package-level parity first, then L2 integration rehearsal. |
| REQ-DATA-001 / DES-002 / DES-004 / IF-DATA-001 / IF-FETCH-001 / CR-007 / CR-008 / CR-024 | WP-001; WP-005 | implement | Source-state vocabulary must survive to ViewModels and fetch/offline flows. |
| REQ-OFFLINE-001 / DES-004 / DES-005 / IF-DATA-001 / IF-FETCH-001 / CR-009 / CR-018 / CR-019 | WP-005 | implement | Offline behavior and no query-time live fetch require fixture evidence. |
| REQ-DATA-DEPTH-001 / DES-005 / DES-012 / IF-FETCH-001 / CR-018 / CR-019 / CR-031 | WP-005 | implement | Data install/fetch/status/snapshot and locked shift-level refusal evidence. |
| REQ-FANTASY-001 / DES-007 / IF-VIEW-001 / IF-WEB-001 / CR-014 / CR-020 | WP-006 | implement | Read ViewModels and safe mutation deferral must share local-state constraints. |
| REQ-FRESH-001 / DES-012 / IF-FETCH-001 / IF-DATA-001 / CR-018 / CR-019 / CR-029 / CR-031 | WP-005 | implement | Upstream/cache failure handling is fixture-backed. |
| REQ-DEP-001 / DES-010 / DES-011 / IF-BUILD-001 / CR-021 / CR-022 | WP-007 | target | Starts only after command-surface replacement/refusal inventory exists. |
| REQ-LEAN-001 / DES-010 / IF-BUILD-001 / CR-021 | WP-007 | target | Remains target-not-met until feature surgery and lean build evidence pass. |
| REQ-CACHE-001..004 / IF-CACHE-001 / CR-033 / VAL-011 | WP-009 | partial | Major analytics cache has initial schema/source-state/invalidation/store and internal consumer fixtures; shipped product-surface claims remain pending until copy and consumer evidence pass. |
| REQ-SIGNAL-001 / IF-SIGNAL-001 / DES-016 / VAL-012 | WP-010 | partial | First core-only Signals descriptors and formula/evidence fixtures exist; shipped surfaces and stable `StatId` promotion remain pending until product-copy/source-state evidence exists. |
| REQ-CODE-001 / CODE_RIGOR.md / VERIFICATION.md command matrix / CR-005 / CR-023 | All packages; WP-008 | implement | Code rigor is a closure gate for every package. |

Disposition values: `implement`, `implement target`, and `target` are used here.
Target rows are planned but must not become `passed` until the named evidence
exists.

## Branch / Change Control

Branch strategy: one child-repo branch per implementation package. Do not mix
ICELINES implementation commits with TRACKER submodule pointer commits.

Worktree strategy: make canonical implementation and VTRACE edits in
`C:\src\ICELINES`; after validation, copy VTRACE docs into
`C:\src\TRACKER\repos\applied-systems\icelines` and update TRACKER pointers only
as a separate portfolio action.

Change-control trigger: update `CHANGE_CONTROL.md` before or with any change
that alters requirement meaning, public interface behavior, architecture
boundary, validation claim, verification method, accepted risk, or code-rigor
constraint.

Rollback or revert strategy: each package identifies affected files, local-state
migration behavior, and feature flags or refusal paths before implementation
closure. Destructive local-state changes require explicit backup/rollback notes
and review approval.

## Commit / Push Policy

Commit scope: commit one work package or coherent sub-slice at a time, with
trace/evidence updates in the same child-repo change when the code changes close
or affect VTRACE rows.

Push condition: L0 checks pass and package-specific pending evidence is either
recorded, target-not-met, blocked, or deferred with owner/revisit trigger.

Merge/readiness condition: L1 checks pass, required assurance/security lanes are
complete or accepted with risk, `TRACE.md` and `REVIEW.md` identify evidence, and
no target row is overclaimed.

## Integration Strategy

Use `INTEGRATION_PLAN.md` for cross-module integration items. Integration is
required when a work package touches shared ViewModels, query/fetch state, TUI
and Web behavior together, URL or local-state schema, reports/exports, Cargo
features, or validation scenarios that need end-to-end proof.

`WP-002` integrates a versioned layout model through local state, TUI restore,
and Web URL/bookmark behavior; `VAL-010` is now passed_with_risk for named
layout persistence.

`WP-001` now has focused leaders evidence through pulse 10: CLI/Web JSON and Web
HTML identity/source/context/result/empty-warning checks, selected Web HTML
recovery and active-context parity, plus selected TUI context/source-state
presentation and selected Markdown export context/source-state presentation. Full
interactive TUI parity, full report/export, provenance, query planner, and broad
browser route/accessibility evidence remain pending.

For `WP-002`, `CHG-001` controls the selected schema/store/URL decision:
versioned records in `~/.icelines/layouts.json`, CLI layout management,
`icelines tui --layout <name>`, and `/dashboard?layout=<name>`.

`WP-008` is reserved for the integration/validation rehearsal after package-level
evidence exists or is explicitly dispositioned.

`WP-009` follows as the major analytics cache foundation. It should first close a
schema/contract slice, then source-state and invalidation behavior, then a
consumer-envelope demo. It must not ship downstream coach/scout/report/card
surfaces in the same pulse unless their cache contract evidence is separately
traceable.

## Verification Strategy

Default documentation checks for VTRACE-only changes:

```powershell
git -C C:\src\ICELINES diff --check
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
```

Default implementation checks unless a package records a narrower affected-slice
rationale:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Package-specific checks are recorded in `WORK_PACKAGES.md` and promoted into
`VERIFICATION.md` evidence rows when they pass.

## Validation Levels

| Level | Scope | Required Commands / Evidence | Required Before |
|---|---|---|---|
| L0 | Fast local sanity | `git diff --check`; affected unit tests, route tests, parser fixtures, or tempdir/httpmock fixtures named by the work package. | commit |
| L1 | Full repo confidence | `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --all-targets` or justified affected-slice equivalent. | push / PR |
| L2 | Integration or release readiness | Cross-surface CLI/TUI/Web/report demos, route/browser inspection, snapshot/export comparison, dependency inspection, lean build smoke, or validation rehearsal evidence as applicable. | merge / release / downstream adoption |

## Risks

| Risk ID | Risk | Mitigation | Owner |
|---|---|---|---|
| RISK-IMPL-001 | A work package could close code but leave evidence rows pending. | Work-package close review requires `TRACE.md`, `VERIFICATION.md`, and `REVIEW.md` updates. | BENCH / Jim Gregory |
| RISK-IMPL-002 | Named layouts could persist renderer-local state and be mistaken for portable semantics. | `WP-002` requires `IF-LAYOUT-001`, versioned schema, migration/refusal behavior, and TUI/Web restore evidence. | Jack Adams / GLASS |
| RISK-IMPL-003 | Source-state or active-context changes could drift differently across CLI, TUI, Web, JSON, and reports. | `WP-001` and `WP-008` require ViewModel/envelope parity checks before cross-surface claims. | Campbell / HART |
| RISK-IMPL-004 | Cargo dependency work could silently remove commands or overclaim lean support. | `WP-007` cannot start closure until command-surface inventory and rollback/refusal paths exist. | KEEL / FORGE |
| RISK-IMPL-005 | Browser or local-state work could expose unsafe mutation or destructive migration behavior. | `WP-003`, `WP-006`, and `WP-002` require GET-read-only, POST/defer mutation boundaries, and local-state preservation checks. | CREST / broadcast |

## Implementation Readiness Decision

Decision: pass_with_risk

Rationale: the VTRACE baseline is controlled enough to plan implementation
packages. Implementation may begin with `WP-002` only after its entry criteria
are confirmed in `WORK_PACKAGES.md`; closure still requires command, fixture,
snapshot, route/browser, demo, or explicit target-not-met evidence. No
implementation status changes are granted by this plan alone.
