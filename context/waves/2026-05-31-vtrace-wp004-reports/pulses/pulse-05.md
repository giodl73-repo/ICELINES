# WP-004 Pulse 05 - Duplicate and Unicode Name Export Evidence

## Scope

Add focused report/export evidence for ambiguous, duplicate, and Unicode player
names in Markdown leaders exports.

## Change

- `icelines-cli/src/commands/export.rs` now has a fixture test with two players
  sharing the same display name and one accented Unicode display name.
- The fixture proves duplicate display names remain separate rendered rows.
- The fixture proves Unicode names survive Markdown rendering without being
  normalized away or collapsed into ASCII-only output.

## Evidence

```powershell
cargo test -p icelines-cli l0_export_leaders_preserves_unicode_and_duplicate_names_as_rows --quiet
```

## Result

`passed_with_risk`: selected Markdown leaders exports now have focused evidence
for duplicate display names and Unicode display names. Lockout, October rollover,
trade continuity, and full report/export matrix coverage remain open for later
WP-004 pulses.

