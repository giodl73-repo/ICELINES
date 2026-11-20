# Descriptive historical snapshot differences

`HistoryCard.changes` is additive and defaults to an empty list for old JSON.
The shared Rust reducer compares adjacent **supplied** snapshots in chronological
order, including explicit missing-player frames. It never skips a missing frame
to create a usable difference. Calendar gaps remain labeled by actual dates;
no interpolation, annualization or significance test is performed.

Raw differences require available player rows, both metric values, and matching
position group, situation, unit, formula and scale. A context/definition mismatch
withholds the combined raw values as well as the difference; original snapshot
values remain in the existing history table. Missing observations are not zero.
Unranked measurements may retain valid raw differences, but rank differences
require both measurements to be ranked with finite percentiles in0–100.

Every difference is later minus earlier. Percentage-valued raw measures are
stored as fractions; display multiplies differences by100 and labels them
percentage points. Percentile differences already use the0–100 rank scale and
are labeled separately. A raw rate may decline while its annual percentile rises.
Higher is not always better. Matching catalog fields does not establish provider
model-vintage, deployment or opportunity comparability. These are descriptive
results, not ability evolution, causal effects, significance or a1–20 mapping.

The dossier shows GP/minutes alongside each pair and retains the full original
measurement table, source hashes and interpretation limits. Native and browser
consumers format the same Rust fields; no difference calculations run in JS.
The historical preview projector also caps its dossier at the selected numerical
season, instead of attaching later supplied results to an earlier selected card.
This changes that preview's history scope, not the selected numerical card or
experimental finishing calculation. Documentary identity keeps its own date.

Focused checks:

```powershell
cargo test -p icelines-core --test rinkcraft_history_changes --locked
cargo check -p icelines-web --locked
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-profile.test.mjs icelines-browser/tests/rinkcraft-sheet.test.mjs
```

After rebuilding WASM with matching wasm-bindgen glue, use the existing window
projector/acceptance harness with `ICELINES_REQUIRE_HISTORY_CHANGES=1` and a new
output directory. Native parity uses the offline exporter:

```text
rinkcraft_history_snapshot HISTORY_JSON PLAYER_ID LAST_SEASON OUTPUT_JSON
```

No new source acquisition, model fit, capability release or game calibration.
Named-player runs and hashes belong to PUCK; Rinkcraft owns interpretation.
