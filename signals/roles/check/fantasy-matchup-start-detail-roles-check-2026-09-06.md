---
skill: roles-check
topic: fantasy-matchup-start-detail
date: 2026-09-06
roles_used: 6
p1_count: 0
verdict: APPROVED
---

# Fantasy matchup start detail — role review

## Artifact identification

- Type: additive core JSON contract plus a private HTML/PDF consumer.
- Domain: fantasy lineup optimization, descriptive scoring rates, browser and print UX.
- Reviewed artifacts: `fantasy_matchup_strategy.rs` and PUCK's `New-FantasyMorningBrief.ps1`.

## Role selection

- HART: validates the new matchup projection shape.
- KEEL: ensures PUCK consumes the shared optimizer instead of recreating it.
- FORGE: reviews the Rust implementation and serialization boundary.
- PACE: reviews the points-per-start calculation and descriptive labeling.
- BENCH: requires exact reconciliation and a focused regression.
- GLASS/broadcast: reviews table hierarchy, browser behavior, accessibility, and print fallback.

## Review

| # | Role | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|---|
| 1 | HART | A start is date-coupled projection output, not player identity or season statistics. | P3 | `FantasyMatchupDailyStartRow` | Keep it nested under the daily projection. |
| 2 | HART | Slot, team, eligibility, and projected value retain the optimizer's exact assignment context. | P3 | start row fields | Preserve the typed `Position` vector. |
| 3 | HART | `serde(default)` keeps older serialized daily rows readable. | P3 | `starting_players` | Retain the compatibility default. |
| 4 | KEEL | PUCK consumes assignments emitted by IceLines rather than rerunning assignment logic. | P3 | report generator | Keep lineup optimization exclusively in core. |
| 5 | KEEL | The CLI JSON remains the one-shot exchange boundary between repositories. | P3 | matchup command | Do not add a PUCK runtime dependency on IceLines internals. |
| 6 | KEEL | The additive contract preserves existing consumers while enabling richer renderers. | P3 | JSON contract | Add fields rather than repurposing existing counts. |
| 7 | FORGE | The implementation clones only small owned output fields at the serialization boundary. | P3 | `project_team` | Keep the assignment row purpose-built and owned. |
| 8 | FORGE | Filtering uses the same game and availability predicates as `usable_starts`. | P3 | start collection | Do not create a second availability rule. |
| 9 | FORGE | No I/O or async behavior entered `icelines-core`. | P3 | crate boundary | Keep HTML rendering in PUCK. |
| 10 | PACE | Weekly points/start is projected points divided by usable starts for the same team and window. | P3 | summary | Continue guarding the zero-start denominator. |
| 11 | PACE | Daily points/start uses that day's projected points and usable starts, not scheduled player-games. | P3 | leverage table | Preserve the explicit “usable lineup” note. |
| 12 | PACE | The page calls the values descriptive projections and distinguishes them from future confirmed starts. | P3 | methodology copy | Re-run when current availability evidence changes. |
| 13 | BENCH | The focused unit test proves the selected starter, slot, and value match the optimized assignment. | P3 | core test | Keep this assertion with future optimizer changes. |
| 14 | BENCH | Generated Week 1 output reconciles 34 listed user starts to 34 usable starts and 34/34 for the opponent. | P3 | report validation | Fail report validation if either total diverges. |
| 15 | BENCH | PowerShell parsing, report generation, HTML markers, PDF generation, and `git diff --check` pass. | P3 | delivery | Retain these release checks. |
| 16 | GLASS/broadcast | Weekly total rate appears in the four-card summary before the denser tables. | P3 | matchup summary | Keep “you–them” in the label, not color alone. |
| 17 | GLASS/broadcast | Daily volume, cumulative volume, rates, and edge are readable in one semantic table with narrow-screen scrolling. | P3 | leverage table | Preserve the horizontal-scroll container. |
| 18 | GLASS/broadcast | The following semantic table lists modeled starters top-down and remains legible in landscape print. | P3 | starters table | Keep the explicit modeled-versus-confirmed caveat. |
| 19 | GLASS/broadcast | Gold `BENCH SUB` rows identify current Yahoo bench players used by the model on both sides without relying on color alone. | P3 | starters table | Retain the text badge and heavier player name. |
| 20 | BENCH | Bench history reads one newest roster export per dated capture, preventing repeated same-day downloads from inflating observations. | P3 | private capture history | Continue archiving one dated capture each morning. |

## Synthesis

Roles reviewed: 6  
P1 blockers: 0 | P2 issues: 0 | P3 notes: 20

Verdict: **APPROVED**

Top finding: the detailed player rows must be emitted by the same optimized lineup that supplies `usable_starts`.

Cross-role consensus: PACE, BENCH, KEEL, and GLASS agree that a readable table is trustworthy only when its player rows reconcile to the shared optimizer's counts and are labeled as descriptive modeled starts.

## Amendments applied

1. Added typed modeled-starter rows to each daily matchup projection and a focused core test.
2. Added weekly and daily points/start plus cumulative weekly start totals with zero-start handling.
3. Added a semantic, print-friendly modeled-starters table and explicit confirmation caveat.
4. Highlighted current Yahoo bench substitutes on both sides and added dated bench-observation history.
