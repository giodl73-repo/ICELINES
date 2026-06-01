# WP-008 Pulse 01 - Integration rehearsal and closeout

## Scope

Final integration rehearsal across VTRACE validation scenarios after all
predecessor packages were closed, closed with risk, or dispositioned.

## Evidence

- Regenerated Lindsay L3 golden outputs with `LINDSAY_L3_REGEN=1 cargo test -p
  icelines-cli --test lindsay_l3_golden l2_lindsay_l3_golden_parity --
  --nocapture` after the integration rehearsal exposed stale text fixtures.
- The updated golden fixtures now preserve the CLI text context/result lines
  emitted by the source-state and result-state work.
- Fixed the broad clippy `field_reassign_with_default` finding in the MDI test
  initializer by constructing `MdiLayout` with struct update syntax.
- Fixed the broad persona regression where invalid `tonight` and `schedule`
  dates returned the no-live message with a zero exit before argument validation.
- Fixed the broad persona regression where plain `fetch boxscore --dry-run`
  returned a live-feed-disabled error before printing its dry-run response, while
  preserving the WP-005 `--for-favorites` no-live refusal before cache writes.
- Fixed the broad Web route regression where immutable read-only fantasy GETs
  could miss freshly written FantasyDb rows by checkpointing WAL writes promptly,
  preserving the no-SQLite-sidecar read-only contract.
- Focused regression gate: `cargo test -p icelines-cli --test persona_foster`.
- Focused regression gate: `cargo test -p icelines-cli --test persona_wave6`.
- Focused regression gate: `cargo test -p icelines-web --test l1_router`.
- Final broad gates:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --all-targets`
  - `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`
  - `git diff --check`

## Validation disposition

| Scenario | Result | Notes |
|---|---|---|
| VAL-001 | passed_with_risk | Covered by selected TUI L0/L2 snapshot evidence; full interactive transcript remains residual risk. |
| VAL-002 | passed_with_risk | Covered by WP-004 selected historical/report fixtures; full report/export matrix remains residual risk. |
| VAL-003 | passed_with_risk | Covered by WP-003 route/no-JS/launch evidence; live browser touch/focus breadth remains residual risk. |
| VAL-004 | passed_with_risk | Covered by selected CLI/Web/TUI/export parity evidence plus Lindsay L3 golden rehearsal. |
| VAL-005 | passed_with_risk | Covered by selected no-live/offline and missing-source evidence. |
| VAL-006 | passed_with_risk | Covered by selected data/fetch/status/snapshot and partial-refresh evidence. |
| VAL-007 | passed_with_risk | Covered by selected fantasy read/local-state and command/API transcript evidence. |
| VAL-008 | passed_with_risk | Covered by selected upstream failure, schema, integrity, CSV, abbreviation, and resume evidence. |
| VAL-009 | passed_with_risk for broad gates; dependency/lean target-not-met | Broad workspace gates pass; FLETCH/SLICE and lean CLI remain explicitly unpromoted. |
| VAL-010 | passed_with_risk | Covered by named-layout durable reload/Web restore and focused TUI restore test evidence. |

## Decision

`WP-008` is `closed_with_risk`.

The VTRACE baseline is caught up to the current implementation evidence. No
standalone/lean release claim is made until WP-007's revisit trigger is satisfied.
