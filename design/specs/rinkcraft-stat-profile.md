# Rinkcraft statistical evidence profiles

Date: 2026-11-08. Initial implementation contract, schema
`icelines.rinkcraft-stat-profile.v1`. Research family meanings remain proposed in
the independent Rinkcraft research repository; no validated role classifier
or tactical blame model is claimed. Rinkcraft owns research; IceLines owns this
reusable computation and source loading.

## Interface

```text
icelines profile "Full Player Name" --json
icelines profile PLAYER_ID --first-season YYYYZZZZ --last-season YYYYZZZZ --json
icelines profile PLAYER_ID --type playoff --json
```

Scope is available loaded career or inclusive whole seasons. Output includes
identity, isolated season type, selected/absent seasons, latest available row,
all stat-catalog cells by season, a strict summary, arithmetic changes, six
nonadditive job-family evidence groups and explicitly unmeasured dimensions.
Current-season availability within the selected scope differs from latest loaded season. Available
career is not a claim that all of the player's NHL career has been obtained.
Season-to-season differences do not establish a role change, trend significance
or increased ability. Partial seasons have different exposure.

Exact calendar-date ranges need coherent game-level inputs and are not implemented
by slicing season aggregates. TUI, Web and browser support remain deferred. The
core view is reusable and deterministic; the CLI is a thin caller/renderer.

## Source and aggregation policy

Season metrics use `StatId::read`, applicability, era and trade guards. The career
fan-out is rehydrated through `load_into_repo` with the configured snapshot store
so available Tier-1/optional reports are not lost. Per-season missing reports
travel with the output. Source routes identify catalog reports, not verified input
hashes/provider model versions. No live fetch is initiated by this command.

Additive counts and actual TOI totals sum only with complete selected-row coverage.
Supported rates are recomposed from summed numerator/denominator components:
shooting and save percentage; goals/assists/points per game; goals/assists per 82;
TOI per game/shift; shifts per game; supported realtime counts per 60. The output
retains component keys, totals and scale. Zero denominators are unavailable.
Do not average seasonal percentages, sum average TOI or divide PP counts by total
TOI. Unsupported multi-season rates, faceoff percentages without attempts, and
model outputs without version/denominator contracts remain unavailable in the
summary while individual season values remain accessible. No empirical thresholds
or class weights are introduced here.

Job-family links are context/proxies, not ability scores: hits are not pressure,
takeaways are not complete controlled recovery, assists are not causal creation,
and goals against are not individual defensive error. Directional style, passing
options, exact assignments, goalie positioning/commitment and causal lineup fit
remain unmeasured by this aggregate source. Future event/microstat adapters must
preserve definitions, windows, opportunities, strength state and source provenance.

## Verification

Core synthetic fixtures cover unequal-denominator shooting/save percentages,
playoff isolation, missing seasons, missing optional reports versus measured zero,
unsupported aggregation, empty windows, unknown identity and reversed range.
CLI integration checks cover public flags and invalid-window rejection. The
source loader is checked for report rehydration. Local validation and actual
command results are recorded in the feature review; no hosted workflow or release.
