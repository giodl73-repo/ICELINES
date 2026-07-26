# Hartford / Coachella Valley AHL Identity Review Pilot

**Run date:** 2026-07-25 local / 2026-07-26 UTC  
**Seasons:** 2023–24, 2024–25, 2025–26  
**Review authorities:** `IceLines exact-evidence pilot`, `IceLines
alias-evidence pilot`, `IceLines conflict-evidence pilot`, and `IceLines
exception-evidence pilot`, all at `2026-07-26T00:20:47Z`

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

The exact-only review accepted 244 of 266 roster appearances (91.7%). The
evidence-by-evidence pass then approved all nine alias appearances, adjudicated
nine of ten birth-conflict appearances while preserving both provider dates,
and explicitly rejected three NHL mappings. The final state is 262 reviewed,
three rejected, and one pending appearance: 99.6% of rows are resolved and
98.5% have canonical NHL identities. No ambiguous rows were found.

## Unique manual exceptions

The 22 pending appearances reduce to 18 unique players:

- aliases: Benoit-Olivier Groulx / Bo Groulx, Cameron Hillis / Cam Hillis,
  J.R. Avon / Jon-Randall Avon, Josh Mahura / Joshua Mahura, Max Lajoie /
  Maxime Lajoie, Nate Knoepke / Nathan Knoepke, Phip Waugh / Philip Waugh,
  Tim Doherty / Timothy Doherty, and Zach Uens / Zachary Uens;
- birth-date conflicts: Bryce McConnell-Barker, Case McCarthy, Conor McCollum,
  Gavin Hain, Grant Gabriele, and Justin Janicke; and
- unmatched: Chris Cameron, Chris Ortiz, and Vince Stalletti.

These rows were deliberately excluded from the exact batch. The alias pass
revalidated shared surnames, equal birth dates, canonical IDs, and retained
official evidence before applying explicit remaps. Exact-name conflict reviews
retained the disagreeing AHL and NHL dates rather than laundering them into
exact matches. Conor McCollum remains pending because the AHL date, NHL date,
and additional public evidence disagree.

Chris Cameron and Chris Ortiz were rejected only as NHL identity mappings:
official club evidence supports them as legitimate AHL-only players for which
the NHL-linked adapter has no canonical ID. Vince Stalletti was rejected as a
non-player after official team evidence identified him as coaching staff. The
AHL provider had placed him in the `Goalies` roster section with a player ID,
position, and jersey, so IceLines did not add a brittle role/name/age parser
exception that could hide legitimate zero-game players.

## Discovery proof

The three snapshots, six final crosswalks, and the sourced prospect context
were passed through `icecast prospect-league` after exception adjudication.
The resulting `prospect_league_discovery.v1` artifact contained one eligible
study, no exclusions, and ranked Jagger Firkus as `injury_obscured_riser` in
Hidden Gems at 98.0. This proves the reviewed historical identity stream
reaches the canonical study and board primitives without provider-ID
shortcuts.

## League coverage proof

The new `icecast affiliate-identities-league` acquisition surface was first
replayed against the sealed 2025–26 snapshot. One cache-first run produced two
child queues covering 82 roster appearances and 82 unique AHL provider players:
Coachella Valley retained 34 exact proposals, two aliases, one conflict, and one
unmatched row; Hartford retained 39 exact proposals, one alias, three conflicts,
and one unmatched row. All 82 rows remained pending, proving batch acquisition
does not inherit review authority.

The six final crosswalks were also composed through
`icecast affiliate-review-league`. The UI-neutral league board independently
recomputed 266 appearances, 262 reviewed mappings, three explicit rejections,
and one pending conflict: 99.62% resolved and 98.50% canonical-identity
coverage. Its four deterministic attention groups are Chris Cameron, Chris
Ortiz, Conor McCollum, and Vince Stalletti. Each now carries two structured
evidence URLs; exception authority is no longer available only by parsing the
review note.

## Expansion rule

League-wide expansion should repeat the exact pass first, publish the coverage
and unique exception queue, then use the narrow alias and rejection surfaces
plus reviewer-authored conflict batches. A season/team is not "complete" until
every context-relevant player is reviewed or explicitly excluded. The pilot's
one deliberately pending identity demonstrates that unresolved evidence stays
visible instead of being forced through for nominal 100% coverage.
