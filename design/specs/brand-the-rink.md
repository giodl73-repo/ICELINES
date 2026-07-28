# IceLines Brand Architecture — The Rink

**Version**: 0.1  
**Date**: 2026-07-19  
**Status**: Implemented vocabulary baseline
**Owner domain**: product language and navigation

---

## Brand Premise

IceLines is a digital hockey rink. The user enters one coherent place to study
players and teams, follow games, manage fantasy decisions, evaluate trades,
replay history, and simulate possible seasons.

The product language has four layers:

1. **IceLines** is the master brand.
2. **The Rink** is the navigational world.
3. **Ice…** names identify major tools and modes.
4. **The Insider** is the consistent explanatory voice.

Hockey expressions name reports and moments inside those layers. They add
character without replacing precise status, evidence, or accessibility copy.

## Core Brand Lines

Primary:

> **IceLines — The whole game, on Ice.**

Supporting:

> **Welcome to The Rink. The Insider has your Game Notes.**

> **Every line. Every player. Every possibility.**

The brand uses “lines” broadly: stat lines, forward/defensive lines, rink
markings, trend lines, scorelines, timelines, storylines, and simulated paths.

## The Rink Map

| Place | Stable meaning | Representative features |
|---|---|---|
| **Center Ice** | League home | dashboard, standings, scores, today's games |
| **The Red Line** | Offense | forwards, scoring, shooting, playmaking, power play |
| **The Blue Line** | Defense | defensemen, suppression, blocks, physical play, penalty kill |
| **The Crease** | Goalies | performance, workload, starter evidence, streams |
| **The Faceoff Circle** | Direct competition | matchups, comparisons, rivalries, game previews |
| **The Bench** | Roster decisions | fantasy lineups, draft, adds/drops, eligibility gaps |
| **The Penalty Box** | Availability and constraints | injuries, suspensions, waivers, locks, illegal moves, stale evidence |
| **The Boards** | Player movement | transactions, waiver wire, trade flow, deadline activity |
| **The Goal Line** | Possible outcomes | game probabilities, season forecast, playoff odds |
| **The Scoreboard** | Recorded outcomes | results, standings, records, streaks |
| **The Video Room** | Review | scouting film concepts, historical replay, calibration, misses |
| **The Press Box** | News and explanation | The Insider, Morning Skate, sourced reports, Game Notes |
| **The Locker Room** | People and roles | rosters, lines, chemistry, coaching, point-in-time personnel |
| **The Front Office** | Franchise control | roster construction, NHL trades, counterfactual seasons |

The map is conceptual, not a requirement to render a literal rink on every
surface. A visual rink may support navigation when it remains readable and
accessible.

## The Ice Product Family

| Product | Job | Rink home |
|---|---|---|
| **IceStats** | records, statistics, rankings | Scoreboard / Center Ice |
| **IceScout** | player and team analysis | Video Room |
| **IceTeams** | roster, depth, capability, ceiling | Locker Room / Red and Blue Lines |
| **IceGoalies** | complete goalie intelligence | Crease |
| **IceBench** | fantasy management | Bench |
| **IceTrade** | fantasy and NHL trade evaluation | Boards / Front Office |
| **IceCast** | game and season forecasting | Goal Line |
| **IceReplay** | historical rolling replay | Video Room |
| **IceLab** | explicit scenarios and counterfactuals | Front Office |
| **IceSignal** | alerts, opportunity, and evidence changes | Press Box |

Product names are user-facing families. Existing CLI commands, routes, JSON
fields, and schemas do not change merely to match brand copy. New aliases are
additive and must preserve script compatibility.

## The Insider

The IceLines Insider is a professional role, not an imitation of a real person
or an omniscient mascot. The Insider lives in the Press Box and explains what
matters across The Rink.

Voice qualities:

- informed, concise, and comfortable taking a position;
- explicit about probability and uncertainty;
- hockey-literate without excluding new fans;
- willing to say that evidence is missing or conflicting;
- clear about what can be executed now; and
- never presents a model event as reported news.

The Insider may speak from recognizable desks without becoming separate AI
personalities:

| Desk | Responsibility |
|---|---|
| **News Desk** | injuries, transactions, sourced updates |
| **Scout's Desk** | capability, development, role, fit |
| **Coach's Desk** | lines, starts, benches, matchups |
| **GM's Desk** | roster construction, trade packages, deadline strategy |
| **Forecast Desk** | game/season probabilities and factors |
| **History Desk** | replay, comparison, and calibration |

