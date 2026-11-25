# Passing record — 2026-11-24

Passing now has a compact primary/secondary assist-rate summary and an eight-row
detail record on the native and browser player sheets. This adds scoring-chain
context to the existing assist-production estimate; it does not change its grade.

## Measurement

Rust produces the record through the shared `profile-records` attachment path:

- All-strength primary/secondary rate = credited assist count × 60 / official
  all-strength minutes.
- Primary share = primary / (primary + secondary). Zero credited assists show
  “No credited assists”; missing split data show “Assist mix unavailable”.
- EV and PP assist rates retain their own strength-specific denominators. EV is
  not a five-on-five-only label. No usable PP time means an unavailable rate.
- The record also shows all-assist rate, PP time share, GP and minutes.

These credits do not measure pass attempts, completion, touches or every chance
created. Secondary credits remain useful involvement evidence. The split does
not establish a hierarchy of passing skill. PP totals cannot establish unit,
station or coaching assignment. No feat, class, capability grade or simulation
bonus changes. Optional records preserve older-catalog rendering.

## Frozen local result

2026–27 snapshot, NHL data fetched 2026-11-23. Existing admitted inputs are reused;
no additional external data were fetched for this change. The prior collection
reconciled all 787 assist splits against NHL goal logs from 377 games.

All 787 skaters have primary/secondary records and EV rates; 703 have PP rates.
All 3,948 catalog profile score arrays are unchanged across all retained seasons.

| Player | Primary /60 | Secondary /60 | Primary share | EV assists /60 EV | PP assists /60 PP |
|---|---:|---:|---:|---:|---:|
| Nathan MacKinnon | 1.93 | 1.04 | 65.0% | 2.82 | 3.59 |
| Sebastian Aho | 0.89 | 1.65 | 35.0% | 2.29 | 3.29 |
| Adam Fox | 1.71 | 1.03 | 62.5% | 1.91 | 5.70 |

MacKinnon and Aho each have 20 assists, split 13/7 and 7/13 respectively.
Their different official minutes also produce different total assist rates.

## Reproduction

Run from the IceLines worktree after building the CLI:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/reading-card-study/enriched/rinkcraft-stat-attributes.json --output-dir target/passing-card-study/enriched
```

Use a fresh output directory for replay. The enriched and replay catalogs and
reports were independently checked byte-for-byte identical.

Input SHA-256: `09896c89e3fe530f7b0da68c5da2bc154b5f7b7edb53bf105fcce2e4f52e39d0`.
Output SHA-256: `166b9ee2d5809f5e0ab8badf819c1a9ca8ad53fa9e4374249bc56a868247fe24`.
Generated catalogs, fixtures, PDFs and screenshots remain ignored under `target`.

## Validation

- Native CLI build including Askama templates, WASM build and browser build passed.
- Eleven focused core tests and three refresh tests passed; browser suite: 245 passed.
- Four live local cards: MacKinnon, Aho, Fox and Killorn. Summary/detail values,
  unchanged grades, prior records, actual WASM worker parity, older-catalog
  fallback, 390-pixel mobile overflow and no page errors passed.
- Four Letter PDFs independently checked with pypdf: one 612×792-point page each,
  with both primary and secondary rates present. Print spacing was tightened to
  retain the footer on the page. Desktop and detail screenshots were inspected.
- Raw cross-runtime minutes allow 1e-9 tolerance; record strings and integer
  grades match exactly. No hosted build, publication, push or merge was performed.

Local preview: <http://127.0.0.1:8780/?player=8477492>. Click Passing for details.
