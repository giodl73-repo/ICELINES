# Reading records — 2026-11-24

Read & React now shows recorded takeaways and penalties taken near the front
attribute rail, with **anticipation unmeasured** visible beside the rates. The
Reading attribute detail adds assist involvement, primary/secondary credits,
exposure and a readable formula. Native Askama and browser rendering consume the
same optional Rust `reading-record.v1` view. No grade, feat, documentary class or
simulation bonus changes.

## Interpretation

The existing Reading grade is 50% favorable takeaways/60 rank, 30% assists/60 rank
and 20% lower penalties/60 rank, using its existing same-season position peers.
It measures a mixture of event production and discipline proxies. Passing also
uses assists, so these two grades share input evidence; they are not independent
observations of different capabilities. Shared inputs are not automatically wrong,
but should remain visible when reasoning about profiles and future simulation.

The new record separates those observations without inventing anticipation,
positioning, opponent recognition or assigned coverage. Credited takeaways do
not enumerate every recovery. Assist credits do not enumerate every pass. Calls
alone do not establish decision quality. Raw rates remain available below the
existing peer-ranking sample threshold; no new GP cutoff is introduced.

All rates use the selected NHL all-strength window. Secondary assists are
converted from admitted counts using the same official minutes; primary rates
are already present in that window's admitted context. In the acquisition path,
assist-type details are admitted only after total credits reconcile with NHL.
Missing detail remains unavailable, even when total assists are present. Zero
events remain zero; no exposure means no rate. Display rounding can make split
rates differ slightly from the displayed total.

## Current records

The retained 2026–27 NHL snapshot was fetched 2026-11-23. All 787 current skaters
have takeaway, assist and penalties-taken rates. Examples, per60:

| Player | Takeaways | Assists | Penalties taken | Primary assists | Secondary assists |
| --- | ---: | ---: | ---: | ---: | ---: |
| McDavid | 0.88 | 4.38 | 0.39 | 3.21 | 1.17 |
| Fox | 1.14 | 2.74 | 0.68 | 1.71 | 1.03 |
| Beniers | 1.51 | 1.00 | 0.00 | 0.50 | 0.50 |

These distinguish recorded patterns; they do not establish an ordering of those
players' anticipation or hockey IQ. Role, puck use and deployment remain relevant.

## Reproduction and coverage

The generic record builder and normal refresh path now include Reading. The
offline CLI rebuilds it from already admitted measurements without fetching:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/handling-card-study/enriched/rinkcraft-stat-attributes.json --output-dir target/reading-card-study/enriched
```

Choose a new output directory for replay. Record metadata is bound to player,
season, GP, group and all-strength minutes. Other seasons are preserved. Older
catalogs without the optional record keep the existing anticipation fallback.

- Input SHA-256: `f551c58eeddb6ca6960e3629d5b61d8c1c749d8e2ab85267f9df0be24066eba3`
- Output SHA-256: `09896c89e3fe530f7b0da68c5da2bc154b5f7b7edb53bf105fcce2e4f52e39d0`

All 3,948 supplied six-integer-grade arrays remain unchanged. Catalog and report
replay byte-identically. The complete preview profiles receive their records
through actual WASM attachment, preserving all other profile fields. Text agrees
exactly; raw exposure is checked within 1e-9 across JSON runtimes.

## Validation

- Eleven focused Rust tests pass: Reading, Handling and defensive records plus
  refresh/cache replay. New checks cover the assist split, zero/missing detail,
  absent exposure, season isolation, window binding and unchanged grades.
- Locked native CLI build passes, including the Askama views. Rust 1.95.0 WASM
  and matching wasm-bindgen 0.2.118 bindings pass.
- Full browser build and 245 unit tests pass.
- Four-player acceptance passes McDavid, Fox, Beniers and Killorn: front rates,
  six-row detail, uncertainty label, retained prior records, actual browser-worker
  attachment, legacy fallback, no page errors and 390px overflow checks.
- Four PDFs independently contain the exact summary rates and anticipation label
  on one 612×792-point Letter page each. Desktop/detail screenshots were inspected.
- Visual review found dark-app text colors leaking into light stat dialogs. Scoped
  text colors and legible recipe typography fix this; acceptance now checks at
  least 4.5:1 contrast for the dialog heading and explanatory paragraphs.

Preview: `http://127.0.0.1:8780/?player=8482665`. Select Reading for the detail.
This is a local generated preview, not a hosted release or full native-runtime
acceptance claim.
