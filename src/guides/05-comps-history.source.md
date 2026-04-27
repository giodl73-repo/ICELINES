# Contract Comps and Historical Research

With 38 seasons of data available, IceLines can answer questions that require
historical context: what did comparable players sign for? How does this rookie
compare to past rookies at the same age?

---

## Similarity search

`query compare --similar` finds players with the most similar profile using
Z-score Euclidean distance on three dimensions:
- PPG (points per game)
- Goals per game
- Draft pedigree (pick number, inverted so #1 overall = highest)

The cohort is same position, age ±2 years.

```bash
icelines query compare "Matty Beniers" --similar 8
```

```
SIMILAR PLAYERS TO Matty Beniers (SEA · C · Age 24 · 2021 R1#2)
────────────────────────────────────────────────────────────────
Rank  Player            Team  Age  Draft      PPG    Similarity
   1  Mason McTavish    ANA   23   21 R1#3   0.547      77%
   2  Marco Rossi       VAN   25   20 R1#9   0.700      73%
   3  Dawson Mercer     NJD   25   20 R1#18  0.512      70%
   ...
Cohort: 79 C players aged 24±2.
```

---

## Historical comps (with data install)

With historical data installed, the similarity search draws from all seasons.
A player who signed a contract in 2018 can be a comp for someone signing in 2026
if their profiles at the same age match.

```bash
# Install data to enable historical comps
icelines data install --seasons 38

# Find historical comps for Beniers
icelines query compare "Beniers" --similar 10
# Will now include players from 1990s-2000s if their profile matches
```

### Contract comp workflow

Contract comps are the foundation of RFA/UFA negotiations. Both sides identify
4-5 comparable players who signed recently at a similar age and production level.

IceLines finds the comps. You then look up what those players signed for.

```bash
# Step 1: Find Beniers' comps
icelines query compare "Beniers" --similar 8

# Step 2: For each comp, check their career arc
icelines query player "Mason McTavish" --percentiles
icelines history "Marco Rossi"

# Step 3: What were they making at Beniers' age?
# → Look up on CapFriendly or PuckPedia (IceLines doesn't have cap hit data)
# → NHL public API doesn't expose salary/cap hit
```

**Cap hit note:** The NHL public API does not expose contract cap hit or AAV.
For salary data, use CapFriendly.com or PuckPedia.com alongside IceLines comps.

---

## Generational comparison

Compare players at the same age across eras:

```bash
# What were the top C scorers when McDavid was 22?
icelines query leaders --pos C --seasons 1 --top 10    # current
# Install and query 2018-19 season when McDavid was ~22
icelines data install --season 20182019
# (season-specific query coming in a future release)
```

---

## Multi-season research

```bash
# 10-year aggregate — sustained franchise players
icelines query leaders --seasons 10 --pos C --sort pts-pace --top 10

# Who improved the most this year vs last?
icelines query leaders --sort improvement --gp-min 40 --top 20

# Draft class performance over time
# 2015 class (McDavid, Eichel, Strome) — how are they doing?
icelines query leaders --draft-year 2015 --sort pts-pace --top 15

# 2022 class (Hutson, Slafkovsky, Cooley) — emerging?
icelines query leaders --draft-year 2022 --sort pts-pace --top 20
```

---

## Career arc analysis

```bash
# Full career history (up to 38 seasons with data install)
icelines history "Wayne Gretzky"     # if data installed back to 1987-88
icelines history "Nicklas Backstrom"
icelines history "Sidney Crosby"

# Deep profile with career arc + percentile
icelines query player "Connor McDavid" --percentiles
```

The career arc shows season-by-season with:
- Season label (e.g. "25-26")
- Team, GP, G, A, PPG, Pts/82
- Career weighted PPG and peak season

---

## Projection engine

Rest-of-season projection based on current pace, with age and career regression:

```bash
icelines project "Macklin Celebrini"                    # default: regressed
icelines project "Connor Bedard" --mode pace            # pure pace projection
icelines project "Nathan MacKinnon" --mode composite    # blended
icelines project "Celebrini" --games 20                 # override remaining games
```

Three modes:
- **pace** — pure current-season rate (alpha = 1.0)
- **regressed** — blends current pace with career average (older players regress more)
- **composite** — pace + age factor (peaks at 25-27, gradual decline after 28)

Output: current PPG, alpha (trust weight), age factor, projected points, confidence band.

---

## Scouting reports

Full 8-section reports suitable for trade deadline research:

```bash
icelines scouting "Evan Bouchard"
icelines scouting "Cale Makar" --format json   # structured JSON output
icelines scouting "Celebrini" --format markdown  # for docs/presentations
```

Sections:
1. Bio (age, nationality, draft, handedness)
2. Current season (all stats including PP, TOI, +/-)
3. Career trajectory (season-by-season table)
4. Peer group rank (vs same draft class ± 1 year)
5. Linemates (same-team position peers)
6. Depth chart position (line/pair rank on own team)
7. Cross-team value (what line would they play on average?)
8. Fit interpretation (regressed projection + assessment)
