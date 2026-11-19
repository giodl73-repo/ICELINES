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
in PUCK; the theme-studio cards have not yet been wired to these annotations.

Replay the existing salary adapter with its checksummed input manifest and new
output paths. Keep its former outputs intact for comparison. Source capture and
salary membership remain those of the prior frozen 2024–25 study; no new scrape
or current-roster inference is needed.
