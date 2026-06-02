# WP-010 Pulse 02 - Signals ViewModel Boundary

Date: 2026-06-02

## Scope

Add the first internal ViewModel boundary for IceLines Signals without promoting
Signals to stable `StatId`, cache, CLI, TUI, Web, or report/export surfaces.

## Changes

- Added `PlayerSignalsView` and `PlayerSignalRow` in
  `icelines-core::view_model::signals`.
- Preserved player identity, active season context, signal descriptors, units,
  polarity, computed values, evidence tiers, missing inputs, methodology,
  limitations, disclosures, and non-claim copy in one canonical row shape.
- Added serde support for signal IDs, units, polarity, evidence tiers, missing
  input labels, and the internal Signals ViewModel.
- Proved the ViewModel does not zero-fill missing realtime evidence and still
  allows Penalty Drag Rate when its required inputs are present.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-core signal --quiet` | passed |
| L0 | `cargo fmt --check` | passed |
| L1 | `cargo clippy -p icelines-core --lib --tests -- -D warnings` | passed |
| VTRACE | `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| Hygiene | `git diff --check` | passed |

## Residual risk

- This is an internal core ViewModel only; no user-facing Signals route, command,
  report, export, leaderboard, cache metric family, or stable `StatId` is
  accepted.
- Future surfaces must preserve the ViewModel evidence and non-claim copy instead
  of recomputing signal meaning locally.
