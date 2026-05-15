---
wave: guard-the-operations
pulse: 03
date: 2026-05-15
status: done
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

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -p icelines-web --no-deps -- -D warnings`
- [x] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-guard-the-operations design\specs\surface-parity.md COMMANDS.md --errors-only`

## Result

Kept web data install/remove deliberately deferred. `/admin` now labels
game-cache controls as POST-backed cache warmers rather than release bundle
install/remove operations, renders explicit data install/remove deferral copy,
and tests prove install/remove routes remain unmounted while invalid game-cache
requests are rejected before network work.
