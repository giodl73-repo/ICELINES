# Observed party relationships — 2026-11-25

Deployment and Unit Links now populate from observed regulation combinations.
Source, scope and interpretation: [Unit Links v1](../design/specs/rinkcraft-unit-links-v1.md).

## Frozen population

The existing 248 native NHL shift captures through November 11, 2026, produce
829 player records: 495 forwards, 262 defensemen and 72 goalies. Thirty-six current
cards lack a matched record and retain their placeholders. Fifty HTML-derived
games from the earlier 298-game study remain outside this native-only import.
No new statistical source was fetched and no PUCK source/output was modified.

Fox's leading observed 5v5 pair is with Gavrikov: 233:50 shared, or 84.0% of Fox's
verified 5v5 time across 17 matched appearances. His leading observed 5v4 group
accounts for 31:37 / 48.2% of his verified 5v4 time. The rest is not automatically
PP2: there are alternative personnel combinations, not inferred numbered units.
Forwards show a joint trio; goalies show the defense pair in front while they play.

240 records have a different unique leading 5v5 combination in the last five
matched appearances compared with the full matched sample. Those windows overlap;
this is descriptive composition, not a held-out change detector or a coach label.
The frozen population has no tied primary 5v5 leaders, but neutral fixtures verify
explicit co-leading behavior and withholding a unique recent/full comparison.

## Implementation

- Rust reuses paired-source admission and complete participation segments. Full
  game TOI verification now exposes eligible goalies as well as skaters; the
  earlier skater pressure sample and its interpretation remain separate.
- A unit segment requires verified active participants on both sides. Conflicts,
  invalid occupancy and unverified participants leave unresolved time instead
  of increasing another group's share. Team identity survives trades.
- The optional stat-profile field carries compact top-two context groups, three
  frequent partners and a last-five matched window. Total observed group counts
  remain available, and raw sources reproduce combinations outside the card cap.
- Native Askama and browser render the same Rust totals, names and percentages.
  Dates/denominators are visible; Assigned Role stays independent. The printable
  form keeps the leading 5v5 relation and matched manpower totals on one Letter page.
- Existing refresh rebinds records only with the same admitted parent fingerprint.
  Unselected goalie records survive skater-only refresh. Changed/unavailable parents
  withhold links; explicit import supplies refreshed observations.

## Reproduce

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-unit-links --season 20262027 --catalog target/pressure-exposure-study/enriched/rinkcraft-stat-attributes.json --game-cache-dir target/goalie-puck-study/cache --shift-manifest target/pressure-exposure-study/inputs.json --output-dir target/unit-links-study/enriched
```

Choose a fresh output directory. The explicit paired manifest and its original
sealed source lineage are retained from the exposure study. Output catalogs,
unit report, paired-source receipts, independent audit and rendered artifacts
remain ignored under `target/unit-links-study`. Seed/trial count do not apply.
The shared report's `quality.eligible_players` diagnostic retains its original
meaning: admitted skaters from the reused pressure-source checker. The attached
unit population above separately includes eligible goalies with positive regulation
time; bench-only roster entries are excluded from unit appearances.

Input catalog SHA-256:
`80d1ccd5ba277114572bce243f1ff56a07fdb643dffae47833f3f240ab8ef026`.
Output catalog SHA-256:
`617db53825c8b7656cc9ea03996a0aa4d0eb1e533cef684323f386e6160b700e`.
The catalog remains within the existing 32 MiB admission bound.

## Validation

- 250 browser tests and 40 focused Rust tests passed: trio/pair/goalie semantics,
  PP/PK separation, uncertain participants, state denominators, trades, ties,
  recent subsets, stale identity/fingerprint rejection and prior pressure/refresh
  contracts. A neutral seven-game fixture verifies a recent leading combination
  that differs from the full sample under the actual last-five rule.
- Native CLI/Askama, browser and pinned unoptimized WASM builds passed.
- An independent reconstruction matches all 829 records: game IDs, state times,
  unresolved seconds, every retained group and its games/time, partner rankings,
  recent groups and all 240 recent/full differences. It processes 9,396 matched
  player-games. All 3,948 attribute-grade arrays remain identical; prior profile
  measurements agree apart from existing floating-point roundtrip noise.
- Offline skater refresh retains all 829 existing unit records (including the
  unselected goalie family) and leaves 36 missing records absent.
- Eight live skater/defender/goalie cards pass group text, state totals, recent
  details, actual worker parity, old-catalog fallback, 390px mobile overflow and
  one-page print checks with details open. Fox's default sheet was visually inspected.
- Eight 612×792-point Letter PDFs independently retain Unit Links, matched
  manpower totals and Attributes. Expanded details remain off the printed form.
- Catalog, report and paired-source receipts replay byte-for-byte. A bad source
  hash is rejected before output creation.

Browser acceptance: `node icelines-browser/e2e/rinkcraft-unit-links.mjs`.
Preview: [Fox](http://127.0.0.1:8780/?player=8479323), see **Unit Links** and
**Deployment**; expand **Alternates, partners & recent**.
No hosted publication or main merge is part of this change.