## Evidence Vocabulary

The Insider uses the same labels everywhere:

| Label | Meaning |
|---|---|
| **Confirmed** | fresh authoritative evidence |
| **Reported** | sourced report that is not final |
| **Estimated** | model-supported but uncertain |
| **Simulated** | occurred only inside an IceCast/IceLab run |
| **Under Review** | stale, incomplete, or conflicting evidence |
| **No Read** | insufficient evidence for a responsible claim |

Formal machine-readable state remains canonical. Personality copy cannot weaken
or contradict these labels.

## Canonical Hockey Reports

| Report name | Meaning |
|---|---|
| **The Depth Chart** | positional depth and roster capability |
| **The Apple Cart** | assists, playmaking, and setup value |
| **Upset the Apple Cart** | upsets, spoilers, and disruptive matchups |
| **Tale of the Tape** | player/team/goalie/trade comparison |
| **Morning Skate** | daily Insider briefing |
| **The Three Stars** | three most important actions or findings |
| **Game Notes** | concise matchup evidence and context |
| **Coach's Clipboard** | lineup and assignment recommendations |
| **Chalk Talk** | model explanation and factor attribution |
| **The Line Blender** | line-combination and roster experiments |
| **Tape to Tape** | chemistry and complementary capability |
| **The Point** | defensemen and blue-line offense |
| **The Slot** | high-danger opportunity |
| **Between the Pipes** | complete goalie report |
| **Who Gets the Net?** | starter evidence and final verification |
| **Top Shelf** | ceilings and upside |
| **Five-Hole** | exploitable weakness |
| **Waiver Wire** | acquisition market |
| **Hot Stove** | NHL trade market |
| **Bubble Watch** | playoff-border teams |
| **Cup Chase** | playoff and championship probabilities |
| **On a Heater** | winning/scoring streak |
| **Cold Snap** | slump or declining form |
| **The Gauntlet** | hardest schedule stretch |
| **Trap Game** | fatigue/look-ahead upset risk |
| **Final Horn** | result review and model learning |

## Strategy Vocabulary

Fantasy and simulation strategy maps to hockey language while retaining the
formal value in JSON:

| Formal value | Display name |
|---|---|
| `floor` | **Protect the Lead** |
| `balanced` | **Roll the Lines** |
| `upside` | **Pull the Goalie** |

## Penalty Box Rules

The Penalty Box contains blocked or constrained actions, not blame. Injury copy
must say “availability” rather than implying that an injured player committed a
penalty.

Recommended states:

- **Delayed Penalty** — a future lock, waiver, or restriction;
- **Under Review** — source conflict or stale evidence;
- **Game Misconduct** — action fully blocked by rules;
- **Back on the Ice** — constraint cleared.

Precise explanations always accompany the metaphor.

## Naming Guardrails

1. One hockey term has one stable product meaning.
2. Primary navigation pairs personality with a plain-language subtitle.
3. Obscure or insulting slang is optional flavor, never required comprehension.
4. Injuries, replacement players, and users are never mocked.
5. “Insider” does not imply unsourced private information.
6. Actual, reported, estimated, and simulated events remain unmistakable.
7. Existing commands and schemas remain backward compatible.
8. Screen readers receive literal purpose labels in addition to visual names.
9. The Rink metaphor supports the task; it does not distort hockey rules or data.
10. New names are reviewed against the canonical map before shipping.

## First Application Slice

Apply brand copy without changing behavior:

- `Morning Briefing` → **The Insider — Morning Skate**;
- `Goalie Plan` → **The Crease — Who Gets the Net?**;
- `Goalie Start Evidence` → **The Crease — Starter Evidence**;
- `Injury Plan` → **The Penalty Box — Availability Report**;
- season forecast/replay family → **IceCast** and **IceReplay**; and
- retain exact legacy CLI verbs and JSON contracts until additive aliases ship.

## Acceptance Criteria

1. Every major current/future feature has one Rink home and one literal purpose.
2. Ice product names do not replace public contracts without compatibility.
3. The Insider consistently distinguishes confirmed, reported, estimated,
   simulated, under-review, and no-read states.
4. Daily fantasy copy remains executable and does not hide locks, waivers,
   acquisition limits, or uncertainty behind slang.
5. Forecast and replay copy never presents simulated transactions or outcomes
   as actual news.
6. Accessibility labels state literal function even when visible copy uses a
   Rink or hockey expression.
