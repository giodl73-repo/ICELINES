# Pulse 04 - Surface Parity and Docs

## Goal

Wire the stable Sim the Spark ViewModels to discoverable, cache-safe user
surfaces and documentation. Web/API may lead; CLI/TUI should only be added if
they can render the same ViewModel fields without new command-local math.

## Governing roles

- **pace**: surface copy must preserve the exact formulas and nullable
  semantics from `PlayerScoringPaceView` and `TeamScoringOutlookView`.
- **scout**: labels must stay descriptive: "on pace", "tracking toward",
  "recent pressure", "below sample floor", and "partial source".
- **wire**: GET routes must not fetch live NHL data or mutate caches. Any missing
  data state must be visible in JSON and HTML instead of silently defaulting.
- **bench**: add parity tests for JSON/source-state contracts and at least one
  missing-source case.

## Owned scope

1. Wire player scoring pace and team scoring outlook ViewModels to web/API routes
   only if inputs are already loaded or cache-derived.
2. Keep route handlers thin: no formula math outside the core ViewModels.
3. Update `COMMANDS.md`, `README.md`, and relevant route/help docs if new
   discoverable surfaces are added.
4. Add L1/L2 tests appropriate to any new routes or commands.
5. Preserve explicit source/completeness fields in JSON.

## Non-goals

- No live network fetch from GET.
- No betting odds, win probability, proprietary xG, or playoff odds.
- No new projection model beyond the Pulse 02/03 descriptive pace contracts.
- No TUI/CLI work unless it can be done by rendering existing ViewModel fields.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-site --quiet`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo clippy -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check design\waves\2026-05-15-sim-the-spark README.md COMMANDS.md --errors-only`
