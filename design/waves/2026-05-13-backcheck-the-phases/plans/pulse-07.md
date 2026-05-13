---
wave: backcheck-the-phases
pulse: 07
date: 2026-05-13
status: done
depends_on: [pulse-01]
governing_roles:
  - keel
  - forge
  - tape
  - wire
  - bench
---

# Pulse 07 - Web Admin Operations Parity

## Mission

Backfill the operational partials left by Ted Lindsay and Jim Gregory: web admin
status/verify/config exists, but destructive install/remove and full persistent
config/report-toggle behavior must either become safe POST-backed mutations or
remain explicitly deferred.

## Deliverables

- Inventory admin data, snapshot, config, and report-toggle capabilities across
  CLI, TUI admin overlay, web HTML, and web JSON.
- Implement only safe, typed, POST-backed admin mutations that already have
  `MutationResultView` support.
- Keep inactive snapshot delete/activate backend guards intact.
- Update `design/specs/surface-parity.md`, `COMMANDS.md`, and release/admin docs
  with true status.

## Likely Files

- `icelines-core/src/view_model/admin.rs`
- `icelines-core/src/view_model/mutation.rs`
- `icelines-cli/src/commands/config.rs`
- `icelines-cli/src/commands/data.rs`
- `icelines-cli/src/tui/admin.rs`
- `icelines-web/src/handlers/admin.rs`
- `icelines-web/templates/admin.html`
- `design/specs/surface-parity.md`
- `COMMANDS.md`

## Gates

- [x] `cargo test -p icelines-core admin` - 1 matched test passed.
- [x] `cargo test -p icelines-cli config` - 98 matched tests passed.
- [x] `cargo test -p icelines-cli data` - 85 matched tests passed.
- [x] `cargo test -p icelines-web admin` - 18 matched tests passed.
- [x] `cargo fmt --check`
- [x] `surface-parity.md` distinguishes implemented, deferred, and dangerous admin operations.

## Pulse 07 Result

- Created `ADMIN-OPERATIONS-INVENTORY.md` with the WIRE decision table across
  CLI, TUI, web HTML, and web JSON.
- Kept web data install/remove and persistent report-toggle UI deferred; no live
  network install or unscoped destructive data removal was added to web admin.
- Added fixture-backed web admin safety coverage for absent data install/remove
  controls and active-snapshot delete rejection.
- Added a core ViewModel test proving config/data/snapshot admin mutations share
  `MutationResultView`.

## Stop Conditions

- Stop if a destructive action cannot be expressed as a typed POST-backed
  mutation intent/result.
- Stop if a web handler would perform live network fetch/install work without an
  existing fixture-backed test.
- Stop if the pulse starts changing unrelated release-smoke policy.
