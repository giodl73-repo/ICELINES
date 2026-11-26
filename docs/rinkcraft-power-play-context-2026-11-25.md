# Power-play context in Role Coverage — 2026-11-25

Role Coverage now expands PP workload and assist composition separately. The
summary shows PP share of assists when supported, otherwise PP share of own TOI.
Four rows expose PP time share, PP assist share, assists/60 PP, and assists/60
outside PP. Contract: [Power-play context v1](../design/specs/rinkcraft-power-play-context-v1.md).

## Frozen examples

| Player | PP share of own TOI | PP share of assists | Assists /60 PP | Assists /60 outside PP |
|---|---:|---:|---:|---:|
| Adam Fox | 16.0% | 33.3% | 5.70 | 2.17 |
| Connor McDavid | 16.7% | 53.3% | 13.96 | 2.46 |
| Nathan MacKinnon | 20.7% | 25.0% | 3.59 | 2.81 |
| Pavel Dorofeyev | 16.5% | 66.7% | 4.13 | 0.41 |
| Matty Beniers | 14.5% | 50.0% | 3.46 | 0.59 |

These describe the existing 2026–27 NHL snapshot. Outside PP includes all remaining
situations, not only five-on-five. Rates have different exposure denominators;
PP share of assists multiplies the PP rate by PP time share before dividing by
the all-strength assist rate. This view does not establish a unit, station,
coaching assignment, passing success or isolated impact.

All 787 current skaters have time-share context: 631 support an assist share,
72 have matched zero assists, and 84 have no recorded PP time. Missing PP rate
with zero PP time does not become a credited count or a composition percentage.
The 78 goalies retain their own workload semantics. No ten-GP gate is added.

## Implementation and validation

Rust derives and validates the optional record from existing admitted context;
native Askama and TypeScript render it. Normal refresh already supplies the inputs
and needs no new fetch or enrichment step. Older serialized cards can omit it.
No grade recipe, feat, class or simulation rule changes.

- 256 browser tests and 20 focused Rust tests passed: PP context, production mix,
  penalty balance and stat-input admission. Cases include different strength
  denominators, no PP time, no outside-PP time, zero assists, absent/inconsistent
  components, invalid scope/exposure, small samples and stale/altered records.
- Browser, pinned unoptimized WASM and native CLI/Askama builds passed.
- All 865 local profiles project and reproject through the shared engine; new
  records roundtrip exactly. Earlier grade histories, achievements, pressure,
  unit links, shot methods, penalty and point composition remain intact.
- Independent arithmetic matches all 787 records against their original context
  values and exposure. The input catalog remains byte-identical.
- Eight live cards passed exact values and Calculation disclosure, actual worker
  parity, unchanged grades, goalie withholding, older-card fallback and 390-pixel
  mobile layout. Fox's expanded PP view was visually inspected.
- Eight independently checked PDFs are single-page 612×792-point Letter sheets,
  retaining Attributes and Role Coverage while hiding supplemental PP details.

Input: `target/draw-strength-study/enriched/rinkcraft-stat-attributes.json`.
SHA-256: `22c98248e403016b7e9a4e34e140bbb03ff37e2b9d3beb4a480a9744c7500315`.
Generated projections, audit script, screenshots, PDFs and acceptance results
remain ignored under `target/power-play-context-study`.

```powershell
node icelines-browser/e2e/rinkcraft-power-play-context.mjs
py target/power-play-context-study/verify.py
```

Preview: [Fox](http://127.0.0.1:8780/?player=8479323), **Role Coverage →
Power play**. No source refresh, hosted publication, push or merge is included.
