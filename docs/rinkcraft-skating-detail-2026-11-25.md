# Populate the measured Skating Detail box — 2026-11-25

The card previously displayed a maximum only when a separate EDGE/style dossier
was attached. The numerical stat profile already held the measured inputs used by
the Skating estimate. Native and WASM projection now expose those inputs directly
through the same Rust formatter. Contract:
[Skating Detail v1](../design/specs/rinkcraft-skating-detail-v1.md).

## Frozen examples

| Player | Reported peak | 20+ mph bursts /60 |
|---|---:|---:|
| Connor McDavid | 24.01 mph | 22.98 |
| Nathan MacKinnon | 24.01 mph | 21.52 |
| Matty Beniers | 21.98 mph | 4.52 |
| Adam Fox | 20.88 mph | 0.91 |

These are recorded all-strength inputs in the existing 2026–27 snapshot, not new
technical skating assessments. Two-decimal displays can coincide while underlying
values differ. Distance and existing peer positions expand separately. Nothing here
measures reaction time, puck-carrying speed, acceleration, turns or pivots.

786 of 787 current skaters have usable inputs; the missing record remains absent.
No goalie receives a skater movement record. Observations have no ten-GP admission
gate; existing feat/peer eligibility rules remain independent. Zero remains zero,
while a missing peak can show an observed burst rate with its correct unit.

## Implementation and refresh

`rinkcraft_skating_detail::build` reads the selected profile's unique measured
Skating terms with exact unit checks, and optional normalized distance. It creates
the typed optional record during stat-profile admission. Catalog validation rejects
stale actor/season/group/GP/minutes or altered display strings. Native Askama and
browser use the Rust result. The old supported maximum-speed fallback remains for
older/absent projections. Goalies retain separate Movement Detail semantics.

The record is derived on admission, so the normal refreshed stat catalog needs no
additional enrichment command. The existing complete input catalog is unchanged:
`target/unit-links-study/enriched/rinkcraft-stat-attributes.json`, SHA-256
`617db53825c8b7656cc9ea03996a0aa4d0eb1e533cef684323f386e6160b700e`.
There was no new source fetch or attribute/feat formula change. The 829 Unit Links
and earlier pressure records remain intact.

## Validation

- 251 browser tests and 38 focused Rust tests passed, covering zero/missing data,
  partial records, exact units/scope, goalie separation, absent old fields,
  stale/altered record rejection and prior score/pressure/unit/refresh contracts.
- Browser, pinned unoptimized WASM and native CLI/Askama builds passed.
- All 865 local profiles were projected and projected again from their serialized
  admitted records. The derived skating record matches after that roundtrip.
  Existing six-grade histories, feats, pressure samples and unit records are retained.
- An independent comparison matches peak, burst rate and available distance display
  values against the original source inputs for all 786 populated current skaters.
- Eight live profiles verify values, expanded measurements, actual worker parity,
  goalie withholding, old-field fallback, mobile overflow and print with details
  open. Fox's expanded box was visually inspected.
- Eight one-page 612×792-point Letter exports independently retain Attributes and
  Unit Links and omit the expanded skating scope.

Generated projections, coverage, screenshots, PDFs and replay helper remain ignored
under `target/skating-detail-study`. Browser acceptance:
`node icelines-browser/e2e/rinkcraft-skating-detail.mjs`.
Preview: [Fox](http://127.0.0.1:8780/?player=8479323), see **Skating Detail** and expand
**Measures & scope**. No hosted publication or main merge is part of this change.
