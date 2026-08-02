# IceLines Sources S0 — Inventory and Compatibility Baseline

**Date:** 2026-07-31
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete

## Frozen artifacts

- [`../data/icelines-fetch-module-inventory.v1.json`](../data/icelines-fetch-module-inventory.v1.json)
  classifies every public `icelines-fetch` module and records the initial
  migration disposition, compatibility schemas, commands, persistence keys,
  and representative fixture hashes.
- [`../data/prospect-ranking-depth-baseline-2026-07-31.v1.json`](../data/prospect-ranking-depth-baseline-2026-07-31.v1.json)
  freezes the current requested top-ten depth result.
- `icelines-fetch/tests/source_module_inventory.rs` mechanically checks module
  coverage, migration vocabulary, fixture hashes, the season-canonical team
  set, ranking-depth arithmetic, and the 23/9/17 summary.

## Responsibility inventory

The original S0 baseline exposed 70 public modules. The mechanically checked
inventory now exposes 78 after the S3-S6 source-package and identity modules
were added. An S7 full-gate run caught and fixed the eight-row inventory lag;
the inventory also records nineteen completed whole-module or responsibility
splits separately from each module's planning disposition. Responsibility counts overlap
because mixed ownership is the condition this architecture is correcting.

| Responsibility | Modules |
|---|---:|
| Feature/domain composition | 46 |
| Source normalization/reconciliation | 36 |
| Cache/snapshot/persistence | 22 |
| Provider DTO/parser | 19 |
| Acquisition/transport | 17 |
| UI/command orchestration | 2 |

Current planning dispositions:

| Disposition | Modules |
|---|---:|
| Domain extraction deferred | 24 |
| Split source from domain | 18 |
| Split transport from parser | 15 |
| Stay in fetch | 15 |
| Move pure parser/normalizer to sources | 6 |

These are planning classifications, not authority to move a module wholesale.
Every split still requires its own compatibility fixture and build-green cut.

## Ranking-depth baseline

At requested depth ten, the frozen artifact reproduces:

- 23 organizations with ten eligible studies;
- nine partial organizations;
- 17 total missing ranking slots;
- FLA at six; BOS, EDM, LAK, NJD, and NYI at eight; and BUF, COL, and WPG at
  nine.

This is deliberately named a ranking-depth baseline. It does not assert a
complete prospect census. That stronger claim remains gated by the future
population-authority source matrix specified for S5/S6.

## Verification

```text
cargo test -p icelines-fetch --test source_module_inventory
3 passed; 0 failed
```

The next slice is S1: add the `icelines-sources` crate and extract only the pure
player-landing parser while preserving the existing output type and
`icelines_fetch::career_landing` facade.
