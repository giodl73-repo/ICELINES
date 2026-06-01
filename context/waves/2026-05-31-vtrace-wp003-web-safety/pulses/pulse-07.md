# Pulse 07: Serve Launch Safety and WP-003 Closeout

## Scope

`WP-003` selected Web/browser safety slice for `icelines serve` launch semantics
and package closeout.

The observed gap was that launch safety was described in the serve command but
the URL-before-open, `--no-open`, and LAN bind warning behavior was not fenced by
focused unit evidence.

## Change

- `icelines serve` now builds its launch output through an internal
  `ServeLaunchPlan`, making URL printing, browser-open gating, and LAN warnings
  explicit and testable before any browser-open side effect.
- Existing runtime behavior is preserved: the URL is printed before auto-open,
  `--no-open` skips auto-open, browser-open failure remains non-fatal, bind
  errors remain loud, and non-localhost binds emit the LAN warning.
- Focused serve tests assert URL output ordering, `--no-open` gating, LAN warning
  text, and existing bind resolution behavior.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-cli commands::serve -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-cli --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for `WP-003`.

Pulses 01-07 prove selected GET-read-only, no-JS shell, viewport/recovery, and
serve launch/host-warning boundaries for `VAL-003`. Residual risks are accepted
for this package: no live browser screenshot/review was captured, touch/focus
interaction remains route/template-level rather than browser-level evidence, and
broader JSON-twin inventory remains covered by selected route tests rather than a
full route matrix. Revisit in `WP-008` before readiness claims.
