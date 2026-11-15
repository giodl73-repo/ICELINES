# Shooting and goalie Tracking in live shot chains

Episode/draft/result 0.5 adds `resolve_shot`. First release a deliberate shot;
optional post/tip contacts may precede resolution. Bind a verified skater shooter
and opposing active goalie. The shooter must match the attempt's original
deliberate release, not an arbitrary actor or the latest incidental tipper.
No phantom goalie or empty-net rule is supplied. Existing CLI: `build DRAFT
REQUEST`, then `REQUEST RESULT`.

Shooting versus Tracking uses the existing selected d20 rule. Strict shooter win
opens an authored `finish` distribution (goal, post, loose). A tie leaves loose
play without inventing goalie contact. A goalie win records physical contact,
then samples authored `containment` (hold or rebound). Both distributions must
contain distinct positive outcomes totaling 10000, including unreached branches.
The profile changes contest odds; these physical distributions remain explicit
parameters, not a newly calibrated model of post hits or rebound control.

Only an actual GoalEvent changes score, through the same penalty/goal reducer as
legacy shots. Post contact leaves the original attempt in flight; tips preserve
it. A goalie touch does not automatically award possession, a whistle or an
official save. Rebounds remain loose; collecting and deliberately shooting again
creates a linked new attempt. Holds whistle and use existing restart/penalty
service. A touch alone does not stop the supported delayed-call path.

Six neutral drafts: baseline, Shooting 16, Tracking 16, unmapped Rebounds 16,
post-chain and tip-chain. Shared seed 20261115; score 10 elsewhere. Conditional
finish is 50% goal/25% post/25% loose; containment is 40% hold/60% rebound. These
are provisional game parameters, not NHL rates. Exact conditional goal chances
are 23.75%, 30.875%, 17%, 23.75% for the first four builds. This is a declared
shot/goalie encounter, not every NHL shot or a full scoring-rate calibration.

Tracking 16 increases goalie contacts and thus the expected count of rebounds,
while the conditional rebound share stays 60%. More rebound events alone do not
establish poorer rebound control. Rebounds/Positioning/Movement/Recovery/Puck Play
are not silently folded into this Tracking contest. Their task maps remain open.
No official SOG/save/scorer/assist attribution, new measured rating or class bonus.
Old 0.1–0.4 outputs must remain byte-identical.
