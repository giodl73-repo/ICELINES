# WP-004 Pulse 01 - Public Markdown Export Disclosure

## Scope

Add a public-copy guardrail to Markdown exports so generated artifacts disclose
their data scope and explicitly avoid unsupported era-adjusted, predictive,
betting, injury, special-teams, deployment, or linemate-analysis meaning before
the shared report content.

## Change

- `icelines-cli/src/commands/export.rs` now writes a `## Disclosure` section
  immediately after YAML front matter for Markdown exports.
- The disclosure states that reports summarize available roster, schedule,
  boxscore, play-by-play, and source-state data exactly as labeled.
- The disclosure states that the artifact is not era-adjusted, predictive,
  betting, injury, special-teams, deployment, or linemate analysis.
- Focused leaders export tests assert the disclosure appears near the top before
  the table and before the existing context section.

## Evidence

```powershell
cargo test -p icelines-cli l0_export_leaders -- --nocapture
cargo fmt --check
cargo clippy -p icelines-cli --no-deps -- -D warnings
```

## Result

`passed_with_risk`: selected Markdown leaders exports now carry a public-copy
methodology limitation guardrail. Historical edge fixtures for lockout,
rollover, ambiguous/Unicode/duplicate names, trade continuity, GP thresholds,
active streaks, and skeleton/completeness disclosure remain open for later
WP-004 pulses.
