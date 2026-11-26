# Goalie feat coverage trial — 2026-11-25

The first identity audit found 17 of 78 current goalies with positive highlights:
34 were below the existing GP minimum and 27 were eligible without one. The three
active goalie feats combine starts, workload, shutouts and saving outcomes. This
trial tests whether separating the existing saving/workload conjunction makes
seasonal records more informative. It adds no new measured capability.

Contract: [Goalie singleton feat trial v1](../design/specs/rinkcraft-goalie-feat-trial-v1.md).
The active registry remains 34 definitions. The explicit trial has 36 definitions,
with **Save percentage** in Task Skills and **Shot workload** in Role Coverage.

## Same-input results

| Season | Goalie rows | Eligible at 10 GP | Existing recognition | Trial recognition | Newly recognized |
|---|---:|---:|---:|---:|---:|
| 2023–24 | 98 | 73 | 31 | 50 | 19 |
| 2024–25 | 103 | 73 | 30 | 46 | 16 |
| 2025–26 | 98 | 72 | 32 | 47 | 15 |
| 2026–27, partial | 78 | 44 | 17 | 27 | 10 |

Recognition means at least one earned statistical feat, not overall ability or
technical skill. All three existing goalie feat objects remain identical under
the trial on these inputs. For the current sample each candidate has 18 qualifiers:
seven Bronze, seven Silver and four Gold.

Current newly recognized examples separate the two concepts:

- Linus Ullmark and Daniil Tarasov: Save percentage · Gold.
- Dylan Garand: Save percentage · Silver.
- Darcy Kuemper: Shot workload · Bronze.
- Sergei Bobrovsky and Joseph Woll: Shot workload · Silver.

The other five new cases and exact conditions are available in the generated
study. Low-GP cases, including the Shesterkin fixture, remain ineligible. Neither
candidate forces every goalie to have a feat or changes any of the six estimates.

## Temporal and source findings

For the two completed adjacent-season comparisons, saving qualification repeats
in 11/28 and 17/29 paired prior qualifiers; workload repeats in 14/27 and 14/28.
These are descriptive overlaps with changing annual references and eligibility,
not fixed-cohort predictions or stable player-type validation. The 2025–26 to
partial 2026–27 comparison is reported separately with eligible-pair attrition.

Five checksum-verified cached NHL pages supply all 377 goalie-seasons, including
both pages of the 103-row 2024–25 report. Captures keep their original dates.
Fifty-five rows have an additional goals-against accounting difference relative
to shots minus saves (10/18/22/5 by season). Reported saves and shots are preserved;
the cause of that extra-field difference is not established. Neither candidate
uses goals against to reconstruct a different save count.

## Validation and reproduction

- Ten focused Rust tests pass: three trial, six repertoire and one source-input
  test. They cover default-registry preservation, baseline feat equality,
  missing/low-GP states, wrong position/window, bad counts, retained accounting
  differences, source completeness and stable existing task behavior.
- 257 existing browser unit tests pass. Native trial example and pinned
  unoptimized WASM builds pass.
- Independent arithmetic matches 377 player-seasons, 754 candidate assessments
  and all six temporal-repeat rows. Native study replay is byte-identical.
- Negative controls reject changed checksums, incomplete pagination and duplicate
  pages without writing partial outputs.
- All 78 current trial award objects match actual WASM evaluation of complete
  public NHL report assemblies; all 78 repertoires match shared Rust/WASM selection.
  The base cards' existing feats must match exactly before trial cards are written.
- Four actual trial cards pass candidate values, same-player active-card links,
  experimental labels, WASM repertoire parity, mobile and one-page print. All 78
  active goalie profile files remain byte-identical. Independent PDF checks verify
  four 612×792-point Letter sheets retain the experimental marker. Ullmark's
  screenshot was inspected and the marker contrast corrected.

All ranking, qualification, source normalization and temporal comparisons are
Rust. Node prepares review artifacts; Python is independent verification only.

```powershell
cargo run -p icelines-sources --example rinkcraft_goalie_feat_trial --locked -- target/goalie-feat-trial/manifest.json NEW_OUTPUT
node icelines-browser/e2e/prepare-goalie-feat-trial.mjs STUDY_JSON BASE_SITE NEW_CHILD_DIRECTORY WASM_DIRECTORY CURRENT_REPORT_ASSEMBLY
node icelines-browser/e2e/rinkcraft-goalie-feat-trial.mjs
py target/goalie-feat-trial/verify.py
```

The manifest schema is `rinkcraft.goalie-feat-trial-input.v1`, containing `pages`
with `season`, `file`, `url`, `sha256` and `captured_at`. Run from the repository
root or supply absolute file paths. Generated manifests, reports, replays, checks
and captures remain ignored under `target/goalie-feat-trial`; isolated review
cards are under the ignored existing preview site's `goalie-feat-trial/` child.

Review: [trial cards](http://127.0.0.1:8780/goalie-feat-trial/) and
[four-season study](http://127.0.0.1:8780/goalie-feat-trial/study.html).
Recommended next disposition: review these as seasonal statistical feat additions,
keeping workload separate from saving quality. Contextual rebound/danger feats
still need their own opportunity and calibration decisions. This producer-applied
methods/goalie review is not independent expert validation. No source fetch,
default-registry promotion, grade/class change, hosted publication or merge occurred.
