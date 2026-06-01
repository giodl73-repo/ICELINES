# WP-004 Report/Export Historical Perspective Wave

## Scope

WP-004 makes public historical perspective and report/export artifacts
fixture-backed, disclosure-forward, and free of unsupported claims. The
controlling baseline is `docs/vtrace/WORK_PACKAGES.md` WP-004.

## Package posture

| Pulse | Slice | Evidence | Status |
|---|---|---|---|
| 01 | Public Markdown export disclosure guardrail | `icelines-cli/src/commands/export.rs`; `cargo test -p icelines-cli l0_export_leaders -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-cli --no-deps -- -D warnings` | passed_with_risk |
| 02 | Active streak status label | `icelines-core/src/view_model/streaks.rs`; `icelines-cli/src/commands/streaks.rs`; `icelines-cli/src/tui/screens/player_streaks.rs`; `cargo test -p icelines-core view_model::streaks::tests:: --quiet`; `cargo test -p icelines-cli streaks --quiet`; `cargo fmt --check`; affected core/CLI clippy | passed_with_risk |
| 03 | Completeness/skeleton disclosure guardrail | `icelines-cli/src/commands/export.rs`; `cargo test -p icelines-cli l0_export_leaders_discloses_methodology_limits_near_top --quiet`; `cargo fmt --check` | passed_with_risk |
| 04 | GP threshold export evidence | `icelines-cli/src/commands/export.rs`; `cargo test -p icelines-cli l0_export_leaders_gp_min_filters_rows_and_reports_threshold --quiet` | passed_with_risk |
| 05 | Duplicate and Unicode name export evidence | `icelines-cli/src/commands/export.rs`; `cargo test -p icelines-cli l0_export_leaders_preserves_unicode_and_duplicate_names_as_rows --quiet` | passed_with_risk |
| 06 | Trade continuity export evidence | `icelines-cli/src/commands/export.rs`; `cargo test -p icelines-cli l0_export_leaders_traded_player_renders_once_with_last_stint_team --quiet` | passed_with_risk |
| 07 | Lockout and October rollover season-window export evidence | `icelines-cli/src/commands/export.rs`; `cargo test -p icelines-cli l0_export_leaders_honors_lockout_and_october_rollover_windows --quiet` | passed_with_risk |
| 08 | Full-lockout season skip evidence and closeout | `icelines-cli/src/commands/data.rs`; `cargo test -p icelines-cli l0_available_seasons_skip_full_lockout_and_keep_neighbors --quiet` | closed_with_risk |

## Residual package risks

- Broader active-streak parity, ambiguous-name disambiguation breadth, and full
  cross-surface report/export matrix coverage are accepted residual risks for
  WP-008 rehearsal.
- `VAL-002` is closed_with_risk for the selected fixture set. Report/export
  portions of `VAL-004` remain pending overall until the integration rehearsal.
