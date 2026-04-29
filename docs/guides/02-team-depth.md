# Team Depth Charts

`icelines team <ABBR>` renders a team's current roster as a 4×3 forward grid
and 3×2 defense grid, with each player color-coded by their fit vs the rest of the league.

---

## Basic usage

```bash
icelines team SEA      # Seattle Kraken
icelines team EDM      # Edmonton Oilers
icelines team NYR      # New York Rangers
```

Team abbreviations are case-insensitive: `sea`, `SEA`, `Sea` all work.

---

## The lineup card

```
SEA — 2025-26
FORWARDS
+--------+------------------+------------------+------------------+
| Line 1 | LW               | C                | RW               |
+--------+------------------+------------------+------------------+
|   L1   | Jared McCann★    | Chandler Steph~  | Jordan Eberle~   |
|        | 73.5 pts/82      | 89.0 pts/82      | 74.4 pts/82      |
+--------+------------------+------------------+------------------+
...

DEFENSE
+--------+---------------------+---------------------+
| Pair 1 | LD                  | RD                  |
+--------+---------------------+---------------------+
|   P1   | Vince Dunn★         | Adam Larsson~       |
|        | 55.7 pts/82         | 32.8 pts/82         |
...

Additional (8):
  Freddy Gaudreau          C  68gp  0.35
  Ben Meyers               C  52gp  0.29
  ...
```

---

## Fit classification

Each player is classified by comparing their pace score to where they'd rank
on each of the other 31 NHL teams at their position.

```
<!-- proof:compiled from="proof:tree kind=org" uri="" -->
```org
Fit Class
├── ★ Elite (green)
├── condition: avg rank elsewhere ≤ own line + 0.5
├── meaning: true caliber for this slot on most rosters
├── ~ Solid (yellow)
├── condition: avg rank elsewhere ≤ own line + 1.25
├── meaning: fits their role, slight upgrade elsewhere
├── ↑ Buried (blue)
├── condition: avg rank elsewhere < own line − 0.75
├── meaning: underused — would play a higher line on most teams
├── ↓ Stretch (red)
├── condition: avg rank elsewhere > own line + 1.25
└── meaning: overextended — playing above their talent level
```
<!-- /proof:compiled -->
```

**Example:** A player who's a 3rd-liner on their own team but would be a 1st-liner
on 20 other teams is classified as "Buried" (↑ blue).

---

## Scoring methodology

Rankings use **pace-adjusted stats** to be fair to players who've played fewer games:

```
pts/82 = (goals + assists) / GP × 82
```

Goals-per-82 serves as tiebreaker. This means a player with 60 pts in 40 games
ranks the same as a player with 120 pts in 82 games — both project to 120 pts/82.

Players with fewer than 10 GP are listed in "Below min GP" and excluded
from fit calculations.

---

## Additional and unplaced players

The depth card shows 4 forward lines (12 players) and 3 defense pairs (6 players).
Extra players appear in the "Additional" section with their position, GP, and PPG:

```
Additional (8):
  Freddy Gaudreau          C  68gp  0.35
  Ben Meyers               C  52gp  0.29
```

"Below min GP" shows players with 1–9 games who can't be pace-projected:

```
Below min GP (2):
  John Hayden              C  3gp
```

---

## Trade impact analysis

See the lineup before and after a proposed trade:

```bash
# How would trading Miro Heiskanen affect the Stars?
icelines trade "Heiskanen" for "Ekman-Larsson" --team DAL
```

Shows BEFORE/AFTER forward lines and defense pairs, plus the pts/82 delta.

---

## Related commands

```bash
icelines rank --pos D --top 20           # best D across the league
icelines query leaders --team SEA        # all Kraken players ranked
icelines query player "Matty Beniers"    # deep profile on one player
icelines scouting "Jared McCann"         # full 8-section scouting report
```
