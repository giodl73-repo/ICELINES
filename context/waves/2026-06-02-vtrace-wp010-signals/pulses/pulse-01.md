# WP-010 Pulse 01 - Core IceLines Signals

Date: 2026-06-02

## Scope

Add the first core-only IceLines Signals metric family without promoting stable
`StatId` catalog entries or shipped user-facing surfaces.

The slice introduces:

- Physical Engagement Rate
- Puck Management Differential
- Penalty Drag Rate

## Changes

- Added `icelines-core::signal_metrics` with signal metric IDs, descriptors,
  units, polarity, required inputs, typed evidence tiers, and pure read methods.
- Kept signals separate from `StatId` so the family can be reviewed before it
  reaches stable stat catalog, cache, CLI, TUI, Web, or report/export surfaces.
- Returned `None` for missing realtime inputs, missing or tiny TOI, and
  below-threshold samples instead of zero-filling unavailable evidence.
- Added L0 tests for descriptor stability, formulas, missing-data behavior,
  sample/TOI refusal, ordering behavior, and lower-is-better penalty polarity.
- Documented the initial signal set and promotion rule in
  `design/specs/icelines-signals.md`.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-core signal_metrics --quiet` | passed |
| L0 | `cargo fmt --check` | passed |
| L1 | `cargo clippy -p icelines-core --lib --tests -- -D warnings` | passed |
| VTRACE | `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| Hygiene | `git diff --check` | passed |

## Residual risk

- Physical Engagement Rate and Puck Management Differential use scorer-recorded
  realtime events and must disclose scorer bias on any future public surface.
- Penalty Drag Rate is descriptive PIM/60, not proof of avoidable team harm.
- No surface, cache, or report/export claim is accepted until a later pulse adds
  product-copy and parity/source-state evidence.
