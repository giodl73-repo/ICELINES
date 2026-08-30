# Felix's Five-Hole 2026-27 draft strategy

Updated 2026-08-28 from IceLines using the `dexters-dawgs` scoring scheme,
14-team positional replacement levels, saved Yahoo position eligibility, and
the exact 2026-27 NHL schedule. The active IceLines roster snapshot is
`20262027-2026-08-28-rosters`; all 32 NHL clubs are present and the sealed
snapshot passes integrity verification.

## Draft geometry

From slot 14 in a 14-team snake, the 16 roster picks are:

`14, 15, 42, 43, 70, 71, 98, 99, 126, 127, 154, 155, 182, 183, 210, 211`

## Projection contract

- 2025-26 IceLines fantasy points: 60% weight.
- 2024-25: 25%.
- 2023-24: 15%.
- Available seasons are re-normalized and scaled from 82 to the new 84-game
  2026-27 schedule.
- NHL.com's Aug. 26 top 250 is used as an availability and role signal, not as
  the scoring model. Its standard categories differ from Felix's Five-Hole.
- NHL rank is therefore an estimate of when the room might draft a player. The
  IceLines rank is how valuable that player is under our league's weights.
- The model does not yet make a numerical deduction for every current injury,
  suspension, or changed deployment. The risk flags below override the raw
  number until the final pre-draft refresh.

The complete draftable board is in
`reports/fantasy-draft-2026-27-projections.csv`.

## Opening turns

| Picks | Preferred pair | Expected FP | Why |
|---|---|---:|---|
| 14/15 | Nick Suzuki + Wyatt Johnston | 376.9 + 350.0 | Best custom-scoring pair expected to reach the turn; Johnston's C/RW eligibility prevents a center jam. MTL and DAL are in different schedule classes and share only 46 game dates. |
| 42/43 | Tim Stützle + Jake Oettinger | 337.3 + 343.3 | Stützle is a custom-scoring value and Oettinger is the last likely elite-volume goalie at this turn. If Hellebuyck or Sorokin falls, take the faller. If all three goalies are gone, use Scheifele, Keller, Connor, or Panarin and wait on goalie. |
| 70/71 | Filip Forsberg + Moritz Seider | 338.9 + 284.9 | Locks a premium LW and the first defenseman. Mika Zibanejad (327.9; public rank 66) is the premium replacement for either player and belongs at this turn, not 42/43. |
| 98/99 | Karel Vejmelka + Andrei Svechnikov | 330.0 + 284.3 | Vejmelka is a priority because this scoring rewards starts and saves. Svechnikov adds LW/RW flexibility. Ovechkin is the schedule-friendly wing pivot; Stamkos has more raw projected points but would compound a Nashville stack. |
| 126/127 | Travis Konecny + best D2 | 272.4 + TBD | Target McAvoy, Dobson, or Morrissey. McAvoy's 226.2 projection is conditional because he will miss the first six regular-season games. Geekie or Tuch is the forward fallback. |
| 154/155 | Juuse Saros + Josh Morrissey | 341.0 + 237.7 | Do not depend on Saros reaching 182. If he is present, take the custom-scoring bargain here and secure three playable goalies; otherwise use Morrissey/Dobson plus Dostal, Spencer Knight, or Ullmark as needed. |
| 182/183 | Tomas Hertl + best remaining value | 265.0 + TBD | If Saros somehow remains, he is an automatic selection. Otherwise use Hertl, Boeser, the best remaining defenseman, or a third starter if one was not secured at 154/155. |
| 210/211 | Kiefer Sherwood + Alexander Nikishin | 241.0 + 229.7 | Late peripherals/upside pair. John Gibson is the third-goalie pivot if he remains available. |

This is a turn map, not a rigid player list. At each turn, IceLines should rerun
the board with the actual taken-player paste and the roster already selected.

## First-turn decision tree

1. Take any unexpected faller from McDavid, Kucherov, MacKinnon, Draisaitl,
   Pastrnak, Robertson, Makar, or Vasilevskiy.
2. Otherwise take Nick Suzuki.
3. Pair him with Wyatt Johnston.
4. If Johnston is gone, use Martin Necas; if both are gone, take the best of
   Evan Bouchard, Cole Caufield, or the remaining elite goalie.

Do not force the named pair over an elite faller. The working first-turn faller
list is McDavid, Kucherov, MacKinnon, Draisaitl, Pastrnak, Celebrini, Robertson,
Makar, and Vasilevskiy.

## Position and goalie construction

- Leave the first turn with two elite skaters unless Vasilevskiy falls.
- Leave pick 99 with two goalies whenever possible. Oettinger plus Vejmelka is
  the ideal pair. The league requires at least two goalie appearances in the
  matchup week; missing the minimum forfeits all goalie points.
- Carry three goalies when the value appears. A third playable starter protects
  the weekly minimum from injuries, rest decisions, and sparse schedules, while
  the custom formula makes volume more valuable than standard rankings imply:
  five saves earn 1.0 point, while one goal allowed costs only 0.25.
- Oettinger and Vejmelka average six combined team-game opportunities per week
  on the 2026-27 schedule. They have at least two every week, but the week of
  2027-02-01 has only two, both belonging to Utah; a third goalie removes that
  single-team failure point. Team games are opportunities, not promised starts.
- Draft D1 by pick 71. Fill D2/D3 between picks 126 and 183; do not sacrifice a
  major forward or goalie bargain merely to fill defense early.
- Favor C/W or LW/RW eligibility. With only four weekly acquisitions, lineup
  flexibility prevents good games from being trapped on the bench.

