# EDGE exposure comparisons

This Rust analysis distinguishes recorded peak skating speed, season burst volume
and bursts per 60 official all-strength minutes. It consumes a frozen exported
Rinkcraft cohort and leaves all profile data, feats and capability grades intact.

## Definition

- Bursts per game = recorded 20-plus-mph bursts / GP.
- Bursts per 60 = recorded bursts × 60 / official all-strength TOI minutes.
- Rank = 100 × (players below + half the tied players) / measured eligible peers.
- Reference: same season and position group (forwards or defensemen), including
  the subject. Missing values are excluded metric by metric; zero counts remain.
- Sensitivity scenarios: minimum 5, 10 or 20 GP, each also requiring 100 minutes.
  These are authored comparisons, not validated reliability cutoffs.

The input catalog hash is required. Output inventories every source profile hash
and carries each usable EDGE response hash. The command refuses existing output,
identity/scope mismatches and duplicate observations. Profiles lacking matched
EDGE remain excluded with their original reason. Goalies are outside this model.

## Interpretation

Per-60 exposure helps distinguish raw volume from recorded frequency per ice time.
It does not measure equivalent skating opportunities. Tracking coverage, deployment,
strength, score, fatigue and rink effects remain uncontrolled. Peak speed remains
a sample maximum affected by opportunities. A burst count does not measure
acceleration, agility, puck control, decision making or quality of skating routes.

These positional midranks are computed by IceLines. They do not replace or explain
the NHL-provided benchmark percentiles. None of these observations fill the other
Skating facets or complete the six-attribute capability battery.

Future observed-record feat candidates can distinguish peak speed, frequent
bursts, and their conjunction. Thresholds and minimum exposure must be calibrated
before those candidates award feats; this analysis issues no awards.

## Replay

```text
cargo test -p icelines-core --lib rinkcraft_edge_exposure --locked
cargo run -p icelines-sources --example rinkcraft_edge_exposure --locked -- FROZEN_DIRECTORY CATALOG_SHA256 NEW_OUTPUT_JSON
python tools/verify_rinkcraft_edge_exposure.py FROZEN_DIRECTORY REPORT_JSON NEW_VERIFICATION_JSON
```

The Python oracle independently recomputes the per-game/per-60 values, positional
reference membership, exact tie midranks and all input hashes. Python is a verifier;
production arithmetic and cohort analysis are Rust.

## Validation — 2026-11-19 Pacific

Three focused Rust tests passed, covering rate arithmetic, zero versus missing,
invalid units/counts, duplicate observations, positional reference separation,
midrank ties and minimum exposure. The frozen four-season cohort supplied 3,561
matched skaters. The independent oracle verified all 3,941 input profile hashes
and 24,993 metric ranks across the three exposure scenarios. Rust formatting and
diff whitespace checks passed. No native/browser UI or published data changed.

The dated numerical findings and reproducibility record belong in PUCK, with
retained report/input lineage under the external research artifact directory.
