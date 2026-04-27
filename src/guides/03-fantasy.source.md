# Fantasy League Management

IceLines includes a complete fantasy hockey system: create leagues and teams,
add players, score against any scheme, simulate and execute trades.

---

## Setup (2 minutes)

```bash
# Create a league with Yahoo standard scoring
icelines fantasy league-create "My League" --scheme yahoo-standard

# Create teams
icelines fantasy team-create "Gio's Rangers" --owner "Gio"
icelines fantasy team-create "Hockey Nerds" --owner "Alex"

# Add players (fuzzy name matching)
icelines fantasy team-add "Gio's Rangers" "McDavid"
icelines fantasy team-add "Gio's Rangers" "Kucherov"
icelines fantasy team-add "Gio's Rangers" "Bouchard"
icelines fantasy team-add "Hockey Nerds" "MacKinnon"
icelines fantasy team-add "Hockey Nerds" "Rantanen"
```

Creating a league automatically sets it as active — subsequent commands
target it without needing `--league`.

---

## Check your team

```bash
icelines fantasy team-show "Gio's Rangers"
```

Output:

```
Roster: Gio's Rangers | League: My League | Scheme: yahoo-standard
────────────────────────────────────────────────────────────────────────
  #    Player                   Team  Pos  GP    Pts     Fantasy
────────────────────────────────────────────────────────────────────────
  1    Connor McDavid           EDM   C    82    138     361.0
  2    Nikita Kucherov          TBL   RW   76    130     331.5
  3    Evan Bouchard            EDM   D    82    95      234.5
────────────────────────────────────────────────────────────────────────
  Total fantasy score: 927.0
```

---

## League standings

```bash
icelines fantasy standings
```

```
STANDINGS — My League (yahoo-standard)
────────────────────────────────────────────────────────────
Rank  Team                   Owner            Score      Per/G
────────────────────────────────────────────────────────────
1     Gio's Rangers          Gio              927.0      3.86
2     Hockey Nerds           Alex             725.5      3.31
```

---

## Scoring schemes

IceLines ships with three built-in schemes:

```proof:tree kind=org indent-width=2
root: Scoring Schemes
- yahoo-standard (default)
  - goals: 3.0 · assists: 2.0
  - PPG bonus: 1.0 · hits: 0.5 · blocks: 0.5
- espn-standard
  - goals: 6.0 · assists: 4.0
  - PPG bonus: 2.0 · shots: 1.0 · plus-minus: 2.0
- simple-pts
  - goals: 1.0 · assists: 1.0 · no bonuses
```

Create a league with any scheme:

```bash
icelines fantasy league-create "ESPN League" --scheme espn-standard
```

Because IceLines tracks hits, blocks, and TOI in addition to scoring stats,
Yahoo/ESPN schemes that reward physical play work out of the box.
See `src/data/schemes.md` for complete weight tables.

---

## Trade simulation

Evaluate a trade before committing:

```bash
# Simulate — shows before/after for both teams, no changes made
icelines fantasy trade "Evan Bouchard" --to-team "Hockey Nerds" --for-player "Mikko Rantanen"
```

```
TRADE ANALYSIS — My League
────────────────────────────────────────────────────────────
  Evan Bouchard (Gio's Rangers)  <->  Zach Werenski (Hockey Nerds)
────────────────────────────────────────────────────────────
  Team                    BEFORE    AFTER     DELTA
────────────────────────────────────────────────────────────
  Gio's Rangers           927.0     890.5     -36.5
  Hockey Nerds            725.5     762.0     +36.5

(use --execute to commit this trade)
```

Execute the trade:

```bash
icelines fantasy trade "Evan Bouchard" --to-team "Hockey Nerds" --for-player "Zach Werenski" --execute
```

---

## Multiple leagues

IceLines supports multiple leagues. The most recently created league is active.
Switch between leagues:

```bash
icelines fantasy league-list               # see all leagues
icelines fantasy league-switch "ESPN League"   # or league-use
icelines fantasy standings                 # now shows ESPN League

# Most commands accept --league to target a specific league
icelines fantasy team-show "My Team" --league "My League"
```

---

## Web dashboard

Start a local HTTP server for a browser-based view:

```bash
icelines fantasy serve --port 8080
```

Available routes:
- `GET /` — HTML standings dashboard
- `GET /api/standings` — JSON standings
- `GET /api/team/:name/roster` — JSON team roster
- `POST /api/team/:name/add` — add player (body: `{"player": "name"}`)
- `POST /api/trade` — trade simulation (body: `{"player1": "...", "to_team": "...", "player2": "..."}`)

Share the URL with your league members for a live view.

---

## Finding waiver wire pickups

Use `query leaders` to find available players:

```bash
# High-pace players with limited GP (just returned from injury / recent callup)
icelines query leaders --gp-min 10 --gp-max 30 --sort pts-pace --top 15

# Undrafted sleepers with high production
icelines query leaders --undrafted --ppg-min 0.60 --sort ppg

# Rookies with strong starts
icelines query leaders --rookie --sort ppg --top 15
```

---

## Manage your roster

```bash
icelines fantasy team-drop "Gio's Rangers" "Bouchard"   # drop after trade
icelines fantasy league-delete "Test League"             # clean up test leagues
```

Deleting a league cascades — removes all teams and rosters in that league.

---

## Notes

- Goalies are not in the skater dataset — only forwards and defensemen
- Player uniqueness is scoped per-league — the same player can be on teams in different leagues
- Fantasy scores are cumulative season stats × scheme weights (not daily)
- For daily delta scoring, run `icelines fetch stats` each day and compare snapshots
