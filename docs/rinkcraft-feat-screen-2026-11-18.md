# Candidate feat screen: local exploratory pass

This offline Rust example screens candidate definitions without modifying the
34-rule live registry, cards, classes, capabilities or simulation. The complete
33-candidate registry is checked; nine candidates receive numerical exploratory
tests, while 24 retain explicit missing-input gates. A gate review is not a
numerical test and does not establish that a candidate would be useful.

## Inputs and replay

Use the retained complete 2024–25 regular-season NHL report assembly from the
stat-feat pilot plus a new complete seasonal timeonice assembly: ten sorted
100-row pages, 920 actors. The runner checks assembly completeness, actor IDs,
season, joined games and positions, EV+PP+SH time accounting, shifts, signed net
penalties and numerator/denominator coherence. The new raw capture manifest
retains exact source URLs and hashes; game type comes from the query and is not
independently echoed by the provider body. No network access occurs in Rust.

```text
cargo test -p icelines-sources --example rinkcraft_feat_screen --locked
cargo run -p icelines-sources --example rinkcraft_feat_screen --locked -- REPORTS_JSON TOI_JSON CANDIDATES_JSON SCREEN_RULES_JSON OUTPUT_JSON
```

Definitions and the proposed next multi-season design are in the separate
Rinkcraft research repository: `research/stat-feats/screen-rules.v0.1.0.json`
and `docs/research/stat-feat-screen-01.md`. Unsupported protocol changes reject
rather than silently reuse the hard-coded 0.1 policy. Output preserves every
eligible actor's values and percentiles for all scenarios.

## Population and screening

The observed cohort has 601 forwards, 319 defensemen and 103 goalies. GP gates
leave 456 forwards, 242 defensemen and 66 goalies. Candidate-specific opportunity
gates further reduce the reference population. Goalies remain in the population
accounting, but their candidate inputs are not available in these captures.

Within each eligible F/D reference use midrank percentile, require every
condition at P60, then compare P55/P65/P75/P90, half/1.5 times the opportunity
gates, and GP10/GP40. This is nine scenarios per computed candidate, not a full
factorial experiment. Lower shift duration is explicitly lower-is-favored;
negative net penalties remain valid observations. Report binary award overlap
on common eligible actors; two empty award sets are not perfect agreement.

The 100-skater/40-goalie population targets are planning guides, not statistical
validation or mandatory reasons to weaken exposure gates. Many specialist
populations are smaller. Separate seasons, repeated identities and opportunity
uncertainty need dedicated evaluation. No held-out data were fitted or tested.

## Findings and checks

Nine computed candidates cover goal ledgers, observed shooting conversion,
PP/SH production rates, whistle balance, season workload, two versions of
PP+PK workload and shift cadence. Seasonal/per-game two-strength workload award
Jaccard overlap is 0.85, supporting a merge review rather than two new feats.
Shooting conversion has only 73 eligible defensemen; PP rate has 45. SH rate
has 78 eligible forwards and 95 defensemen at the primary exposure gate and is
sensitive to the exposure cutoff. These are descriptive results, not validated
talent claims. Aggregate findings and all 33 dispositions are recorded in
Rinkcraft; named-player outputs and provenance belong in PUCK.

Four Rust tests passed. An independent Python oracle recomputed 88,014
condition values/ranks/decisions across 81 candidate-scenario evaluations and
verified 63 overlap comparisons. Seven malformed-input integration cases
rejected. Exact replay was byte-identical; ten raw-page hashes verified.
The private dated artifact packet retains raw pages, original inputs, frozen
specification/candidates, output, oracle, executable, source commit and receipts.
No frontend, native app, WASM or hosted changes are part of this increment.
