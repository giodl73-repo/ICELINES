# IceLines Sources S8 — Reuse and Release-Gate Closeout

**Date:** 2026-08-01
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete; S6 authority inputs remain independently pending

## Reuse and documentation

- Fantasy Yahoo eligibility is the required non-prospect consumer of the same
  deterministic source boundary; file I/O remains in fetch and provider parsing
  is reusable from caller-supplied bytes.
- The inventory records 19 completed whole-module or responsibility splits and
  78 classified fetch modules. The closing audit separates transport,
  persistence, review, and feature-domain composition from provider parsing.
- Architecture, commands, plan/spec indexes, source-family contracts,
  compatibility paths, per-family migration notes, surface parity, and this
  changelog now describe the implemented boundary.
- CLI, TUI, Web, cards, fantasy, simulation, and Window consumers continue to
  receive core/ViewModel or fetch-facade types; no renderer parses provider
  payloads.

## Full validation

The all-target workspace gate was split only to isolate failures; the two
commands cover the same workspace target set:

```text
cargo test -p icelines-cli --all-targets
passed

cargo test --workspace --all-targets --exclude icelines-cli
passed

cargo fmt --all -- --check
passed

cargo clippy --workspace --all-targets -- -D warnings
passed

powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
passed against target/release/icelines.exe
```

Targeted architecture, inventory, AHL, official-identity, career-landing,
goalie parity, and 80-column visual tests also pass. The release smoke verifies
offline version/help, leaders, goalies, TUI/serve help, docs, Markdown export,
poach, The Window, and server URL startup.

## Bugs fixed during the gate

- All test-only Clap parsing now uses the same explicit 16 MiB parser stack as
  production, preventing Windows test-harness stack overflow as commands grow.
- The `--explain` no-load contract now asserts that an isolated HOME remains
  untouched instead of using a flaky wall-clock threshold under parallel load.
- The goalie text leaderboard retains every metric while fitting in 75 columns,
  restoring the documented 80-column no-color contract.

## Remaining authority gate

S6 is not waived by architectural completion. Strict all-32 prospect
publication still requires real, authorized, independently reviewed identity,
contract-control, and camp-participation ledgers. IceLines continues to publish
the deterministic official candidate board and typed blockers rather than
inventing reviewer names, registry URLs, or control facts.