Suzuki plus Johnston produces 122 distinct active dates from 168 combined
team-games, with 46 same-date overlaps (54.8%). Suzuki plus Necas has 48 overlaps
(57.1%), while Necas plus Johnston is the least attractive schedule pair of the
three because COL and DAL share 53 dates (63.1%) and occupy the same schedule
equivalence class.

## Schedule construction rules

- Prefer high-end players first; schedule is a tiebreaker for close values.
- Avoid stacking fringe players from the same schedule class, especially on the
  bench. Same-team skater/goalie pairs are less damaging because they occupy
  separate active slot families.
- The best full-season quiet-slate teams are NYR (18), COL/SJS/DET (17), WSH
  (16), and UTA/CHI/ANA (15).
- The tentative roster intentionally reaches NYR, DET, UTA, and SJS rather than
  concentrating only on traditional Tuesday/Thursday-heavy teams.
- Nashville has only five quiet-slate games in the current schedule analysis.
  Forsberg or Saros can each be a value, but avoid automatically adding Stamkos
  and building a large NSH stack.
- Configure the fantasy playoff start before the final board so IceLines can
  add legal playoff-lineup value to every candidate.

## Highest preliminary projections

| Rank | Player | Pos | Expected FP | NHL rank |
|---:|---|---|---:|---:|
| 1 | Connor McDavid | C | 469.2 | 1 |
| 2 | Nikita Kucherov | RW | 465.6 | 3 |
| 3 | Nathan MacKinnon | C | 458.6 | 2 |
| 4 | Leon Draisaitl | C/LW | 392.4 | 5 |
| 5 | David Pastrnak | RW | 387.4 | 6 |
| 6 | Macklin Celebrini | C | 382.2 | 4 |
| 7 | Nick Suzuki | C | 376.9 | 15 |
| 8 | Andrei Vasilevskiy | G | 368.9 | 13 |
| 9 | Jason Robertson | LW/RW | 362.1 | 9 |
| 10 | Connor Hellebuyck | G | 355.6 | 27 |
| 11 | Ilya Sorokin | G | 354.4 | 28 |
| 12 | Wyatt Johnston | C/RW | 350.0 | 16 |
| 13 | Jake Oettinger | G | 343.3 | 41 |
| 14 | Juuse Saros | G | 341.0 | 189 |
| 15 | Cale Makar | D | 340.8 | 8 |
| 16 | Martin Necas | RW | 340.4 | 14 |
| 17 | Filip Forsberg | LW | 338.9 | 67 |
| 18 | Mark Scheifele | C | 338.9 | 32 |
| 19 | Tim Stützle | C/LW | 337.3 | 50 |
| 20 | Evan Bouchard | D | 334.4 | 21 |
| 21 | Jeremy Swayman | G | 332.0 | 62 |
| 22 | Igor Shesterkin | G | 331.3 | 42 |
| 23 | Karel Vejmelka | G | 330.0 | 117 |
| 24 | Matt Boldy | LW/RW | 330.0 | 10 |
| 25 | Logan Thompson | G | 329.5 | 26 |

## Draft-room watch list

| Player | IceLines signal | Draft treatment |
|---|---|---|
| Juuse Saros | 341.0 FP, IceLines 14, NHL 189 | Largest goalie arbitrage; take him at 154/155 if available rather than depending on a slide to 182/183. |
| Karel Vejmelka | 330.0 FP, NHL 117 | Priority around 98/99; projected workload matters greatly here. |
| Filip Forsberg | 338.9 FP, NHL 67 | Strong target at 70/71, but account for Nashville's weak quiet-night profile. |
| Mika Zibanejad | 327.9 FP, NHL 66 | Let the standard-ranking room push him toward 70/71. |
| Moritz Seider | 284.9 FP, NHL 76 | Preferred D1 near 70/71 if Makar/Bouchard do not fall. |
| Charlie McAvoy | 226.2 FP, NHL 133 | Six-game suspension; draft only at a discount and use IR+ if Yahoo marks him eligible. |
| Kiefer Sherwood | 241.0 FP, NHL 220 | Late hits-driven upside around 210/211. |

Current injury/status holds: Connor Bedard, Seth Jarvis, Kevin Fiala, Troy
Terry, Owen Tippett, Alex Pietrangelo, and Filip Gustavsson require a fresh
status check before selection. This is a watch list, not an automatic do-not-
draft list.

## Pre-draft refresh gates

- Replace the April Yahoo export with the live 2026-27 player-pool export for
  current eligibility and Yahoo rank/ADP.
- Refresh all completed 32-in-32 player projections once NHL.com finishes the
  series.
- Apply current injury, suspension, goalie-role, line, and power-play evidence.
- Set the league's actual playoff calendar.
- Run a final availability simulation using current Yahoo ADP rather than only
  the NHL expert rank.

## Sources

- NHL.com, [Top 250 fantasy rankings, Aug. 26, 2026](https://www.nhl.com/news/topic/fantasy/nhl-fantasy-hockey-top-250-200-rankings-drafts-players-big-board-281505474)
- NHL.com, [Top 25 fantasy goalie rankings](https://www.nhl.com/news/topic/hockey-fights-cancer/nhl-fantasy-hockey-top-25-goaltender-rankings-pools-282860450)
- NHL.com, [Utah 2026-27 fantasy projections](https://www.nhl.com/news/topic/32-in-32/utah-mammoth-fantasy-projections-for-2026-27-season-32-in-32)
- NHL.com, [Charlie McAvoy six-game suspension](https://www.nhl.com/news/charlie-mcavoy-suspended-six-regular-season-games-for-actions-in-boston-bruins-game)
