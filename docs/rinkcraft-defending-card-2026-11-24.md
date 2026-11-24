# Defensive records on the character sheet — 2026-11-24

The existing Threat Denial box now displays two separate records near the top of
the sheet. **Defensive Work** shows NHL blocks and takeaways per all-strength
60 minutes. **Chance Control** shows MoneyPuck.com five-on-five xGA/60 and the
off-ice-minus-on-ice difference. Both remain measured context; no ability score,
documentary class, feat award or simulation bonus changes.

The shared Rust `rinkcraft_defending_record` module formats both records, the
supporting rows, coverage notes and peer comparison. Browser TypeScript binds and
renders that result. Native Askama uses the same Rust fields. The optional
`skater_context.defending` field has actor/season/GP/position checks; older catalogs
without it retain their existing fallback.

The compact front uses two side-by-side signals, followed by Response Timing and
Skating Detail. The Defense Record disclosure contains eight supporting rows,
the formulas, scope and limitations. The same record is accessible from the
Defending attribute detail. Print hides the disclosure and preserves both front
signals on one Letter page.

## Current data

The existing preview remains the frozen NHL snapshot fetched 2026-11-23, with
787 skaters. The new published season-summary CSV was captured at
`2026-11-24T15:00:55.8255574Z`. Five-on-five rows match official player/GP for
537 skaters. The other 250 retain activity records and explicitly unavailable
Chance Control; no older-season replacement or unmatched-GP rate is attached.

Examples for the **2026–27** window:

- Fox: 4.45 blocks and 1.14 takeaways/60; 2.16 five-on-five xGA/60;
  0.37 fewer xGA/60 on ice than off ice.
- McDavid: 1.46 blocks and 0.88 takeaways/60; 2.07 five-on-five xGA/60;
  0.32 fewer xGA/60 on ice.
- Beniers: activity remains available. His provider row has 21 GP versus the
  frozen NHL snapshot's 20, so Chance Control is unavailable for this window.

Raw rates are visible below the peer-ranking threshold. A rank requires 10 GP,
100 five-on-five minutes and 20 measured same-position peers. There are 386
ranked records in this supplied snapshot. This is a **GP-matched subset**, not a
claim of a complete league reference. Quantized midranks split ties. Team systems,
teammates and opponents remain uncontrolled; on/off does not isolate player impact.

## Routine refresh and frozen-source import

The normal `profile-refresh` path now attempts the published skater season CSV
and caches it with the existing source receipts. Where available it enriches the
selected window, then creates the defensive records. If that optional source is
unavailable, activity remains and Chance Control is explicitly unavailable.
Malformed admitted source data still fails validation. Re-enrichment clears old
five-on-five values before admitting a replacement, preventing stale records
from surviving a later missing/mismatched source row.

For the retained preview, the new Rust CLI command imports an explicitly captured
CSV without changing the NHL window or fetching other data:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-defending-records --season 20262027 --catalog target/attribute-calibration/inputs/rinkcraft-stat-attributes.json --season-summary target/defending-card-study/skaters-2026.csv --captured-at 2026-11-24T15:00:55.8255574Z --output-dir target/defending-card-study/enriched
```

Choose a new output directory for replay. This import leaves all 3,948 supplied
six-integer-grade arrays unchanged and preserves other seasons. Its report records
source URL/date, input/output hashes and coverage. Source URL:
[MoneyPuck.com 2026 regular-season skaters](https://moneypuck.com/moneypuck/playerData/seasonSummary/2026/regular/skaters.csv).

- Source SHA-256: `b569710f4563ff0b263eeccfc4aa6bf32572a758da5b3f6ae0e45c87b69da7a0`
- Enriched catalog SHA-256: `d84ab01a4ecc26005f503e1d95311796f3a722c97ba08b087467525de6972eb4`

## Completed checks

- Nineteen focused Rust tests pass: record construction/window binding, existing
  defensive comparisons and carryover, defensive/skater source parsing, stale
  context replacement, other-season preservation and refresh replay/cache guards.
- Locked native CLI build passes, compiling the Askama template. Rust 1.95.0 WASM
  build and matching wasm-bindgen 0.2.118 bindings pass.
- Full browser build and all 245 existing browser tests pass. A TypeScript-only
  build initially left stale offline hashes; rebuilding the full bundle resolved
  the offline test failures.
- All 787 complete preview profiles receive their records through actual WASM
  attachment. Formatted record fields agree exactly with the Rust import; raw
  floating rank/minute fields are compared within 1e-9. Integer grades agree
  exactly; no floating-point byte-identity claim is made across JSON runtimes.
- Browser acceptance passes Fox, Slavin and Beniers: top-of-sheet values, expanded
  supporting rows, attribute detail, exact record transfer through the actual
  browser worker, missing-window behavior, legacy fallback and mobile overflow.
- Three independently inspected PDFs each have one 612×792-point Letter page
  containing Chance Control and Defensive Work. Screenshots were inspected.
- Frozen import replay reproduces catalog and import-report bytes exactly.

Preview: `http://127.0.0.1:8780/?player=8479323`. This is a local generated profile
preview with matching renderer/worker/WASM. No hosted publication or full native
runtime acceptance is claimed.
