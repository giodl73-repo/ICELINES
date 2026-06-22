# Phase Canadiens Source - GSAx catalog source block

Status: Closed

## Intent

Keep the reserved goalie GSAx catalog family visible and stable without
promoting unsupported values before a verified goalie xGA source exists.

## Scope

- Pin the reserved catalog keys for goalie xGA, goalie xGA/60, GSAx, and
  GSAx/60.
- Keep those IDs source-blocked: `read()` returns `None` and the report overlay
  does not claim `/goalie/advanced` or `/goalie/savesByStrength` as their
  source.
- Document that promotion requires schema fixture evidence, goalie identity
  join evidence, freshness/source-state metadata, and explicit non-claim copy.
- Preserve QS%/SA/60 and skater on-ice xGA as non-substitutes for goalie xGA.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core stats_catalog::tests::l0_goalie_gsax_catalog_keys_remain_source_blocked`
- `git diff --check`
