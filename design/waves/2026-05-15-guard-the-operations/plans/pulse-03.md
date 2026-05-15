---
wave: guard-the-operations
pulse: 03
date: 2026-05-15
status: planned
governing_roles:
  - wire
  - bench
  - forge
  - glass
---

# Pulse 03 - Admin Data Operation Safety

## Goal

Resolve the web admin data-operation partials conservatively. Safe fixture-backed
operations should be tested and labeled; live/network installs and destructive
removes must either gain a scoped confirmation/dry-run contract or remain
explicitly deferred.

## Owned Scope

- Audit `/admin` and `/api/v1/admin/*` data operation routes.
- Verify game-cache load and favorites-cache load have fixture-backed tests and
  clear source-state/user feedback.
- Decide whether data install/remove can be exposed safely without live network
  tests or real-user filesystem risk.
- Update `ADMIN-OPERATIONS-INVENTORY.md` or create a successor note if the
  decision table changes.

## Non-goals

- No live release download tests.
- No broad data-store refactor.
- No destructive filesystem mutation without explicit scoped confirmation.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -p icelines-web --no-deps -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-guard-the-operations design\specs\surface-parity.md COMMANDS.md --errors-only`
