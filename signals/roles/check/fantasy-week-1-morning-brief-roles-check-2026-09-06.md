---
skill: roles-check
topic: fantasy-week-1-morning-brief
date: 2026-09-06
roles_used: 8
p1_count: 0
verdict: APPROVED
---

# Fantasy Week 1 morning brief — role review

## Artifact identification

- Type: Rust CLI resilience change plus a private PowerShell/HTML fantasy decision surface.
- Reviewed artifacts: `icelines-cli/src/commands/fantasy.rs` and PUCK's
  `New-FantasyMorningBrief.ps1`, league calendar input, rendered HTML, and PDF.
- Domain signals: fantasy scoring, schedule alignment, incomplete rookie history,
  lineup legality, goalie minimums, acquisition scarcity, browser UX, and privacy.

## Role selection

- HART: verifies that missing prior-season stats are not converted into invented canonical data.
- KEEL: verifies that IceLines remains the calculation engine and PUCK remains the private presentation layer.
- FORGE: reviews the Rust error/degradation path and ownership shape.
- PACE: reviews the descriptive projection, margin, and uncertainty language.
- BENCH: requires a regression test for the original rookie failure.
- EDGE: checks missing players, invalid matchup weeks, missing calendar data, and empty states.
- SCOUT: checks that rookie and deployment uncertainty is represented as hockey context.
- GLASS/broadcast: checks scan order, positions, faces, accessibility, and browser/print behavior.

## Review findings

### HART

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A 2026–27 roster identity can legitimately lack a 2025–26 `SeasonStats` row. | P2 | IceLines matchup adapter | Omit it from the descriptive rate pool; never synthesize a stats row. Implemented. |
| 2 | Resolvability is evaluated against the selected completed-season pool, preserving the season/type axis. | P3 | `resolved_player_keys` | Keep the check local to the matchup adapter. |
| 3 | The change does not alter player identity, eligibility, roster persistence, or repository invariants. | P3 | Scope | No model migration is warranted. |

### KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Re-deriving matchup scoring in PUCK would create a second engine. | P2 | PUCK report generation | Invoke `fantasy matchup-plan --json`; implemented. |
| 2 | Yahoo matchup assignment and faces are private manager context, appropriately retained in PUCK. | P3 | Layer boundary | Keep raw Yahoo exports ignored and emit only private reports. |
| 3 | The IceLines CLI JSON remains the shared matchup contract; PUCK only renders it. | P3 | Cross-surface flow | Preserve the saved JSON beside each report for diagnosis. |

### FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The previous `with_context(...)?` made one missing rookie fatal to the whole report. | P2 | `build_team` | Use deterministic `filter_map` degradation plus an explicit warning; implemented. |
| 2 | The helper accepts borrowed rosters and a resolved-key set without new I/O or heavy cloning. | P3 | `unresolved_matchup_roster_players` | Retain the pure helper boundary. |
| 3 | No unwrap, unsafe code, new dependency, or cross-thread state was introduced. | P3 | Rust diff | Accept. |

### PACE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A -26.9 margin omitting Ivar Stenberg could be mistaken for a complete 2026–27 forecast. | P2 | Matchup headline | Label values descriptive and show the rookie omission above the fold; implemented. |
| 2 | Completed-season per-game rates are a baseline, not a claim about rookie development or current deployment. | P3 | Diagnostic copy | Keep the distinction visible and retain detailed warnings. |
| 3 | The report does not elevate the model's logistic win probability into betting-like certainty. | P3 | Matchup summary | Continue showing margin bands and usable starts instead. |

### BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The original regression was an all-or-nothing failure on a roster with one unresolved rookie. | P2 | CLI unit coverage | Add a fixture proving the rookie is named and omitted while known players resolve; implemented. |
| 2 | PowerShell syntax and league-settings JSON require deterministic validation. | P3 | PUCK validation | Parse both and run `git diff --check`. |
| 3 | The rendered browser result is part of the acceptance evidence. | P3 | Visual validation | Regenerate HTML/PDF and inspect a 1400px screenshot. |

### EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A missing or zero Yahoo week could silently map before Week 1. | P2 | Week mapping | Reject weeks below 1 and missing `week_1_start`; implemented. |
| 2 | A player absent from historical stats must not suppress the opponent or daily schedule rows. | P3 | Matchup degradation | Continue with all resolvable players and name every omission. |
| 3 | A future empty/missing matchup should leave a useful generic daily checklist. | P3 | Empty state | Keep matchup rendering conditional and the routine independent. |

### SCOUT

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Stenberg and Martone require deployment review rather than a fabricated veteran rate. | P2 | Daily watch item | Name rookie deployment as a preseason check; implemented. |
| 2 | Equal usable starts mean Week 1 is driven by scoring quality, goalie execution, and collision management. | P3 | Diagnosis | State that volume alone does not close the baseline gap. |
| 3 | Saturday's collision list is more actionable than a weekly aggregate alone. | P3 | Daily leverage | Surface the exact date and modeled bench names. |

### GLASS / broadcast

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Opponent cards initially displayed Yahoo roster slots such as `BN` instead of player eligibility. | P2 | Threat cards | Render eligible hockey positions; implemented. |
| 2 | The decision order should be action first, matchup second, lineup third, supporting tables last. | P3 | Page hierarchy | Implemented in the HTML and landscape PDF. |
| 3 | Color is not the sole encoding: margins have signs/text, faces have names/alt text, and tables have headers. | P3 | Accessibility | Accept; retain focus states and responsive one-column collapse. |

## Synthesis

Roles reviewed: 8  
P1 blockers: 0 | P2 issues raised: 8, resolved: 8 | P3 notes: 16

Verdict: **APPROVED**

Top finding: the matchup baseline must continue when a rookie lacks prior-season
NHL statistics, but the undercount must be explicit rather than replaced with
invented production.

Cross-role consensus: HART, FORGE, PACE, EDGE, and SCOUT agree on omission plus
visible disclosure; KEEL requires the calculation to remain in IceLines; GLASS
requires the limitation to appear near the headline rather than only in logs.

## Amendments completed

1. Replaced fatal missing-player resolution with deterministic omission and a named warning in IceLines.
2. Added explicit week-calendar validation, rookie caveat, dynamic collision/player names, and eligible positions in PUCK.
3. Added a focused Rust regression test and validated the generated browser/print artifact.

