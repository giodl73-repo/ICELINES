# Depth Chart & Line Value — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented (core algorithm) + Draft (TUI screens)

---

## Purpose

Answer: "What line would this player play if they were on any other team?"

The depth chart feature has two views:
1. **League rankings** — 32 teams ranked by depth score
2. **Team depth chart** — one team's LW/C/RW/LD/RD grid with fit coloring

Both views support two scoring modes and feed the cross-team line value algorithm.

---

## Core Algorithm

Implemented in `icelines-core/src/cross_team.rs`.

### Scoring Modes

| Mode | Formula | Use case |
|------|---------|----------|
| **Pace** (Pts/82) | `pace_score.sort_key()` | Pure hockey analysis |
| **Fantasy** | `G×3 + A×2 + PPG×1 + PPA×0.5 + SHG×1 + SHA×0.5 + GWG×0.5 + HIT×0.5 + BLK×0.5` | Yahoo fantasy league |

Toggle with `s` key on any depth screen.

### Cross-Team Line Value

```
for each player P at position pos with score S:
  own_line  = rank of S among own team's pos group (1-indexed)
  avg_line  = mean rank of S across ALL 32 teams' pos groups
  delta     = own_line - avg_line
              positive = player is buried (plays lower line than avg)
              negative = player is stretch (plays higher line than capable)
```

**Fit classification** (WebFitClass):

| Class | Symbol | Color | Condition |
|-------|--------|-------|-----------|
| Elite | `★` | Green | `avg_line ≤ own_line + 0.5` |
| Solid | `~` | Yellow | `avg_line ≤ own_line + 1.25` |
| Buried | `↑` | Cyan | `delta > 0.75` |
| Stretch | `↓` | Red | `avg_line > own_line + 1.25` |

A player is `Buried` when they rank significantly lower on their own team than they
would across the league — a trade-target signal.

**Threshold justification (resolved from PACE blocker)**:

Thresholds are the same for forwards and defensemen. Rationale: the metric is relative
(avg_line minus own_line), so position-specific adjustment is not needed. A delta of 0.75
means a player is, on average, 0.75 line positions higher across the league — roughly
"would play a line up on most teams." This is meaningful regardless of position.

Empirical basis: the 0.75 buried threshold was validated against the Python v2 analysis
(`Rangers/fantasy_team_analysis_output.txt`) where it correctly identified known buried
assets (players clearly underused relative to league-wide comparable value). The Elite
+0.5 and Solid +1.25 thresholds define a two-tier "good fit" band:
- Elite: avg league rank within half a line of own rank (playing at their level)
- Solid: within 1.25 lines (slightly above their level but manageable)
- Stretch: more than 1.25 lines above their level (significantly overextended)

These thresholds may be tuned in v2 based on user feedback; they are intentionally
conservative to avoid over-flagging players. Document any threshold changes in CHANGELOG.

**Position independence**: Defensemen and forwards use the same thresholds because:
1. The metric normalizes by rank within position group (D vs D, F vs F)
2. Line 1 for defense means "top pair" just as line 1 for forwards means "top line"
3. The relative distance concept (delta) is position-agnostic

**Greedy assignment rationale**: Greedy (highest-score-first) is used because it maximizes
the best players getting their best position slot, consistent with how coaches actually
build lines. Optimal assignment (Hungarian algorithm) would consider global optimum but
is O(N³) and not meaningfully different in practice since most players have a single
eligible position.

### Team Strength Score

For the league ranking table:
```
team_score = top-4 LW (score sum)
           + top-4 C  (score sum)
           + top-4 RW (score sum)
           + top-3 LD (score sum)    ← 3 pairs = 6 D slots, split LD/RD
           + top-3 RD (score sum)
```

This matches the Python v2 algorithm: 12 forwards + 6 defensemen.

---

## League Rankings View

Current: `Screen::Depth`  
v2: Tab 1 (League), sub-view 1 (default)

```
  Rk  Team    LW      C      RW      LD      RD    Total   Bar
   1  EDM    312    298     276     184     167    1237    ████████████████
   2  TBL    287    265     241     176     158    1127    ████████████░░░░
  ...
```

