# WP-004 Pulse 03 - Completeness Disclosure Guardrail

## Scope

Make Markdown export disclosure explicit about incomplete, partial, stale,
missing, and skeleton source states so public report readers treat them as
evidence limits instead of zero-value truth.

## Change

- `icelines-cli/src/commands/export.rs` now adds a `Completeness` disclosure line
  to the near-top Markdown export disclosure section.
- The disclosure directs readers to source, warning, and empty-state sections
  before using rendered rows.
- Focused export tests assert the new completeness language remains before report
  tables and does not weaken the existing unsupported-claim limitation wording.

## Evidence

```powershell
cargo test -p icelines-cli l0_export_leaders_discloses_methodology_limits_near_top --quiet
cargo fmt --check
```

## Result

`passed_with_risk`: selected Markdown exports now disclose completeness and
skeleton-state limits near the top of public artifacts. Lockout, October
rollover, ambiguous/Unicode/duplicate names, trade continuity, GP thresholds, and
full report/export matrix coverage remain open for later WP-004 pulses.
