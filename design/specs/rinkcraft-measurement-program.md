# Complete skater attribute measurement program

**Research track, superseded for the primary profile.** The owner's later
direction selects simple public-stat estimates for all six skater attributes;
the active scoring and completion rules are in
[Simple stat attributes](rinkcraft-simple-stat-attributes.md). The 23-component
technical measurement battery below remains a separate research program. It
does not gate the six primary profile estimates or imply that they measure
technical skills directly.

The original research objective was to finish measurement work for **all six skater
attributes**, rather than extend Finishing alone. The accepted registry requires
23 components. This program preserves those constructs and does not replace
them with easier aggregate-stat proxies. Goalies retain their separate battery;
this six-skater implementation does not complete goalie measurement work.

## Completion evidence

A measurement specification, working importer, synthetic test or source maximum
does not establish completed real-player measurements. Completion needs actual
source-linked observations for the target players/window, complete required task
dimensions, appropriate context/opportunity denominators, resolved missingness
and attribution, sensor/coder reliability, and chronological replication and
validation. Population coverage must include ordinary players, forwards and
defensemen. Report insufficient exposure explicitly. A league reference and
validated 1–20 aggregation are additional requirements before headline grades.

The current source captures do not meet these requirements for any complete
headline. The goal remains incomplete. No public cards or simulator values are
changed by the intake implementation.

## Required measurements and remaining work

These are the existing registry's task constructs expressed as collection and
analysis requirements. Formulas below are task-level descriptive quantities,
not already fitted/calibrated capability estimators.

| Attribute / component | Numerical measurements to collect | Required analysis / important restriction |
| --- | --- | --- |
| Skating / speed | Repeated calibrated speeds (m/s), runway, starting speed, controlled endpoint | Robust upper quantile and distribution conditional on comparable tasks; a season maximum is context |
| Skating / acceleration | Time over registered first 6 m or separately calibrated speed band, starting state | Lower matched time is better; decision delay is separate; no unbridged test pooling |
| Skating / agility | Turn/braking time, entry/exit speed, route compliance, geometry | Joint time/speed-loss model conditioned on path compliance and difficulty |
| Skating / transitions | Pivot latency, entry/exit speed and orientation, usable exit stance | Speed retained = exit/entry when entry speed is positive; latency alone omits correct exit |
| Handling / reception | Playable arrivals, controlled receptions, arrival quality, receiving pressure | Controlled / resolved playable arrivals; passer errors and ambiguous arrivals retained separately |
| Handling / control | Defined control tasks, controlled endpoints, separations, speed/traffic | Completed / resolved tasks; not possession duration or carry distance |
| Handling / protection | Identified challenges, control-preserving endpoints, wall/contact/support | Preserved / resolved challenges; deliberate controlled release can succeed |
| Handling / pressure execution | Evasion attempts, registered exit, control continuation, pressure | Controlled escapes / resolved attempts; route choice is separate |
| Passing / delivery | Intended targets, attempts, playable arrivals, actual reception separately | Playable / resolved deliveries; receiver control is not automatically passer execution |
| Passing / range | Target success, distance, side, bank/loft variant and receiving access | Standardize success over a fixed registered task mix; freeze distance boundaries before fitting |
| Passing / timing | Release/arrival times, feasible arrival window, delivery playability | Window-aligned / resolved tasks plus arrival margin; separate decision latency |
| Passing / deception | Matched contexts, concealment/action, opponent response, usable delivery | Conditional usable-delivery contrast; no sole causal credit from one defensive reaction |
| Shooting / finishing | Goals, baseline xG, exposure, shot context and model vintage | Conditional shrunk conversion effect; existing Rust Gamma–Poisson annual approximation remains restricted |
| Shooting / accuracy | Intended target, measured placement, target hit, interference, shot type | Placement error in metres and target-hit probability; SOG / attempts is an outcome proxy without aim |
| Shooting / release | Committed shot motion to release time, window, quality/placement preserved | Quality-preserving latency and window-completion probability; decision time separate |
| Shooting / velocity | Repeated calibrated shot speeds, shot type, location and interference | Matched shot-type distribution; single hardest shot is context |
| Reading / recognition | Feasible options and information visible before outcome; blinded option quality | Decision regret = best feasible option value minus chosen value under a registered option-value rubric; assists do not supply the option set |
| Reading / anticipation | Available cues, prepared response, correct/false commitments and costs | Matched preparation quality plus false-commitment tradeoff; not raw movement speed |
| Reading / decision timing | Feasible commitment window, actual commitment, premature-action costs | Conditional window alignment and decision quality; raw reaction milliseconds insufficient |
| Defending / gap | Carrier/challenger paths, relative speed, support, containment/bypass | Matched containment probability and gap evolution; closest distance is not inherently best |
| Defending / coverage | Named tested threat/duty, maintained relation, handoff and replacement cover | Contested / resolved threat intervals; untested occupancy is not success and unknown duty blocks assignment blame |
| Defending / denial | Named attempted option, attributable intervention, legality, resulting restriction | Legal denial / resolved intervention with difficulty and penalty-cost treatment; voluntary dump is not automatically forced |
| Defending / recovery | Feasible initial disadvantage, target, restoration/control, time margin | Restored / resolved recovery tasks plus time margin; teammate recovery is not automatically actor success |