- Sorted by `total` descending
- Color tiers: green top-8, yellow mid-16, red bottom-8
- `s` toggles Fantasy / Pace
- `Enter` → team depth chart

---

## Team Depth Chart View

Current: `Screen::DepthTeam(String)`  
v2: Tab 1 (League) → drill-down

### Column layout

```
│ LEFT WING  │  CENTER   │ RIGHT WING │    LD     │    RD     │
│────────────│───────────│────────────│───────────│───────────│
│ L1 Panarin │ L1 Trocheck│ L1 Kreider │ L1 Fox    │ L1 Miller │
│   218 FPts │  205 FPts │  199 FPts  │  87 FPts  │  95 FPts  │
│    ★       │    ★      │    ★       │    ★      │    ★      │
│────────────│───────────│────────────│───────────│───────────│
│ L2 Kakko   │ L2 Chytil │ L2 Lafren. │ L2 Trouba │ L2 Lindgr.│
│   172 FPts │  164 FPts │  155 FPts  │  62 FPts  │  58 FPts  │
│    ~       │    ~      │    ~       │    ~      │    ~      │
│────────────│───────────│────────────│───────────│───────────│
│ L3 Vesey   │ L3 Goodrow│ L3 Blais   │ L3 Jones  │ L3 Schneid│
│    94 FPts │   88 FPts │   81 FPts  │  44 FPts  │  48 FPts  │
│    ↓       │    ↓      │    ↓       │    ↓      │    ↓      │
```

### Position assignment rules

**Forwards (LW / C / RW):**
1. Sort all forwards by score descending
2. Greedy assignment: each player goes to their primary position
3. If primary position is full (≥4 players): spill to natural wing
   - Left-hand shot → LW
   - Right-hand shot → RW
4. If natural wing is also full: spill to least-populated forward slot

**Defense (LD / RD):**
- Split by `shoots_catches` field: `"L"` → LD, `"R"` → RD
- Unknown/None → LD (safe default — most D shoot left)
- Rationale: left-hand shot D typically plays left side, right-hand shot plays right

**Starter depth:**
- Forwards: top 4 per position shown at full brightness; depth players dimmed
- Defense: top 3 per side (3 pairs × 2) shown at full brightness

---

## Fit Symbol Placement

Each player row shows: `L{n} {name:<14} {score:>5.0}  {fit}`

The fit symbol is colored:
- `★` = Green (Elite)
- `~` = Yellow (Solid)
- `↑` = Cyan (Buried — trade target)
- `↓` = Red (Stretch — overextended)

---

## Universal `g`/`f` Support

On both Depth screens, `g` opens the group picker and `f` adds to Favorites for the
currently highlighted player. The `get_selected_player()` method in `app.rs` handles
the screen-specific resolution.

> Note: in v1, the Depth screens don't have cursor-based row selection.
> v2 enhancement: add row selection to team depth chart for `g`/`f` to work on
> any specific player.

---

## Relation to Other Specs

- **fantasy-scheme.md** — defines the Fantasy scoring formula used here
- **tui.md** — current screen implementation details
- **tui-v2.md** — where these screens live in v2 nav structure
- **cross_team.rs** — the Rust implementation (icelines-core)

---

## Known Limitations

1. **`eligible_pos` is always single-element** — the data layer sets
   `eligible_pos: vec![position]` from NHL API data. True multi-position eligibility
   (e.g., a C/LW player) isn't captured. The greedy spill algorithm mitigates this
   for display purposes.

2. **Goalies excluded** — goalie depth not shown. See Open Questions below.

3. **No row cursor on depth chart** — can't navigate to a specific player with ↑↓.
   Pressing `g`/`f` on DepthTeam screen is not yet implemented.

---

## Open Questions

1. **Goalie tier** — should goalies get their own depth ranking? They have fantasy
   value (W×5, GA×-1, SVS×0.2, SHO×3 in Yahoo). Would require a goalie data model.
   Deferred pending goalie-spec.md.

2. **Row cursor for DepthTeam** — add `↑↓` navigation within the team chart for
   `g`/`f` targeting? Requires cursor state that spans columns.

3. **Historical cross-team analysis** — when time-traveling to a historical season,
   the cross-team rankings compare against that season's player pool. Is this always
   the right behavior, or should there be an option to compare against current season?
