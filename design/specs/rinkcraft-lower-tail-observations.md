# Balanced profile observations — exploratory v0.1

The salary-contrast Rust adapter now reports existing positive feats, existing
caution flags, and lower-tail measured observations together. These annotations
do not change the active 34-rule registry, capability grades, or simulation rules.

## Classification and gates

- **Low output:** goals, assists or points per game at or below P20.
- **Low activity:** shots, hits, blocks or takeaways per game at or below P20.
  Event volume describes activity, not defensive effectiveness or effort.
- **Low opportunity-conditioned output:** shooting conversion or PP points/60 at
  or below P20 in the existing experimental candidate screen. Retain its 100-shot
  or 120-PP-minute exposure gate and conditional F/D reference population.
- At least 20 games and 20 measured peers; use the existing season/position-group
  ranks. The current salary cohort separately requires at least 40 games.
- Exclude raw PP/SH totals, penalties and giveaways from the lower-tail whitelist.
  Missing values, absent opportunities and missing assessments are not flaws.
- Collapse repeated conditions to one observation per metric. Reject inconsistent
  copies of evidence. Preserve measured values, ranks and source lineage.

P20 is an exploratory display cutoff, not a calibrated weakness threshold.
No causal, repeatability or predictive validity claim is made. Per-game rates
remain sensitive to deployment; a future minutes-conditioned view may disagree.

Aim for at least five informative entries across strengths, cautions and lower
tails when supported. Report unmet coverage explicitly. Do not lower positive
thresholds to reach five. Entry counts are not independent skill counts: related
feats and caution flags reuse metrics. Goalies need their own coverage work.

## Defensive coverage gaps

Same-position peer rankings do not repair missing event measurements. This
report cannot assess entry denial, controlled exits, retrievals, gap/lane
protection, net-front coverage, rotation/cover decisions, passing under pressure
or PK prevention. Each player section calls these out as unmeasured here.
No new player-specific weakness is inferred from these shared coverage gaps.

## Validation and integration boundary

Producer review lenses: statistical validity (opportunities, overlapping metrics),
hockey interpretation (activity versus effectiveness), and profile design
(balanced strengths/cautions, visible unknowns). This is one producer's review,
not independent expert approval.

Six focused Rust tests passed. A separate Python check reproduced the annotated
metric set, verified all preexisting output fields unchanged for 16 actors, and
checked the five-entry target for the 12-player sample. The generated report is
in PUCK; the subsequent browser integration is described below.

Replay the existing salary adapter with its checksummed input manifest and new
output paths. Keep its former outputs intact for comparison. Source capture and
salary membership remain those of the prior frozen 2024–25 study; no new scrape
or current-roster inference is needed.

## Browser card integration — 2026-11-19

The four skater Season examples now carry a versioned
`rinkcraft.profile-observations.v1` child under their achievement profile. The
Rust `rinkcraft_observation_profiles` example accepts checksummed fixture/report
inputs and binds actor, window, GP, group and original report lineage. It preserves
the original JSON number tokens in all existing profile fields using RawValue;
this avoids rounding changes during an annotation-only projection. No additional
parser or scoring formulas were implemented in TypeScript.

The compact browser sheet calls the populated section **Cautions & Low Metrics**.
It shows positive/caution/lower-tail counts, up to two lower-tail summaries with
percentiles, expandable measurements, and a closed assessment-gap disclosure.
Additional observations remain browseable. Existing caution feats retain their
sources and detailed conditions. Entry counts include overlapping definitions;
they are not player levels or independent abilities.

The browser rejects mismatched identity, season, competition, group, games and
source hashes, insufficient peers, missing measurements, duplicate observations
and invalid lower-tail ranks. Rendering uses text nodes. Historical observations
are not attached to Current or Career alternatives. The goalie example receives
no skater annotation. No native template/API, production WASM data loader, goalie
assessment or simulation behavior is changed by this preview integration.

Replay:

```text
cargo test -p icelines-sources --example rinkcraft_observation_profiles --locked
cargo run -p icelines-sources --example rinkcraft_observation_profiles --locked -- INPUT_MANIFEST OUTPUT_FIXTURES
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-observations.test.mjs icelines-browser/tests/rinkcraft-sheet.test.mjs icelines-browser/tests/rinkcraft-profile.test.mjs
node tools/prepare_rinkcraft_theme_studio.mjs OUTPUT_FIXTURES target/compact-sheet/theme-studio
node icelines-browser/e2e/rinkcraft-observations.mjs
```

Validation: three Rust tests, 24 Node tests, exact preservation of all five
profiles' preexisting JSON values and ten Current/Career variants, four
annotation/report matches, 30 desktop and 30 mobile theme cases, ten scope
isolation checks, and five single-page Letter exports. Desktop/mobile screenshots
are retained with the generated fixtures. Existing capability ratings remain
unrated. These are local preview checks, not full native/WASM acceptance.