## What current public sources can support

MoneyPuck annual and shot files provide outcomes, xG and shot context. The
[provider's data page](https://www.moneypuck.com/data.htm) describes shots, misses
and goals and explicitly excludes blocked shots from the shot dataset. They do
not by themselves identify receiving attempts, intended aim or feasible options.
Avoid turning an incomplete event census into a denominator for all actions.

NHL EDGE provides descriptive speed, shot and zone context. The
[NHL description](https://www.nhl.com/news/nhl-edge-launches-website-for-puck-and-player-tracking-data)
defines zone time in terms of the puck while the player is on ice, not individual
player location. These records cannot establish coverage duty or individual
prevention. Existing source bindings and caveats remain unchanged.

The historical NHL PPT per-game endpoint was separately probed on 2026-11-20:

- `https://live.nhle.com/ppt/20232024/2023020500/LiveEventSummary.json`: HTTP200,
  45 entity summaries including non-players. Actor mapping uses the matching
  `EntityRegistration.json`, not row order or nearest location.
- The same filename under `20252026/2025020500` returned HTTP404. This is one
  tested missing game, not a census proving season-wide absence.
- The downloaded older game has possession duration, game speed summaries,
  distance and multiple time clocks. This adds possible exposure/context evidence;
  no pass-attempt or control-success denominator is inferred.
- Feet and feet/second are unit assumptions following the inspected
  [client source](https://github.com/ethanrawson/bluelineR/blob/Model/R/game-tracking.R).
  The payload itself does not authenticate those units. Raw values are preserved;
  derived conversions remain conditional pending independent verification.
- A last-frame location is not a complete motion trace. The sample was captured
  with provider period3/clock9tenths; full-game finality is not asserted.

## Implemented measurement intake

`rinkcraft_task_measurements::measure` accepts completed coding with all23 task
contracts, a registry hash, explicit skater population, context cells, source
hashes, sampling selection status, and three strictly chronological game blocks.
Each observation identifies actor, component, game, episode, opportunity, source
locator, assessor and observed interval. Every declared numerical channel needs
either a finite bounded value or an explicit missingness reason on resolved rows.
Unresolved/out-of-scope rows cannot masquerade as performance measurements.

The nine-outcome accounting follows the registry. Eligible excludes out-of-scope;
resolved contains success, technical failure, choice failure, opponent wins and
teammate failure. Non-success is not automatically an individual error. The
success/resolved ratio is descriptive and does not replace component-specific
joint or conditional models in the table above. Reading requires an explicit
pre-outcome-coding assertion; the validator cannot authenticate that assertion.

Reports keep actor/component/context/split cells separate and retain per-game
outcomes and source lineages. They provide empirical type7 quantiles and a
delete-one-game jackknife SE for success/resolved with at least three resolved
games. This is not a calibrated confidence interval or reliability certificate;
between-game dependence and constant-outcome zero-SE cases remain limitations.
Every player receives all23 coverage rows, including genuinely absent data.

Native CLI:

```text
cargo test -p icelines-core --test rinkcraft_task_measurements --locked
cargo run -p icelines-core --example rinkcraft_task_measurements --locked -- INPUT_JSON SOURCE_PATHS_JSON NEW_REPORT_JSON
py -3.12 tools/verify_rinkcraft_task_measurements.py INPUT_JSON REPORT_JSON NEW_RECEIPT_JSON
cargo test -p icelines-sources --lib nhl::tracking_summary --locked
cargo run -p icelines-sources --example rinkcraft_tracking_summary --locked -- GAME_ID SUMMARY_JSON ENTITIES_JSON NEW_REPORT_JSON
py -3.12 tools/verify_rinkcraft_tracking_summary.py SUMMARY_JSON ENTITIES_JSON REPORT_JSON NEW_RECEIPT_JSON
```

The source-path inventory maps every protocol source ID to an explicit local file
(relative paths resolve beside the inventory). The CLI verifies every source
SHA256 and rejects an existing output. Fingerprints authenticate retained bytes,
not observational truth, semantic suitability or annotation independence.

## Validation completed 2026-11-20

- Six focused intake tests pass, including full23-component accounting, explicit
  channel missingness, genuinely absent players, ordering invariance, game-level
  variation, identity/context/source/range guards, pre-outcome Reading and strict
  chronological separation. Test-only synthetic export is explicitly opt-in.
- Two source-parser tests pass, including real endpoint single-object-array
  envelopes, game/actor identity, missing/zero separation, conditional unit
  conversion, duplicate entities and invalid physical domains.
- Native executables build. Independent Python arithmetic checks pass for1,863
  **synthetic** observations,69 actor/component/context/split cells and46 component
  coverage rows. This is software verification, not real-player assessment.
- Exact replay is byte-identical; a changed source checksum is rejected before
  output creation; existing output is rejected and its bytes remain unchanged.
- The captured real-game replay maps40 tracked players (36 skaters/4 goalies),
  with38 possession records,38 differing time-clock pairs and4 zone-subtotal
  discrepancies. An independent oracle checks all400 numerical fields plus
  identity, zone values, diagnostics and source hashes. Floating source values
  use relative2e-15/absolute1e-12 tolerance for cross-runtime f64 representation;
  integer fields and identities are exact. This is one historical sample, not a
  population census or a verified final-game record.
- Strict Clippy is blocked by existing diagnostics in other modules, including
  player-line history, simulator task code, EDGE exposure and the earlier
  Finishing fitter. This increment does not rewrite those unrelated modules.
  A subsequent warning-permitted diagnostic run completed all selected targets
  with28 existing diagnostics and none in this increment's new files; it does
  not constitute a strict Clippy pass.

Source observation and coding assertions still require measurement review.
Producer review is not an independent coding/research panel. No hosted build,
publication, workflow change, score release or main-branch merge was performed.

## Remaining implementation and evidence sequence

1. Complete the public-source feasibility census at game/action level, including
   coverage and eligible denominators, rather than inferring completeness from
   a single successful endpoint or example player.
2. Obtain source-supported task observations across all23 components. Freeze
   collection context, channel units, task boundaries, options/targets and
   outcome-blinding procedures before confirmatory collection.
3. Verify timing/geometry, actor attribution and independent coding reliability;
   preserve unresolved cases and distinguish overlapping evidence lineages.
4. Fit component-specific context models on the first two chronological blocks;
   test stability and later-game performance without tuning to held-out players.
5. Calibrate the complete battery and joint uncertainty with a full reference
   population; audit role/position common support and missing-player coverage.
6. Only then populate the six headline scales under the accepted aggregation
   contract. No current intake output satisfies that release gate.
