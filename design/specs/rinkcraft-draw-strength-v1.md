# Faceoff manpower partitions v1

The existing **Contested draws** response expands into five player-relative
categories. Its aggregate and the separate late-close score-state response retain
their original meanings. This adds context, not a feat or capability rating.

| Category | Definition |
|---|---|
| Even skaters | Both goalies reported present, each team has 3–5 skaters, counts equal |
| Skater advantage | Same goalie/count conditions, player's team has more skaters |
| Skater disadvantage | Same conditions, player's team has fewer skaters |
| Other / empty net | Valid code with an absent goalie or six reported skaters |
| Unknown strength | Missing or malformed code, or unsupported goalie/skater count |

Decode the NHL event situation code as away goalie, away skaters, home skaters,
home goalie. Resolve the actor's team from the admitted roster independently of
who won. A single contest therefore gives opposite advantage/disadvantage
contexts to the two participants. Equal categories include 3v3 and 4v4, not just
5v5. Empty-net situations are not relabeled power plays.

Each row has credited wins / contests and a win percentage. No opportunities means
**No observed opportunities**, not 0%. Retain unknown and other contests so row
counts and wins reconcile exactly to the all-situations aggregate. A credited win
does not establish a clean win, retained possession or downstream scoring value.

`Faceoffs.strengths` is optional, defaulting to absent for old samples. When present,
only the five known keys are admitted, wins cannot exceed contests, and both sums
must match the parent. Core rebuilds display rows and rejects altered projections.
The existing breakdown DTO and native/browser presentation carry the rows; the
heading distinguishes manpower from the late-close score-state breakdown.

The offline cache importer and normal refresh aggregate the same parser output.
Source captures retain their dates and hashes. No live fetch, full-season coverage
claim, minimum-opportunity gate, change to existing grades or simulation rule is
introduced. Print keeps the compact response and hides expanded details.
