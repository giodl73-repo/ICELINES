# Player-specific NHL EDGE evidence

Owner steering: use the public NHL EDGE API to populate attribute evidence.
This extends the previous one-player historical capture to all player-seasons
in the public preview. It does not release capability grades.

## Acquisition and scope

The source is the official NHL host, observed detail routes:

```text
https://api-web.nhle.com/v1/edge/skater-detail/{player_id}/{season}/2
https://api-web.nhle.com/v1/edge/goalie-detail/{player_id}/{season}/2
```

`tools/capture_rinkcraft_edge.ps1` takes an explicit preview catalog and a new
output directory, with four concurrent requests by default (maximum six), bounded
request deadlines and retained response bytes/URL/SHA-256/size/access time.
Requests are unique by actor/season. No browser credentials or private tracking
access is used. The source contract is an observed public response schema, not
a promise that NHL maintains these endpoints indefinitely.

The 3,941 requests produced 3,935 initial captures and six unavailable responses.
Three timeout cases were recovered once into a separate capture directory and
derived manifest retaining the original manifest checksum. Three HTTP 404s remain.
After strict source/frame validation, 3,933 records are usable:

| Season | Matched EDGE contexts | Baseline records |
| --- | ---: | ---: |
| 2026–27 | 856 | 858 |
| 2025–26 | 1,038 | 1,038 |
| 2024–25 | 1,021 | 1,023 |
| 2023–24 | 1,018 | 1,022 |

Eight exclusions are explicit data gaps, not player limitations: three missing
responses, four historical goalie appearance-count disagreements, and one
shot-location conversion inconsistency. In the last case the response reports
two goals, one shot and a fraction of 2 in a location band. Its cause is unresolved;
it is not accepted as a conversion probability. No source counts were repaired.

Skating-speed detail was also inspected during discovery; it exposes selected
top-speed events and burst bands. The cohort pipeline below uses detail responses
only. It does not claim complete movement trajectories or attempt-level tasks.

## Rust source adapter and projection

`icelines_sources::nhl::edge::parse` binds exact URL, actor, regular-season type,
season availability, position group, appearance count, timestamp and source bytes.
Goalie detail omits roster position; its goalie-specific route and supplied NHL
summary group bind that case. Missing values are not zero-filled.

Checks include numeric/fraction domains, integer counts, unique location rows,
physical-unit consistency and usable conversion counts. The imperial/metric
check permits max(0.02, 0.1% of metric value) discrepancy to accommodate rounded
provider conversion factors; this is not a sensor-precision claim. Physical
zero maxima are not interpreted as zero speed. No-opportunity fractions stay
unknown. Provider fractions and location codes remain provider observations.

`rinkcraft_edge_profiles` requires checksummed catalog, capture and baseline file
inventory inputs. It adds only `edge_evidence` and `edge_capture_status` under the
matching numerical frame, preserving original JSON number tokens. All 3,941
baseline profiles and the catalog remain exactly unchanged after removing those
two additions. A replay from the frozen baseline export is byte-identical.

NHL EDGE is a separate capture vintage from the baseline NHL reports. Matching
GP is an explicit alignment gate, not proof of identical clocks, tracking coverage
or provider definitions. Both source dates remain visible.

## Evidence meaning and presentation

- Skating: season maximum, bursts above 20 mph and season distance. Maxima/counts
  are context for Speed, not acceleration, agility, transition or endurance tests.
- Shooting: season maximum shot speed and reported location outcomes. Velocity
  maximum is not accuracy, release latency or a conditional finishing estimate.
- Defending: on-ice puck-zone time, explicitly not player location or individual
  coverage/denial effectiveness.
- Goalies: saving by location, explicitly not Tracking, Positioning, Movement,
  Rebounds, Recovery or Puck Play technique measurements.

Provider percentiles are retained on their 0–1 source scale and labeled **NHL
percentile** on display. Provider reference counts/eligibility are not supplied;
comparability with IceLines position-group midranks is not assumed. Location
subgroups are not forced to partition the all-location total. Repeated values,
URLs or transforms from this provider are not independent corroboration.

The attribute rulebooks show player-specific observations above the draft facet
recipe, with capture date, source response, benchmark limits and hashes. Skating
Detail on the front shows a matched observed maximum. Missing direct evidence
and the eight source exclusions remain explicit. No 1–20 capabilities, classes,
feat awards, thresholds, coaching assignments or simulation bonuses change.

## Replay and validation

```text
pwsh -NoProfile -File tools/capture_rinkcraft_edge.ps1 -Catalog CATALOG_JSON -OutputDirectory NEW_CAPTURE_DIRECTORY
cargo test -p icelines-sources --lib nhl::edge --locked
cargo test -p icelines-sources --example rinkcraft_edge_profiles --locked
cargo run -p icelines-sources --example rinkcraft_edge_profiles --locked -- INPUT_MANIFEST NEW_OUTPUT_DIRECTORY
python tools/verify_rinkcraft_edge_profiles.py INPUT_MANIFEST NEW_OUTPUT_DIRECTORY VERIFICATION_JSON
node tools/prepare_rinkcraft_public_preview.mjs NEW_OUTPUT_DIRECTORY STATIC_DIRECTORY
node icelines-browser/e2e/rinkcraft-edge-evidence.mjs
```

Five focused Rust tests passed. Thirty-six Node tests passed. The independent
oracle checked all 3,941 baseline profiles, all capture hashes, 24,892 measurement
values/provider percentiles, 15,732 location rows and 3,941 byte-identical replay
profiles. Browser acceptance passed 24 player/season contexts, all eight missing
cases, six mobile themes and single-page printing. The existing public explorer's
20 player/season and six mobile cases also passed. Screenshots were inspected.

Producer-applied lenses: evidence custody (raw bytes and exact source binding),
methods (provider ranks and maxima are not capability estimates), defense/goalie
(context is not technique), and consumer boundary (Rust owns parsing; TypeScript
renders; raw evidence stays in private artifact storage). This is self-review,
not an independent expert panel.

## Public preview version 3 — 2026-11-19 Pacific

Published at https://rinkcraft-nhl-preview.ivoryorchid34.chatgpt.site with public
visibility unchanged. Static source commit: `eefd2da0be2c6656c20c86d5b7bef0111b882382`.
Saved version: `appgprj_6aff7eac4c1081919202e36a7730dc2a~appgver_4d916bb78e248191adb009cf8736fcbc`.
Deployment: `appgdep_6aff9ac867608191815c4f17f5448e71` (succeeded).

The deployed-site acceptance independently passed the same 24 player/season
contexts, eight unavailable cases, six mobile themes and one-page Letter export,
with no page errors. All capability grades remained Unrated. Three deliberately
corrupted inputs (catalog hash, capture hash, baseline file hash) were rejected.
No main merge, hosted CI run or existing IceLines Pages deployment was performed.
