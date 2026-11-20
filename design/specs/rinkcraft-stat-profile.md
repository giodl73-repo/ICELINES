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
by slicing season aggregates. TUI support remains deferred. The core view is
reusable and deterministic; surfaces are thin callers/renderers.

## Player UI and WASM subset (CHG-126)

CHG-127 adds experimental descriptive `attribute_seasons` and player-specific
band/step-chart rendering; see [attribute contract](rinkcraft-attributes.md).
This is separate from the unvalidated class/ability/style fields.

The native player page links to `/player/:id/rinkcraft`; its JSON twin is
`/api/v1/player/:id/rinkcraft` (standard IceLines data/meta envelope). Both accept
`type=regular|playoff`, optional inclusive `first_season`/`last_season` IDs;
HTML also accepts `season` to display one loaded season instead of the summary.
Omitted competition defaults to regular, independent of the global header.
The native adapter uses canonical local career fan-out and report hydration on
a blocking worker, without mutating the shared repository or fetching data.

The standalone browser player dialog includes the same evidence profile. Worker
operation `rinkcraftProfile` calls `BrowserEngine.rinkcraft_profile(request_json)`
with the core request shape. The storage-independent core builder accepts
borrowed views from normal or sparse repositories. It filters identity, competition
and scope, sorts and deduplicates seasons (first supplied matching view wins),
and uses the same stat catalog and aggregation as native IceLines.

Browser coverage is only the resident prepared packages, up to eight windows.
Unloading/eviction shortens it. It never initiates career downloads and package
v1 does not carry native optional reports. Those values remain unavailable;
this is semantic parity for identical inputs, not equal source coverage. The
dialog preserves the existing per-package source/date evidence. Rendering, search
and season selection do not introduce alternative hockey calculations or scores.

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
