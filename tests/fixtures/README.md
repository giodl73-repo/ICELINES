# IceLines Test Fixtures

This directory contains all test fixtures used by L1 integration tests and L2 system tests.

## The 9 BENCH Archetypes

Every fixture file covers exactly these 9 players (player IDs 8480001–8480009).
These are synthetic players — not real NHL players — with properties designed to
exercise every code path in IceLines.

| ID | Name | Team | Pos | GP | G | A | Role |
|----|------|------|-----|----|---|---|------|
| 8480001 | Connor McPlayer | EDM | C  | 82 | 50 | 90 | **Elite** — McDavid-tier, always Elite classification |
| 8480002 | Solid Skater    | SEA | LW | 74 | 28 | 40 | **Solid** — Good pace, fits lineup slot |
| 8480003 | Hidden Gem      | SEA | C  | 68 | 15 | 22 | **Buried** — Above-average pace, playing low line slot |
| 8480004 | Stretched Thin  | SEA | RW | 55 |  4 |  8 | **Stretch** — Below pace for his line role |
| 8480005 | Journey Mann    | MIN | D  | 40 | 12 | 18 | **Traded** — Mid-season trade (SEA→MIN), split stats |
| 8480006 | Young Star      | SEA | C  | 22 |  5 |  7 | **Rookie** — Small sample, GP=22 (above MIN_GP=10) |
| 8480007 | Hurt Hero       | SEA | LW | 10 |  3 |  5 | **Injured** — Exactly GP=MIN_GP=10, included in rankings |
| 8480008 | Sitout Steve    | SEA | RW |  0 |  0 |  0 | **Absent** — GP=0, always excluded, pace_score=None |
| 8480009 | Utility Mann    | SEA | C+LW | 78 | 20 | 35 | **Multi** — Eligible at C and LW |

## Expected Values (document in every test assertion)

### Pace projections (pts/82 = (G+A)/GP × 82):

| Archetype | Calculation | Result |
|-----------|-------------|--------|
| Elite | (50+90)/82 × 82 | **140.000** |
| Solid | (28+40)/74 × 82 | **75.351** |
| Buried | (15+22)/68 × 82 | **44.647** |
| Stretch | (4+8)/55 × 82 | **17.891** |
| Traded | (12+18)/40 × 82 | **61.500** |
| Rookie | (5+7)/22 × 82 | **44.727** |
| Injured | (3+5)/10 × 82 | **65.600** (at exactly MIN_GP) |
| Absent | n/a | **None** (GP=0) |
| Multi | (20+35)/78 × 82 | **57.821** |

### Fantasy points (yahoo-standard: G=3, A=2, +PPG=1, +PPA=0.5, +SHG=1, GWG=0.5, HIT=0.5, BLK=0.5):

| Archetype | Calculation | Result |
|-----------|-------------|--------|
| Elite | 50×3 + 90×2 + 16×1 + 39×0.5 + 2×1 + 2×0.5 + 8×0.5 + 40×0.5 + 30×0.5 | **407.5** |
| Absent | 0 | **0.0** (no fantasy score) |

## File Index

```
fixtures/
├── README.md                          # This file
├── sample_skaters.csv                 # Yahoo-format CSV with 9 archetypes
├── moneypuck/
│   └── skaters_schema_sample.csv       # MoneyPuck-shaped skater CSV schema fixture
└── api/
    ├── bios_page1.json                # NHL /skater/bios response, 9 players, total=9
    ├── stats_page1.json               # NHL /skater/summary response, 9 players
    ├── roster_SEA.json                # NHL /v1/roster/SEA/20252026
    ├── roster_COL.json                # NHL /v1/roster/COL/20252026 (TODO)
    ├── boxscore_2025020001.json        # NHL /v1/gamecenter/2025020001/boxscore
    ├── player_8480001_landing.json    # NHL /v1/player/8480001/landing (TODO)
    └── schedule_today.json            # NHL /v1/schedule/now (TODO)
```

Files marked TODO are needed for Phase 2+ tests. Add them as the corresponding
commands are implemented.

The MoneyPuck fixture is a compact schema fixture, not a full upstream data
snapshot. It locks the currently parsed skater CSV columns plus a few ignored
source columns so parser tests can distinguish required-column drift from
unsupported future deployment surfacing.

## Mock Server

`tests/mock/mod.rs` contains `MockNhlServer` — a thin wrapper around `httpmock`
that provides ergonomic registration helpers and uses these fixture files.

See `docs/specs/test-strategy.md` for the full L0/L1/L2 test strategy.
