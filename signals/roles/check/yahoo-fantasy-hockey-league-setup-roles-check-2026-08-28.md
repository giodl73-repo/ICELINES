---
skill: roles-check
topic: yahoo-fantasy-hockey-league-setup
date: 2026-08-28
roles_used: [pace, bench, edge, scout, glass, broadcast]
p1_count: 0
verdict: NEEDS-WORK
---

# Roles Check: Yahoo Fantasy Hockey League Setup Guide

## Artifact identification

- **Artifact:** `docs/guides/07-yahoo-fantasy-hockey-league-setup.md` at
  `origin/master` commit `1f5286f5`
- **Type:** public commissioner how-to / decision guide
- **Domain signals:** Yahoo configuration, numerical scoring rules, draft
  operations, hockey roster construction, browser readability, newcomer UX
- **Review depth:** standard

The guide was checked against Yahoo's current official help for Private League
defaults, player ranks, pre-draft rankings, roster changes, and draft behavior.

## Role selection

| Role | Why selected |
|---|---|
| PACE | The guide makes quantitative claims about scoring, team size, playoffs, and draft timing. |
| BENCH | The guide contains hardcoded settings and external links that need repeatable verification. |
| EDGE | Commissioners encounter odd manager counts, absent drafters, mobile-only users, and changed seasonal defaults. |
| SCOUT | Roster and scoring choices materially alter position scarcity and player value. |
| GLASS | The primary requirement is that a new commissioner can understand and act quickly. |
| broadcast | Kate will open a public GitHub/browser page, potentially on a phone and without repository context. |

HART, KEEL, TAPE, FORGE, WIRE, and CREST were not selected. This artifact does
not change the domain model, Rust code, data pipeline, API schema, or visual
identity. GLASS covers the limited presentation concerns more directly than
CREST.

## PACE review

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Seasonal values are called “current” without a last-verified date, so the statement will silently age. | P2 | League settings | Add “Verified against Yahoo Help on 2026-08-28” and tell commissioners to compare the live Settings page. |
| 2 | The custom tables show weights but not one worked example demonstrating stacked bonuses. | P3 | Points scoring | Show the total for one power-play goal and one representative goalie start under both systems. |
| 3 | “Six for most leagues; consider four for a small league” lacks an explicit team-count boundary or rationale. | P3 | League settings | Present six as Yahoo's default and make four versus six an explicit league vote based on how many teams should qualify. |

## BENCH review

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The scoring and roster values are only sourced in a reference list, making row-level drift hard to detect. | P2 | Roster and scoring tables | Add a short source/verification note directly above or below each Yahoo-standard table. |
| 2 | There is no repeatable validation showing that all links resolve and the page renders in the docs build. | P2 | Whole artifact | Add a link check and strict MkDocs build to documentation validation when tooling is available. |
| 3 | The checklist verifies the chosen column but does not ask the commissioner to compare the final Yahoo Settings screen with the guide. | P3 | Final checklist | Add a final side-by-side verification step and have another manager review it before draft day. |

## EDGE review

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The guide omits Yahoo's default minimum of three goalie appearances per weekly matchup. This can materially change matchup operation. | P2 | League settings | Add the goalie minimum, explain its effect, and make any deviation an announced league decision. |
| 2 | “Keep Yahoo defaults” is followed by a long table of recommendations, so a novice may change settings unnecessarily. | P2 | Quick setup / league settings | Add a self-contained 60-second quick path listing only the fields a new commissioner must touch. |
| 3 | The even-manager warning does not explain the consequence: Yahoo says an odd-manager Head-to-Head live draft may be forced to Autopick during the pre-draft check. | P3 | League size / draft | State the consequence next to the warning and link the official pre-draft help. |

## SCOUT review

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The custom scoring description understates how much special-teams bonuses and goalie volume can reorder player value. | P2 | Custom scoring | Add a plain-language “who gains value” note covering PP players, hitters/blockers, and high-save-volume goalies. |
| 2 | The roster comparison does not explain that 4 D versus 3 D plus Util changes defenseman scarcity during the draft. | P2 | Roster positions | Add one sentence describing the draft consequence, not merely the slot difference. |
| 3 | The missing goalie-start minimum leaves managers without guidance on whether carrying two playable goalies is operationally necessary. | P2 | League settings / roster | Add the minimum and connect it to goalie roster construction. |

## GLASS review

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The guide promises a quick path but still requires a newcomer to scan the entire advanced comparison. | P2 | Opening / whole artifact | Put a five-step “Fastest setup” block before the detailed sections and label everything after it optional detail. |
| 2 | The settings table contains long prose cells and the comparison tables are wide for phone viewing. | P2 | Tables | Shorten cells, move explanations below tables, and verify horizontal scrolling in the rendered site and GitHub mobile view. |
| 3 | The selected Yahoo-versus-Kraken path is not reinforced consistently at each decision point. | P3 | Roster / scoring / checklist | Use the same “Yahoo standard” and “Kraken custom” labels and add a one-line path marker at each section. |

## broadcast review

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A cold browser visitor is told IceLines is “only hosting” the guide, which adds repository context but does not help complete setup. | P2 | Opening | Remove the sentence from the user-facing introduction or move provenance to a footer. |
| 2 | The guide recommends desktop but gives no mobile fallback for someone who only has the Yahoo app or mobile browser. | P2 | Opening / create league | Say that basic creation works in the app, while missing commissioner tools require a mobile or desktop browser. |
| 3 | The public link targets GitHub source rather than a rendered documentation URL, limiting navigation and mobile control over large tables. | P3 | Distribution | Prefer the published MkDocs page when available; retain GitHub as a durable fallback. |

## Synthesis

```text
Roles reviewed: 6
P1 blockers: 0  |  P2 issues: 12  |  P3 notes: 6

Verdict: NEEDS-WORK

Top finding: The promised Yahoo-standard quick path is not yet a genuinely
short path for a first-time commissioner.

Cross-role consensus: EDGE and GLASS both found that the default route is
buried; PACE and BENCH both found insufficient drift/verification controls;
EDGE and SCOUT both flagged the missing goalie-appearance minimum.
```

The guide is directionally correct and substantially better than a single
prescriptive 14-team setup. The verdict reflects usability and operational
gaps, not a rejection of the two-path design.

## Amendments

1. Add a five-step, 60-second Yahoo-standard setup at the top. Move Kraken
   roster/scoring comparisons under clearly optional detail and remove IceLines
   provenance from the opening.
2. Add the weekly goalie-appearance minimum, the odd-manager Autopick
   consequence, and short explanations of how roster/scoring choices affect
   draft strategy.
3. Date-stamp Yahoo defaults, place source notes beside the affected tables,
   add a final live-settings comparison step, and validate links plus mobile
   rendering before treating the guide as approved.

## Authoritative references used

- [Yahoo default Fantasy Hockey settings](https://help.yahoo.com/kb/SLN6815.html)
- [Yahoo player-rank definitions](https://help.yahoo.com/kb/SLN6287.html)
- [Yahoo pre-draft rankings and Autopick behavior](https://help.yahoo.com/kb/fantasy-hockey/autopick-draft-sln6163.html)
- [Yahoo Private League setup](https://help.yahoo.com/kb/fantasy-hockey/create-customize-private-league-sln25711.html)
- [Yahoo roster-position changes](https://help.yahoo.com/kb/fantasy-hockey/sln6941.html)

