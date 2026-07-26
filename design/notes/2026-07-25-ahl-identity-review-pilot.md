# Hartford / Coachella Valley AHL Identity Review Pilot

**Run date:** 2026-07-25 local / 2026-07-26 UTC  
**Seasons:** 2023–24, 2024–25, 2025–26  
**Review authority:** `IceLines exact-evidence pilot` at
`2026-07-26T00:20:47Z`

## Result

Official team-scoped AHL snapshots were sealed for Hartford and Coachella
Valley in all three seasons. Official NHL search and landing evidence produced
six snapshot-bound identity queues.

| Season | Affiliate | Roster | Exact reviewed | Alias | Conflict | Unmatched |
|---|---|---:|---:|---:|---:|---:|
| 2023–24 | Coachella Valley | 37 | 37 | 0 | 0 | 0 |
| 2024–25 | Coachella Valley | 46 | 43 | 2 | 1 | 0 |
| 2025–26 | Coachella Valley | 38 | 34 | 2 | 1 | 1 |
| 2023–24 | Hartford | 54 | 48 | 3 | 2 | 1 |
| 2024–25 | Hartford | 47 | 43 | 1 | 3 | 0 |
| 2025–26 | Hartford | 44 | 39 | 1 | 3 | 1 |
| **Total appearances** | | **266** | **244** | **9** | **10** | **3** |

The exact-only review accepted 244 of 266 roster appearances (91.7%). All 22
non-exact appearances remain pending. No ambiguous rows were found and no
non-exact row was reviewed accidentally.

## Unique manual exceptions

The 22 pending appearances reduce to 18 unique players:

- aliases: Benoit-Olivier Groulx / Bo Groulx, Cameron Hillis / Cam Hillis,
  J.R. Avon / Jon-Randall Avon, Josh Mahura / Joshua Mahura, Max Lajoie /
  Maxime Lajoie, Nate Knoepke / Nathan Knoepke, Phip Waugh / Philip Waugh,
  Tim Doherty / Timothy Doherty, and Zach Uens / Zachary Uens;
- birth-date conflicts: Bryce McConnell-Barker, Case McCarthy, Conor McCollum,
  Gavin Hain, Grant Gabriele, and Justin Janicke; and
- unmatched: Chris Cameron, Chris Ortiz, and Vince Stalletti.

These rows require separate evidence-by-evidence accept/remap/reject decisions.
They were deliberately excluded from the exact batch.

## Discovery proof

The three snapshots, six exact-reviewed crosswalks, and the sourced prospect
context were passed through `icecast prospect-league`. The resulting
`prospect_league_discovery.v1` artifact contained one eligible study, no
exclusions, and ranked Jagger Firkus as `injury_obscured_riser` in Hidden Gems
at 98.0. This proves the reviewed historical identity stream reaches the
canonical study and board primitives without provider-ID shortcuts.

## Expansion rule

League-wide expansion should repeat this exact-only pass first, publish the
coverage and unique exception queue, then review aliases/conflicts/unmatched
rows separately. A season/team is not "complete" until every context-relevant
player is reviewed or explicitly excluded.
