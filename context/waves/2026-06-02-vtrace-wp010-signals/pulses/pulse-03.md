# WP-010 Pulse 03 - Signals CLI + JSON surface (Phase Hurricane)

Date: 2026-06-18

## Scope

Ship the first user-facing Signals surface: a read-only `icelines signals
"<player>"` command that renders the existing `PlayerSignalsView` as a text table
and, with `--json`, a frozen `signals.v1` envelope. Scoped to CLI + JSON only;
Signals stay out of `StatId`, the `--filter` catalog, leaderboards, TUI, Web,
reports, exports, and the analytics cache.

## Changes

- Added `icelines-cli/src/commands/signals.rs` — resolves a player (active-season
  plus historical bundled-season name fallback + lazy career fan-out, mirroring
  `commands::query::run_player`) and renders `PlayerSignalsView`. All Signal math
  stays in `icelines-core::signal_metrics`; the command only resolves + renders
  (Contract 3).
- Text surface prints value or `unavailable` (never `0.0`) with polarity arrow,
  per-60 unit, evidence tier, and missing-input labels, followed by methodology,
  limitations, disclosures, and the non-claim disclaimer.
- `--json` emits a frozen `signals.v1` envelope (`schema`, `schema_version`,
  `route`, `data` = serialized `PlayerSignalsView`, `meta`).
- Registered `Signals { player, season, season_type, json }` in `cli.rs` with a
  `long_about` carrying the example set, unit/polarity legend, and non-claim copy;
  dispatched in `main.rs`; module added to `commands/mod.rs`.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-cli --bin icelines signals` (4 signal L0 tests) | passed (6 ran incl. 2 incidental) |
| L2 | `cargo test -p icelines-cli --test signals_system` (3 system tests) | passed |
| L0 | `cargo fmt -p icelines-cli -- --check` | passed |
| L1 | `cargo clippy -p icelines-cli -- -D warnings` | passed |
| Smoke | `icelines signals "Connor McDavid"` and `--json` (offline) | passed |

The L2 honesty fence `l2_signals_missing_evidence_renders_null_not_zero` proves
1988-89 skeleton-season Gretzky returns `null` for Physical Engagement Rate (no
realtime), not `0.0`.

## Promotion-rule gate (design/specs/icelines-signals.md)

- Product copy reviewed for the CLI surface (labels, methodology, limitations,
  disclaimer in command output + `long_about`).
- Source/completeness disclosure: `unavailable`/`null` + evidence tier + missing
  inputs for partial/missing evidence.
- Parity: CLI text + JSON are one ViewModel, two encodings. TUI/Web parity fence
  deferred to pulse-04.
- Cache-envelope methodology: N/A (not cached).
- Explicit refusal of predictive/betting/injury/deployment/coaching claims:
  printed `non_claims` line + `long_about`.

## Residual risk

- CLI-only by design this pulse; TUI/Web parity is pulse-04 and must add a
  cross-surface identity fence.
- Name resolution uses case-insensitive `full_name` contains; accent-insensitive
  matching parity with `query player` is a pulse-04 refinement.
- Signals remain descriptive and scorer-biased where they use realtime
  rink-recorded events.
