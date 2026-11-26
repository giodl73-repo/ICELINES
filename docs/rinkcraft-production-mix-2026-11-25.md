# Point composition in Playing Tendencies — 2026-11-25

The player card now shows a compact **Point mix** beside penalty balance and the
existing feat selection. Expand it for goal/assist rates, their shares of all
points, and matched primary/secondary assist detail. Contract:
[Production mix v1](../design/specs/rinkcraft-production-mix-v1.md).

## Frozen examples

| Player | Goals as share of points |
|---|---:|
| Adam Fox | 7.7% |
| Connor McDavid | 22.4% |
| Nathan MacKinnon | 33.3% |
| Pavel Dorofeyev | 50.0% |
| Matty Beniers | 53.8% |

These describe the selected 2026–27 NHL production snapshot, not permanent
finisher/playmaker identities. Position, deployment, teammates and opportunity
affect the results. Of 787 current skaters, 694 have points; 93 explicitly show
**No recorded points** instead of a percentage. Goalies receive no skater record.
Existing class assignments, six-attribute grades, feats and simulation rules stay
unchanged. All calculations and record admission are shared Rust.

## Validation

- 254 browser tests and 17 focused Rust tests passed, covering the production
  record, penalty balance and stat-input admission. A subsequent focused browser
  run also passed after updating the representative headline wording.
- Browser, pinned unoptimized WASM and native CLI/Askama builds passed.
- All 865 local preview profiles project and reproject through the shared engine.
  Serialized production records roundtrip exactly. Earlier grades, achievements,
  shot methods, pressure and Unit Links remain intact.
- Independent arithmetic checks all 787 derived records against the original
  context rates, including no-point states and matched assist-detail admission.
- Eight live cards pass exact strings and assist-breakdown rows, actual WASM
  worker parity, unchanged grades, goalie withholding, older-card fallback and
  390-pixel mobile layout. Fox's expanded point mix was visually inspected.
- Eight independently checked PDFs remain single-page 612×792-point Letter
  sheets; Attributes and Playing Tendencies remain, while the disclosure is hidden.

Normal refresh already supplies the required rates and assist credits; no extra
fetch or import stage is needed. The original catalog remains unchanged:
`target/shot-methods-study/enriched/rinkcraft-stat-attributes.json`, SHA-256
`93975bc201a7692c375b43c748a0ab5d76af569c2e57bda359552504c50a2f95`.
Generated projections, audit script, screenshots and exports remain ignored in
`target/production-mix-study`.

```powershell
node icelines-browser/e2e/rinkcraft-production-mix.mjs
py target/production-mix-study/verify.py
```

Preview: [Fox](http://127.0.0.1:8780/?player=8479323), **Playing Tendencies →
Point mix**. No new source capture, hosted publication, push or merge is included.
