---
skill: roles-check
topic: fantasy-replacement-lookahead
date: 2026-09-06
roles_used: 9
p1_count: 0
verdict: APPROVED
---

# Fantasy replacement lookahead — role review

## Artifact identification

- Type: additive core view model, CLI command, and private HTML/PDF consumer.
- Domain: fantasy add/drop optimization, exact daily lineup capacity, quiet-night coverage, and acquisition restraint.
- Reviewed artifacts: `fantasy_replacement_lookahead.rs`, fantasy CLI dispatch, and PUCK's `New-FantasyMorningBrief.ps1`.

## Role selection

- HART: model and season-axis integrity.
- KEEL: IceLines/PUCK responsibility boundary.
- FORGE: Rust ownership and error behavior.
- PACE: scoring rate, weighting, threshold, and runtime claims.
- BENCH: regression and end-to-end evidence.
- EDGE: injury, waiver, zero-budget, and provisional-player failure modes.
- SCOUT: roster construction and goalie reasonableness.
- GLASS: dense-table hierarchy and non-color encoding.
- broadcast: browser, print, and narrow-screen behavior.

## Review

| # | Role | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|---|
| 1 | HART | Candidate scoring is explicitly coupled to the requested completed stats season. | P3 | CLI loader | Keep the stats season in the command contract. |
| 2 | HART | Current-team evidence is resolved against the requested schedule season. | P3 | player assembly | Do not silently reuse a prior-season team. |
| 3 | HART | The output owns stable player identity keys while positions remain typed. | P3 | core rows | Preserve keys and `Position` values in JSON. |
| 4 | KEEL | IceLines owns all schedule, scoring, and legal-lineup calculations. | P3 | architecture | Keep PUCK limited to private inputs and rendering. |
| 5 | KEEL | PUCK consumes one additive JSON contract without linking IceLines internals. | P3 | report generator | Retain the CLI exchange boundary. |
| 6 | KEEL | The same core lineup optimizer supplies baseline and replacement results. | P3 | simulation | Do not reimplement slot logic in PowerShell. |
| 7 | FORGE | Core remains deterministic and I/O-free. | P3 | core module | Keep loading and candidate discovery in CLI. |
| 8 | FORGE | Input errors identify missing drop targets and invalid horizons. | P3 | validation | Preserve fail-fast validation. |
| 9 | FORGE | Owned output rows avoid repository-borrow leakage. | P3 | public API | Continue converting at the serialization boundary. |
| 10 | PACE | Each option reruns exact legal lineups across all 21 days instead of adding raw team games. | P3 | weekly simulation | Keep starts and points deltas tied to active assignments. |
| 11 | PACE | Week 1 receives the highest weight, with later weeks discounted and quiet starts explicit. | P3 | weighted score | Keep per-week raw deltas visible beside the rank. |
| 12 | PACE | Completed-season per-game values are labeled descriptive and candidates below 20 GP are excluded from the ordinary shortlist. | P3 | disclosures | Treat emerging-player upside as a separate watch lane. |
| 13 | BENCH | The focused core test proves a replacement creates one legal quiet-night start and its exact point delta. | P3 | unit test | Keep this regression with optimizer changes. |
| 14 | BENCH | CLI compile, real-roster execution, PowerShell parse, JSON generation, HTML generation, and PDF generation pass. | P3 | integration | Retain these release checks. |
| 15 | BENCH | Real output reports one acquisition remaining and distinct add alternatives. | P3 | fixture evidence | Recheck against each new Yahoo capture. |
| 16 | EDGE | Zero remaining acquisitions produces a blocked posture without suppressing the future watch list. | P3 | posture | Preserve the explicit blocked state. |
| 17 | EDGE | Injury replacements are marked only when the explicit injury target is also a legal drop target. | P3 | target validation | Never infer an injury solely from historical stats. |
| 18 | EDGE | Current IR/IR+ occupants are excluded from ordinary drops while newly injured active/bench players can enter the injury lane. | P3 | PUCK target selection | Continue reading live Yahoo status before each run. |
| 19 | SCOUT | A skater-drop analysis cannot recommend a third goalie from unconfirmed schedule appearances. | P3 | candidate guard | Require an explicit goalie drop before evaluating goalies. |
| 20 | SCOUT | Forward and defense replacements may compete for a bench slot because the exact roster optimizer tests their legal utility. | P3 | candidate pool | Keep the proposed drop visible for roster-balance judgment. |
| 21 | SCOUT | Role, deployment, and injury-return uncertainty remain outside the descriptive score. | P3 | methodology | Confirm live hockey context before acting. |
| 22 | GLASS | The table leads with add, drop, and posture before numeric detail. | P3 | replacement table | Preserve the decision-first column order. |
| 23 | GLASS | Week 1, 2, 3, and total deltas are readable without decoding the weighted score. | P3 | replacement table | Keep the internal score out of the main report. |
| 24 | GLASS | Text posture accompanies the gold/green row treatment. | P3 | CSS | Do not rely on color alone. |
| 25 | broadcast | Player names remain links to canonical IceLines pages. | P3 | browser table | Preserve keyboard-visible focus behavior. |
| 26 | broadcast | The wide table is inside the existing horizontal-scroll container. | P3 | responsive layout | Retain the minimum table width and mobile overflow. |
| 27 | broadcast | The visual review shows the replacement panel between matchup evidence and the lineup board, and landscape PDF generation succeeds. | P3 | report hierarchy | Keep the decision evidence before the decorative formation. |

## Synthesis

Roles reviewed: 9  
P1 blockers: 0 | P2 issues: 0 | P3 notes: 27

Verdict: **APPROVED**

Top finding: acquisition advice is trustworthy only when the same add/drop is
replayed through exact legal daily lineups and the current weekly budget is
shown alongside the result.

Cross-role consensus: PACE, EDGE, SCOUT, BENCH, and GLASS agree that the report
must separate a quantified opportunity from permission to transact. A
threshold-clearing row means review now, not automatic add.

## Amendments applied

1. Added explicit current-budget postures and Week 1/2/3 deltas.
2. Restricted goalie candidates to explicit goalie replacements.
3. Excluded sub-20-game candidates from the ordinary replacement shortlist and
   disclosed the candidate policy.
4. Deduplicated CLI output to the best drop fit per available player.
5. Added injury-target validation, live-platform caveats, and non-color posture
   labels.
