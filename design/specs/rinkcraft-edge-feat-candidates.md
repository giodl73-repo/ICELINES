# EDGE skating feat candidates

This Rust research command evaluates one feat family from the checksummed,
previously verified EDGE exposure report. It emits candidates and a census;
it does not alter the production feat registry, cards, abilities or simulation.

## Family: recorded skating speed

| Candidate | Peak-speed midrank | Bursts/60 midrank |
| --- | --- | --- |
| Top Gear | >= P90 | < P90 |
| Burst Engine | < P90 | >= P90 |
| Two Gears | >= P90 | >= P90 |

Two Gears replaces the individual labels. At most one candidate is emitted per
player/season; two related observations are not two independent evidence sources.
The proposed placement is Distinctive Features, describing recorded activity.
These names do not establish acceleration, endurance, pursuit, puck control,
route quality or overall Skating capability. Not qualifying produces no negative
feat. Missing measurements or small references are Unassessed; insufficient
exposure is a distinct state, not a failed measurement.

The P90 cutoff and minimum 20 measured peers are authored hypotheses, not validated
thresholds. The input evaluates 5-, 10- and 20-GP settings with 100 minutes each.
Ranks stay within the same season and position group. Both required metrics must
be measured and have sufficient peers before this exclusive family is classified.
Separate metric reference counts remain visible. The selected window is part of
the claim; these are never permanent or career-wide traits by default.

## Calibration and checks

The verifier separates changes among players assessed under both exposure settings
from lost eligibility. Otherwise a higher games cutoff can appear to change a
trait when it merely removes the player. Consecutive-season snapshot overlap is
also reported only for players assessed in both windows and in the same position
group. It is not held-out validation or a corrected reliability estimate. Partial
current seasons are not equivalent to completed seasons.

Production arithmetic and classification are Rust. The Python oracle independently
checks every classification, reference fields, counts, hash linkage, sensitivity
and snapshot overlap. Copied display floats are compared at 1e-12 tolerance for
JSON floating-point round trips; the original exposure bytes and their SHA-256
remain the evidence. Boundary tests cover P90 inclusivity, exclusive combination,
missing data, exposure and minimum peer limits, and invalid ranks.

```text
cargo test -p icelines-sources --example rinkcraft_edge_feat_candidates --locked
cargo run -p icelines-sources --example rinkcraft_edge_feat_candidates --locked -- EXPOSURE_JSON SHA256 NEW_OUTPUT_JSON
python tools/verify_rinkcraft_edge_feat_candidates.py EXPOSURE_JSON CANDIDATE_JSON NEW_VERIFICATION_JSON
```

Two focused tests passed; the independent oracle checked 10,683 classifications
across three exposure settings and four seasons. Dated census, sample players,
sensitivity and repeat-window findings are stored in PUCK and the retained
research packet. Candidate thresholds remain under study; no awards are released.
