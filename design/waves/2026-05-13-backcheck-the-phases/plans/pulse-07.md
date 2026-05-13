---
wave: backcheck-the-phases
pulse: 07
date: 2026-05-13
status: planned
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

- [ ] `cargo test -p icelines-core admin`
- [ ] `cargo test -p icelines-cli config`
- [ ] `cargo test -p icelines-cli data`
- [ ] `cargo test -p icelines-web admin`
- [ ] `cargo fmt --check`
- [ ] `surface-parity.md` distinguishes implemented, deferred, and dangerous admin operations.

## Stop Conditions

- Stop if a destructive action cannot be expressed as a typed POST-backed
  mutation intent/result.
- Stop if a web handler would perform live network fetch/install work without an
  existing fixture-backed test.
- Stop if the pulse starts changing unrelated release-smoke policy.
