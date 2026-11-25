# Penalty balance in Playing Tendencies — 2026-11-25

The compact card now has an expandable penalty balance beside its existing
Playing Tendencies. This makes both drawing calls and taking calls visible
without replacing earned feats or changing any ability estimate. Contract:
[Penalty balance v1](../design/specs/rinkcraft-penalty-balance-v1.md).

## Frozen results

All 787 current skaters have admitted all-strength inputs. At two-decimal display
precision, 291 have positive drawn-minus-taken rates, 345 negative, and 151 even.
The 78 goalies retain their separate semantics and receive no skater record.

| Player | Drawn /60 | Taken /60 | Balance /60 |
|---|---:|---:|---:|
| Nathan MacKinnon | 1.63 | 0.45 | +1.19 |
| Matty Beniers | 0.33 | 0.00 | +0.33 |
| Adam Fox | 0.34 | 0.68 | −0.34 |
| Sebastian Aho | 1.02 | 1.02 | +0.00 |

Balance uses unrounded rates, so rounded component displays can differ by 0.01
from the displayed balance. These are calls, not minutes or power plays created.
The window is the selected 2026–27 NHL snapshot already in the catalog, not a
new live capture. There is no ten-GP gate or new classification rule.

## Implementation and validation

Rust derives the optional record from the existing selected-season context during
profile admission; native Askama and browser cards consume the same values. Normal
refresh supplies these existing inputs without another fetch or import stage.
Older catalogs omit the field and remain valid. Missing component rates prevent
a balance; measured zero remains zero. Actor, season, group, GP and minutes are
checked, and catalog validation rejects stale or altered records.

- 253 browser tests and 24 focused Rust tests passed: penalty balance, stat-input
  admission, shot methods and skating detail. Tests include pre-rounding math,
  negative/zero/partial observations, scope and exposure, small samples, stale
  records, optional serialization and preserved existing measurements.
- Browser, pinned unoptimized WASM and native CLI/Askama builds passed.
- All 865 preview profiles project and reproject through the shared WASM engine.
  Repertoire records, prior six-grade histories, feats, pressure and Unit Links
  are retained. Independent arithmetic matches all 787 penalty-balance records.
- Six live cards passed exact values, actual worker parity, unchanged grades,
  goalie withholding, older-card fallback and 390-pixel mobile overflow checks.
  Fox's expanded disclosure was visually inspected.
- Six independently checked PDFs are single-page 612×792-point Letter sheets,
  retaining Attributes and Playing Tendencies while hiding the new disclosure.

The input catalog remains byte-identical:
`target/shot-methods-study/enriched/rinkcraft-stat-attributes.json`, SHA-256
`93975bc201a7692c375b43c748a0ab5d76af569c2e57bda359552504c50a2f95`.
Generated projections, independent recount, screenshots, PDFs and acceptance
results remain ignored under `target/penalty-balance-study`.

```powershell
node icelines-browser/e2e/rinkcraft-penalty-balance.mjs
py target/penalty-balance-study/verify.py
```

Preview: [Fox](http://127.0.0.1:8780/?player=8479323), **Playing Tendencies →
Penalty balance**. No source refresh, hosted publication, push or merge is
included in this step.
