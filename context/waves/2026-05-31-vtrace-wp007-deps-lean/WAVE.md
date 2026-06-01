# VTRACE WP-007 Dependency/Lean-Build Wave

## Scope

Work package: `WP-007` - dependency seams and lean CLI target.

Primary requirements: `REQ-DEP-001`, `REQ-LEAN-001`, and `REQ-CODE-001`.

Primary validation scenario: `VAL-009`.

## Objective

Inventory every FLETCH/SLICE dependency seam and command surface before Cargo
surgery. Do not remove a dependency, command, selector, or feature path until the
affected surface has a replacement, explicit refusal, compatibility shim, or
rollback plan.

## Pulse Log

| Pulse | Scope | Evidence | Status |
|---|---|---|---|
| 01 | Dependency and lean-feature inventory/disposition | `cargo tree -i fletch-core`; exact `cargo tree -i ...slice-core...`; `cargo build --no-default-features --features cli`; `EVID-WP007-DEP-INVENTORY-L0`; `CHG-070` | target-not-met_dispositioned |

## Current Status

`WP-007` is `target-not-met_dispositioned`. Pulse 01 confirms the current
workspace cannot claim standalone/lean support:

- `fletch-core` is a path dependency from `../FLETCH/crates/fletch-core`, pulled
  by `icelines-fetch` and transitively by CLI, Web, and site surfaces.
- `slice-core` is a direct git dependency of `icelines-query` at rev
  `353564781f6cad53fc5a934178a7927824824e3e`.
- A second `slice-core` rev, `50b63a2eefc66916e9a015a915c845c28d80773c`, is
  pulled transitively through `fletch-core`.
- The expected lean command currently fails before compilation because no
  selected package exposes a `cli` feature.
- Affected command surfaces include `fetch fletch-sources`,
  `fetch fletch-partitions`, `fetch fletch-quivers`, and
  `fetch fletch-cache-index`; affected selector code is
  `icelines-query/src/slice_selectors.rs`.

## Remaining Work

- A future dependency surgery wave must choose replacement/refusal/shim/rollback
  per affected FLETCH command and SLICE selector.
- A future feature-boundary wave must define package features for `cli`, `tui`,
  `web`, `net`, and `reports` before any lean build claim.
- WP-008 may proceed with `VAL-009` explicitly marked target-not-met and owned by
  the maintainer/release lens.

