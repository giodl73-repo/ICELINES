# Pulse 01: Named layout schema and restore surfaces

## Goal

Implement and evidence the first `WP-002` slice: a shared named layout schema,
CLI save/update/list/show/delete operations, TUI restore, and Web dashboard
restore through bookmark-safe state.

## VTRACE Work Package

Active work package: `WP-002`.

Parent IDs: `REQ-WB-003`; `REQ-WB-001`; `REQ-WB-002`; `REQ-WEB-001`.

Boundary IDs: `DES-006`; `DES-007`; `DES-015`; `IF-LAYOUT-001`;
`IF-WEB-001`; `IF-VIEW-001`; `CR-012`; `CR-013`; `CR-014`; `CR-020`;
`CR-032`; `INT-002`; `CHG-001`.

Review gate: Work Package Close Review, preceded by this S4 execution
checkpoint.

## Execution Scope

Allowed files/modules:

- `icelines-core/src/workbench_layout.rs`
- `icelines-core/src/lib.rs`
- `icelines-cli/src/cli.rs`
- `icelines-cli/src/config.rs`
- `icelines-cli/src/main.rs`
- `icelines-cli/src/commands/layout.rs`
- `icelines-cli/src/commands/mod.rs`
- `icelines-cli/src/commands/menu.rs`
- `icelines-cli/src/commands/serve.rs`
- `icelines-cli/src/tui/mod.rs`
- `icelines-cli/src/tui/mdi.rs`
- `icelines-web/src/config.rs`
- `icelines-web/src/handlers/dashboard.rs`
- `docs/vtrace/*.md`
- `context/waves/2026-05-30-vtrace-wp002-layout/**`

Forbidden files/modules:

- Dependency removal, FLETCH/SLICE replacement, and lean CLI feature work
  reserved for `WP-007`.
- Historical report/export fixture work reserved for `WP-004`.
- Fetch/offline/data-depth reliability work reserved for `WP-005`.
- Fantasy mutation/read-model closure reserved for `WP-006`, except shared
  local-state safety evidence already named by `CR-020`.

Discovery allowed: yes, limited to directly affected layout, local-state, TUI,
Web dashboard, and VTRACE evidence files.

## Validation

| Level | Command / Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-core workbench_layout --lib` | passed: 4 tests |
| L0 | `cargo test -p icelines-web dashboard --lib` | passed: 41 tests, including `l0_dashboard_named_layout_restores_bookmark_safe_state` |
| L0 | `cargo test -p icelines-cli l0_layout_cli_save_and_show_round_trip --bin icelines` | passed |
| L0 | `cargo test -p icelines-cli l0_mdi_applies_persisted_workbench_layout --bin icelines` | passed |
| L1 | `cargo fmt --check` | passed |
| L1 | `cargo clippy --workspace --all-targets -- -D warnings`; broader workspace tests or affected-slice rationale | partial: core clippy passed; broad workspace/web/CLI clippy blocked by unrelated existing lints outside WP-002 |
| L2 | `VAL-010` restart/reload demo plus stored-layout inspection | passed_with_risk: CLI reload/store inspection and Web restore demo passed; TUI restore covered by bin test |

## Trace And Evidence Updates

| File | Required Update | Status |
|---|---|---|
| `docs/vtrace/WORK_PACKAGES.md` | Move `WP-002` to active execution with real paths, CHG/stage/pulse, and L0/L1/L2 evidence posture. | done |
| `docs/vtrace/TRACE.md` | Record `REQ-WB-003`, `IF-LAYOUT-001`, `VAL-010`, and WP-002 evidence with explicit L1 risk. | done |
| `docs/vtrace/VERIFICATION.md` | Record WP-002 L0/L1/L2 command evidence and remaining workspace lint gap. | done |
| `docs/vtrace/VALIDATION.md` | Update `EVID-VAL-010` with L2 demo evidence and accepted TUI transcript limitation. | done |
| `docs/vtrace/REVIEW.md` | Add S4 pulse checkpoint and open close-review risks. | done |

## Outcome

Status: closed_with_risk.

Evidence: shared layout schema/store, CLI management commands, TUI restore hook,
Web dashboard `layout=<name>` restore, focused L0 tests, CLI durable reload
inspection, and Web L2 restore demo are present.

Open risks: full workspace clippy confidence remains blocked by unrelated
existing lint debt, and broader `EVID-CR-008` local-state safety remains outside
this slice. `REQ-WB-003`, `IF-LAYOUT-001`, `VAL-010`, and `EVID-VAL-010` are
therefore passed with risk for WP-002 rather than globally closed.

Next pulse: start the next VTRACE work package or WP-008 integration rehearsal;
do not use this close to claim full workspace clippy or broader local-state
safety.
