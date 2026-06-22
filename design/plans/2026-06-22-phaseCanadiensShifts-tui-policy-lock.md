# Phase Canadiens Shifts - TUI policy lock

Status: Closed

## Intent

Carry the locked shift-capability policy into the TUI Admin capability matrix so
dashboard users see that shifts remain off because there is no supported shift
source/bundle/fetch policy yet.

## Scope

- Expand the Admin overlay's `shifts` capability row from a generic locked label
  to a policy-lock label.
- Keep the capability value itself at `off`.
- Add a focused render test that proves the Admin overlay exposes the lock.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli tui::screens::misc::tests::l0_render_admin_labels_shift_policy_lock -- --nocapture`
- `git diff --check`
