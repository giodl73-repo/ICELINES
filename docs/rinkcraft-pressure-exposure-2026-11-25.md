# Matched late-lead exposure — 2026-11-25

The Protecting lead disclosure now shows observed on-ice time and action counts
from an explicitly matched shift subset. The original 377-game headline counts
stay separate. Neither the time nor the counts become a capability grade.

## What the sample shows

Reused public evidence supplies 248 native shift-chart games through November 11,
2026, from the earlier 298-game line-analysis capture. Fifty HTML-derived games
are deliberately outside this native-only import path. There was no new fetch.
Source hashes were checked against the earlier sealed manifest before preparing
the import; the importer independently checks every paired file again.

757 current skaters receive matched exposure: 262 defensemen and 495 forwards.
Thirty other skaters retain their action counts without invented exposure.
552 have positive observed late-lead ice time. There are 54 excluded player-game
records with overlapping intervals or shift/box TOI discrepancies beyond the
authored one-second tolerance. Thirty-four cards have some unresolved team clock;
seven of these have no retained player seconds. Those seven are not evidence of
zero deployment. Another 198 have zero observed seconds without unresolved clock
in their matched subset; this does not establish deployment outside that subset.

| Player | Matched roster games | Observed late-lead ice time | Matched blocks | Matched takeaways |
|---|---:|---:|---:|---:|
| Adam Fox | 17 | 13:38 | 1 | 0 |
| Matty Beniers | 13 | 1:28 | 0 | 0 |
| Sebastian Aho | 16 | 9:41 | 0 | 0 |
| Nathan MacKinnon | 16 | 4:16 | 0 | 0 |

All four examples have no unresolved team clock in this subset. Different game
coverage and team score histories prevent reading this table as a quality ranking
or a coach-confirmed role. Ice time describes exposure, not the number of action
opportunities. The paired sample contains 163 credited late-lead blocks and 18
takeaways; the larger headline sample still contains 231 and 33 respectively.
These are different samples, not improved/worsened performance.

## Method and integration

Rust reuses the regulation participation reconstructor, exposing untruncated
team segments without changing existing outputs. Score changes split the final
five minutes. A goal at a boundary supplies the subsequent half-open interval's
score. Duplicates cannot inflate time. Conflicting shifts and invalid occupancy
remain censored. Full-game TOI reconciliation includes overtime, while the reported
context excludes it. All manpower situations remain included.

Counts come from the same games as the retained player shift sample, but an event
can occur inside censored clock or on an ambiguous change boundary. The system
does not divide those counts by retained ice time or claim event-time assignments.
Unresolved team seconds are not added to player ice time. Details and measurement
limits: [Under Pressure v1](../design/specs/rinkcraft-under-pressure-v1.md).

An explicit offline CLI import attaches the optional typed exposure to the
existing action sample. Rust formats the disclosure; browser and Askama render
the same result. Old catalogs remain compatible and show no inferred minutes.
WASM reprojection preserves the study. Offline record rebuilding and routine
refresh preserve it only when the parent sample is identical; changed captures
withhold it until an explicit paired import is rerun. No generic shift fetch,
feat, role assignment, simulation modifier or attribute formula is added.

## Reproduce

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-pressure-exposure --season 20262027 --catalog target/pressure-actions-study/enriched/rinkcraft-stat-attributes.json --game-cache-dir target/goalie-puck-study/cache --shift-manifest target/pressure-exposure-study/inputs.json --output-dir target/pressure-exposure-study/enriched
```

Use a fresh output directory. The bounded manifest schema is
`rinkcraft.pressure-shift-inputs.v1`, with season, RFC3339 source capture timestamp,
and unique game records. Each record carries game ID, boxscore/shifts paths,
SHA-256 values and explicit official URLs. Native endpoints are
`https://api-web.nhle.com/v1/gamecenter/{game}/boxscore` and
`https://api.nhle.com/stats/rest/en/shiftcharts?cayenneExp=gameId={game}`.
The play-by-play body must belong to the checksum-verified parent cache.

Prepared inputs, source selection, original-manifest fingerprint, independent
audit, generated catalogs and browser artifacts remain ignored under
`target/pressure-exposure-study`. Seed/trial count do not apply: this is a
deterministic descriptive import. No source file or PUCK scenario was modified.

Input catalog SHA-256:
`4a8fd139f45540b84a324f59bc3fe7f45b165a5b5d7dafa696c0b17aef6dcb9a`.
Output catalog SHA-256:
`80d1ccd5ba277114572bce243f1ff56a07fdb643dffae47833f3f240ab8ef026`.

## Validation

- 249 browser tests and 44 focused Rust tests passed. Source cases cover score
  boundaries, duplicates, overlapping/partial shifts, TOI discrepancies, wrong
  games/actors, incomplete/native-only admission, invalid clocks and overtime.
  Core cases cover exposure bounds, game scope and stale-parent removal.
- Native CLI/Askama, browser and pinned unoptimized WASM builds passed.
- Independent second-by-second reconstruction matches all 757 attached studies:
  game IDs, observed seconds, unresolved clock and same-game action counts.
  All 3,948 integer attribute-grade arrays remain identical. Other measurements
  match apart from existing float parse/serialize noise (maximum 2.85e-14).
- Full offline routine refresh retains all 757 studies and leaves the other 30
  absent. Changed parent fingerprints withhold exposure in the core test.
- Offline record rebuilding also retains all 757 studies. Catalog, report and
  paired-source receipts replay byte-for-byte. A wrong source hash is rejected
  before any output directory is created.
- Eight live cards pass action/exposure text, actual worker parity, goalie
  preservation, defensive gaps, missing-exposure/legacy fallback, mobile overflow
  and print checks. Fox's expanded disclosure was visually inspected.
- Eight one-page 612×792-point Letter PDFs independently retain Attributes and
  Estimated Record while hiding expanded explanations. No page errors.

Browser acceptance:
`node icelines-browser/e2e/rinkcraft-pressure-exposure.mjs`.
Preview: [Fox](http://127.0.0.1:8780/?player=8479323), expand **Protecting lead**.
No hosted publication or main merge is part of this change.
