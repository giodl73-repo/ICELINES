# IceLines Sources S5 — Prospect Census Progress

**Date:** 2026-07-31
**Status:** In progress

## First slice

- `ProspectCensusView` is a UI-neutral `prospect_census.v1` contract in core.
  It carries package fingerprint, effective and knowledge cutoffs,
  reconciliation and eligibility policy versions, organization rows, league
  totals, typed losses, disclosures, and source/player/position dimensions.
- Each discovered candidate advances monotonically through canonical identity,
  controlled relationship, prospect eligibility, usable career evidence,
  built study, and ranked output. A candidate that stops must name the exact
  blocked transition, typed reason, and explanatory message.
- Population authority and ranking depth are separate gates. A team can have
  enough ranked studies while population authority is incomplete, or complete
  source authority while ranking depth is short. In either case numeric score
  and ordinal rank states are explicitly withheld.
- The strict publication guard refuses a league artifact if any requested
  organization remains blocked. The audit view itself remains serializable so
  missing-source and stage-loss diagnostics are never destroyed by the gate.
- Dimension rows partition candidates by organization, discovery source
  family, player class, and skater/goalie position group. League totals are
  derived from organization counts rather than recomputed by a separate path.
- The fetch-owned composer validates a sealed package, lowers reviewed staged
  assertions through S4, derives authority from each organization's exact
  manifest outcomes, joins optional versioned eligibility/career/study/rank
  evidence, and emits the core census without provider DTOs escaping fetch.
- `icecast prospect-census --source-package PATH [--pipeline PATH]
  [--require-publishable] [--json]` is implemented. Audit output remains
  writable when incomplete; strict mode checks before output construction and
  does not create the requested file on refusal.
- A real 2026-27 all-32 run produced 1,053 discoveries: 807 canonical current
  NHL assignment identities and 246 unresolved draft/trade identities. The
  current catalog establishes no contract/control facts, so all 807 canonical
  rows stop at `unsupported_control`, all 246 staged rows stop at
  `unresolved_identity`, and every organization remains population-incomplete.
  NYR exposes 39 discoveries and SEA 30. This is a causal diagnostic, not a
  zero-valued prospect ranking.
- Existing discovery and program artifacts now compose directly into
  `prospect_census_pipeline.v1`; a hand-authored evidence bridge is optional,
  not required.

## Remaining S5 work

1. Replay the current nine partial organizations and prove their shortfalls are
   causal stage losses while Rangers and Kraken remain stable unless new facts
   expand their populations.
2. Add complete source-family/player-class/skater-goalie reconciliation tests
   over the real all-32 sealed reference path.

## Validation

```text
cargo test -p icelines-core --lib view_model::prospect_census::tests -- --test-threads=1
cargo clippy -p icelines-core --lib -- -D warnings
```
