# Rinkcraft descriptive attributes (CHG-127)

Protocol `rinkcraft.descriptive-attributes.0.1.0-experimental`. Implements the
independent Rinkcraft display protocol/decision 0007 after producer-applied role
review. This is descriptive measurement, not an empirically validated role model.

The profile v1 JSON gains additive `attribute_seasons` (default empty on older
records). CLI JSON, native player HTML/JSON and the resident WASM player dialog
use the same pure core builder. Native sources: canonical local loaded season
windows. WASM sources: resident prepared packages, preserving optional report gaps.

Each assessment identifies season/competition, historical roster-position group
(forwards, defense, goalies), GP, reference counts and all-strength scope. Each
attribute carries layer, raw value/unit/formula, interpretation, measured N,
midrank percentile, optional five-band display, suppression state and SHA-256 of
sorted reference IDs/exact values plus protocol/season/competition/group/metric.
Each player is counted once, including trade rows through canonical identity.

Provisional reference inclusion: skaters >=10 appearances, goalies >=5.
Minimum measured references: 20 skaters, 10 goalies. These are display filters,
not official NHL eligibility or validated reliability thresholds. Missing value,
insufficient exposure or small reference suppresses bands; observed zero survives.
References exclude missing/nonfinite metric values and expose measured/eligible N.

Percentile is 100*(below+0.5*equal)/N, including the assessed eligible player.
Bands are [0,20),[20,40),[40,60),[60,80),[80,100]. They mean more measured
activity/output, never a universal player quality level. Equal observations tie.
Use exact values for classification, not rounded display. Cohort changes require
an explicit protocol revision; no career percentile averaging.

Skaters: shots on goal/GP, assists/GP, goals/GP, recorded hits/GP, recorded blocks/GP,
catalog average TOI/GP, observed shooting percentage. Goalies: observed save
percentage, shots faced/appearance, season starts. No scoring/shot-quality model,
lineup bonus, style inference or finishing ability estimate is added.

All-strength, unadjusted comparisons confound manpower/deployment and other context;
display those limits alongside exposure. Statistical uncertainty intervals and
reliability are unimplemented, not inferred from GP. No five-on-five claim.

Render player-specific cards before generic family descriptions. Each card shows
five steps, raw rate/count, percentile, peer/exposure context, expandable meaning
and arithmetic, and seasonal band history. Charts have textual equivalents and
break across missing/unranked seasons. A band crossing is not a detected role change.
Style, assignment and persistent ability unknowns remain explicit.
