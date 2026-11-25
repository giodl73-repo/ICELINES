# Shooting record — 2026-11-24

The Shooting field adds two short lines: NHL shots on goal per 60 and NHL shot
conversion. Its detail view separates generation, observed finishing, modeled
shot danger and maximum tracked puck speed. The existing Shooting grade remains
unchanged, as do all other grades, feats, classes and simulation inputs.

## Measurement

The shared Rust `profile-records` attachment produces an optional
`rinkcraft.shooting-record.v1` record for the selected season. Native Askama and
browser cards consume the same preformatted values. Older catalogs retain their
existing generic Shooting details.

The eleven rows show NHL goals/60, shots/60 and conversion; MoneyPuck.com
goalie-in-net unblocked attempts, goals, expected goals, goals minus expectation,
xG per attempt and average attempt distance; EDGE maximum tracked shot speed;
and official GP/minutes.

- NHL rates = official counts × 60 / official all-strength minutes.
- NHL conversion = goals/60 ÷ shots/60 × 100. Zero goals with positive shots give
  0%; zero shots or missing exposure give unavailable conversion.
- Archive goals minus expectation = archive goals − archive xG. These must come
  from the same provider sample. No NHL totals or minutes enter this calculation.
- Archive average danger includes missed unblocked attempts. It is not a shot
  conversion percentage. No sampled attempts means no average or residual.

Archive and official snapshots can differ. Capture notes remain visible in the
detail view. A positive residual does not establish persistent finishing talent;
chance environment, model error and sampling variation remain. A recorded
maximum shot speed does not establish accuracy or release quickness.

## Frozen inputs and reproduction

Input: `target/passing-card-study/enriched/rinkcraft-stat-attributes.json`.
SHA-256: `166b9ee2d5809f5e0ab8badf819c1a9ca8ad53fa9e4374249bc56a868247fe24`.
The 2026–27 NHL snapshot was fetched 2026-11-23. Existing admitted observations
are reused; no new external data fetch is needed for this presentation.

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/passing-card-study/enriched/rinkcraft-stat-attributes.json --output-dir target/shooting-card-study/enriched
```

Always choose a fresh output directory. Generated inputs, projections, PDFs and
screenshots stay ignored under `target/shooting-card-study`. Sources and coverage
retain their original windows; absence does not become zero.

## Local results

All 787 skaters receive records with official goal and shot rates. Archive
observations are present for 776; EDGE maximum shot speed is present for 781.
All 3,948 retained profile score arrays are unchanged. Catalog and report replay
are byte-identical. Output catalog SHA-256:
`9fc4ee1625e407c88592bffe80c11bc0b6cc566105d282d4ae080cb86a574e77`.

| Player | Shots on goal /60 | NHL conversion | Archive goals minus xG |
|---|---:|---:|---:|
| Nathan MacKinnon | 10.54 | 14.1% | +2.58 |
| Sebastian Aho | 8.14 | 20.3% | +2.78 |
| Adam Fox | 4.10 | 5.6% | −0.77 |

Native CLI (including Askama templates), WASM and browser builds passed. All
245 browser unit tests and 14 focused Rust record tests passed. Four actual local cards (MacKinnon, Aho, Fox,
Killorn) passed summary/detail values, unchanged grades, prior-record retention,
actual WASM worker parity, older-catalog fallback, mobile overflow and absence of
page errors. Four one-page 612×792-point Letter PDFs independently passed pypdf
checks for both summary values. Desktop and detail screenshots were inspected.

No hosted build, publication, push or merge is part of this change. Preview:
<http://127.0.0.1:8780/?player=8477492>; click Shooting for the breakdown.
