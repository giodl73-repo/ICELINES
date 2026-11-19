# Three-season candidate feat transfer

Offline Rust analysis of the three advancing candidates: Shooting Conversion,
PP Production Rate and Whistle Balance. These remain experimental candidates;
the live34-rule registry, cards, capability grades and simulator are unchanged.

## Execution boundary

`tools/capture_rinkcraft_reports.ps1` captures explicit seasons into a new output
directory, never overwriting an existing capture. It assembles sorted official NHL
summary, realtime, penalties, goalie summary and timeonice reports, preserving
each raw page, URL, timestamp and SHA256. Regular-season scope is declared by the
query, not independently echoed in the response. Rust performs offline analysis.

The existing `rinkcraft_feat_screen` example runs the frozen nine-definition
single-season screen for2023–24 and2025–26; retained2024–25 inputs/results are
reused. Only the three advancing IDs enter longitudinal analysis. Per-season
screen specification changes only the declared season. All nine original
formulas/sensitivity settings remain unchanged.

`rinkcraft_feat_longitudinal` accepts an explicit JSON manifest containing three
ordered `{season,reports,screen}` objects. It validates source/actor/GP/group
alignment, definitions, opportunity gates and annual decisions. Development is
2023–24/2024–25; evaluation is2025–26. Prior studies already inspected those years,
so the runner requires `untouched_holdout:false`. This is retrospective temporal
transfer, not a pristine holdout, prospective study or calibrated skill model.

For each candidate/group, freeze weighted development references. Each distinct
actor has total weight one split over their eligible development seasons. Apply
weighted midrank percentiles and the P60 conjunction unchanged to evaluation
values. Also retain annual within-cohort awards and P55/P65 frozen sensitivity.
Persistence uses common eligible actors in the same position group; report
prior winners who disappear, change group or lose eligibility separately. No
sampling/probability independence is established by weighting.

```text
cargo test -p icelines-sources --example rinkcraft_feat_longitudinal --locked
cargo run -p icelines-sources --example rinkcraft_feat_longitudinal --locked -- INPUT_MANIFEST OUTPUT_JSON
```

## Observed scope and checks

Across three seasons:1,328 distinct actors,3,083 player-seasons. New captures
contain82 verified raw pages:924 skaters/98 goalies in2023–24 and940 skaters/98
goalies in2025–26. The prior2024–25 cohort has920 skaters/103 goalies. Candidate
eligibility remains strength/opportunity-specific; no goalie candidate was filled
from skater or all-strength substitutes.

Among comparable prior forward winners in2024–25, frozen-reference awards persist
in2025–26 for27/48 shooting-conversion,29/44 PP-rate and55/90 whistle-balance
players. Defense counterparts are9/14,8/13 and22/44. These are descriptive
conditional proportions subject to eligibility survival, context and uncertainty,
not predictive accuracy or stable ability. Evaluation shooting awards differ:
81 frozen-reference versus56 annual-reference awards. Reference semantics must
be visible before interpreting levels or changes.

Four Rust tests pass, including a future-value perturbation showing evaluation
data cannot change development references. Independent Python verification
recomputes7,606 annual and7,606 frozen condition values/ranks, all24 persistence
rows and six reference groups. Eight adversarial integration cases reject;
exact replay is byte-identical. New raw82 page hashes pass. Proportionate tests
cover only these offline tools; no UI/native/WASM/hosted rebuild is claimed.

Rinkcraft's `docs/research/feat-longitudinal-01.md` records the pre-result
exploratory plan and prior-exposure audit. Aggregate findings live there; PUCK
retains actor-level output, input/provenance manifests and immutable dated
artifact packet. The packet includes frozen definitions/candidates, both complete
new captures, retained baseline, source commit, executable and oracle. No automatic
promotion, class,1–20 grade or simulation bonus follows from persistence.
