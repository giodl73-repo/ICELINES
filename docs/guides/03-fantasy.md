# Fantasy League Management

IceLines includes a complete fantasy hockey system: create leagues and teams,
add players, score skaters and goalies against any scheme, find roster gaps,
simulate add/drop scenarios, and execute trades.

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

## Start each fantasy day here

The season cockpit gives one prioritized decision, the next goalie or lineup
deadline, legal lineup and reserve context, acquisition headroom, quiet-night
bench coverage, and explicit source readiness:

```bash
icelines fantasy today
icelines fantasy today --json
icelines tui fantasy
```

The default path is advisory and read-only. It uses the existing fantasy
database, a sealed player-rate sample, and the cached official NHL schedule; it
does not fetch, add, drop, start, bench, waive, or trade. If required local
evidence is absent, the output names a recovery command instead of treating the
missing value as zero. Deep pickup and sleeper searches remain separate because
they are slower and may consume refreshed inputs.

The default surfaces use the league-aware `fantasy_today.v2` contract: CLI
text/JSON, the TUI Fantasy workbench, browser HTML at `/fantasy/today`, and JSON
at `/api/v2/fantasy/today`. The stable `/api/v1/fantasy/today` route remains a
compatibility projection. PUCK and other private consumers can follow the
[v2 handoff contract](../contracts/fantasy-today-v2.md) without storing private
league preferences in IceLines.

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

## Roster gaps

Mark your team as the roster IceLines should evaluate, then ask which categories
need help:

```bash
icelines fantasy team-use "Gio's Rangers"
icelines fantasy gaps --category hits,blocks,shots
icelines fantasy gaps --category hits,blocks,shots --json
```

The gap view compares your roster against imported available players, applies
the active fantasy scheme weights, and returns `add_now`, `watch`, or
`no_action` recommendations. The same `FantasyRosterGapView` powers CLI text,
TUI fantasy gaps, web HTML, and `/api/v1/fantasy/gaps`.

---

## Season simulation and scenarios

Project the active league over a schedule horizon:

```bash
icelines fantasy simulate --weeks 4
icelines fantasy simulate --weeks 4 --json
```

Test add/drop decisions without mutating the league:

```bash
icelines fantasy simulate --add "Connor McDavid" --drop "Evan Bouchard"
icelines fantasy simulate --add "Connor McDavid"
icelines fantasy simulate --drop "Evan Bouchard"
```

Scenario players are resolved to canonical names before projection. Invalid
drops are rejected explicitly instead of producing a misleading projected
roster. The same `FantasySimulationView` powers CLI text/JSON, TUI simulation,
web `/fantasy`, and `/api/v1/fantasy/simulate`.

---

## Scoring schemes

IceLines ships with three built-in schemes:

<!-- proof:compiled from="proof:tree kind=org" uri="" -->
```org
Scoring Schemes
├── yahoo-standard (default)
  ├── goals: 3.0 · assists: 2.0
  ├── PPG bonus: 1.0 · hits: 0.5 · blocks: 0.5
├── espn-standard
  ├── goals: 6.0 · assists: 4.0
  ├── PPG bonus: 2.0 · shots: 1.0 · plus-minus: 2.0
└── simple-pts
  └── goals: 1.0 · assists: 1.0 · no bonuses
```
<!-- /proof:compiled -->

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

Start the local web dashboard and open `/dashboard` for the Jack Adams browser
bench: scores ribbon, Favorites/watchlist pane, central workspace,
Schedule/context pane, and command bar.

```bash
icelines serve --port 8000
```

Available routes:
- `GET /dashboard` - multi-pane browser dashboard with workspace command bar
- `GET /fantasy` - HTML roster gaps and simulation scenarios
- `GET /api/v1/fantasy/gaps` - JSON `FantasyRosterGapView`
- `GET /api/v1/fantasy/simulate` - JSON `FantasySimulationView`
- `GET /poach` - HTML poacher board
- `GET /api/v1/poach` - JSON `PoachBoardView`

Command examples inside `/dashboard`:

```text
gaps cats=hits,blocks,shots top=8
poach rw cats=hits,blocks free top=12
fantasy poach top=8 available
fantasy simulate add Connor_McDavid drop Bench_Forward
report weekly cats=shots,hits top=12
```

`icelines fantasy serve --port 8080` remains available for the local fantasy
server workflow, but the main dashboard is the parity surface for fantasy
read/product views.

---

## Finding waiver wire pickups

Use `poach` for fantasy-specific pickup recommendations:

```bash
icelines poach --availability imported-available --category hits,blocks --top 15
icelines report weekly --availability imported-available --category shots
```

You can still use `query leaders` for raw player searches:

```bash
# High-pace players with limited GP (just returned from injury / recent callup)
icelines query leaders --gp-min 10 --gp-max 30 --sort pts-pace --top 15

# Undrafted sleepers with high production
icelines query leaders --undrafted --ppg-min 0.60 --sort ppg

# Rookies with strong starts
icelines query leaders --rookie --sort ppg --top 15
```

---

## In-season Yahoo workflow

IceLines can store immutable observations from a private Yahoo league without
embedding league-specific teams or credentials in this repository. Copy
`examples/fantasy-yahoo-platform-snapshot.json`, replace the example standings,
current matchup, goalie appearances, and player statuses, then preview it:

```bash
icelines fantasy snapshot-yahoo --file yahoo-week.json --league "My League"
icelines fantasy snapshot-yahoo --file yahoo-week.json --league "My League" --apply
icelines fantasy snapshot-show --league "My League"
```

Applying a snapshot records standings history, confirmed platform injury
statuses, and the included weekly matchup. `matchup-plan`, `goalie-plan`, and
`morning` then use matching saved matchup points or goalie appearances when a
manual positive value was not supplied. The source JSON remains the explicit
boundary for private platform observations; IceLines does not log in to Yahoo.

To evaluate whether the bench actually creates starts rather than merely adding
games, run the exact-date substitution planner:

```bash
icelines fantasy bench-coverage --week 2026-10-05 --weeks 3 --league "My League"
```

The report identifies each inferred baseline bench player, the starter and slot
they can cover, usable dates, crowded-slate collisions, and starter dates that
remain uncovered. Saved roster membership does not contain Yahoo's BN labels,
so the baseline bench is inferred by optimizing a legal full-roster lineup.

---

## Manage your roster

```bash
icelines fantasy team-drop "Gio's Rangers" "Bouchard"   # drop after trade
icelines fantasy league-delete "Test League"             # clean up test leagues
```

Deleting a league cascades — removes all teams and rosters in that league.

---

## Notes

- Goalies are scored through the goalie side of the active fantasy scheme.
- Player uniqueness is scoped per-league — the same player can be on teams in different leagues
- Fantasy scores are cumulative season stats × scheme weights (not daily)
- For daily delta scoring, run `icelines fetch stats` each day and compare snapshots
