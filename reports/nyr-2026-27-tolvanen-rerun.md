# New York Rangers 2026-27 rerun — Eeli Tolvanen signing

**Evidence date:** 2026-09-02

**IceCast contract:** `team_season_forecast.v1`

**Simulation:** 10,000 trials, seed `20262027`, complete 1,344-game league schedule

## Result

| NYR outcome | Before signing | With Tolvanen | Change |
|---|---:|---:|---:|
| Average points | 94.8655 | 94.9089 | +0.0434 |
| P10 / median / P90 points | 84 / 95 / 106 | 84 / 95 / 106 | unchanged |
| Playoff probability | 51.31% | 51.41% | +0.10 pp |
| Second round | 24.93% | 24.98% | +0.05 pp |
| Conference final | 11.53% | 11.59% | +0.06 pp |
| Stanley Cup Final | 5.49% | 5.52% | +0.03 pp |
| Stanley Cup | 2.63% | 2.68% | +0.05 pp |

The rounded forecast remains **95 points and roughly a 51% playoff chance**.
Tolvanen improves the forward floor and lineup competition, but this model does
not treat a middle-six winger replacing the fringe of the roster as a major
team-level swing.

## Player and lineup evidence

- Tolvanen signed a one-year, $1.5 million Rangers contract on 2026-09-01:
  <https://puckpedia.com/signing/10786>.
- IceLines' sealed 2025-26 data records 78 GP, 12 goals, 24 assists, 36 points,
  110.37 seconds of power-play time per game, and 62.22 seconds of short-handed
  time per game.
- The updated camp simulation gives Tolvanen a 97.98% opening-roster
  probability and a 95.60% dressed probability.
- The pre-signing modal Blender score was 37.2214. The updated modal score is
  37.8799. Applying the documented 0.25 score-to-team-strength conversion gives
  the signing event a `+0.1646` strength delta.

## Reproduction

```powershell
target/debug/icelines.exe icecast camp `
  --input examples/icecast-nyr-training-camp.json `
  --trials 10000 --seed 20262027 --json `
  --out examples/icecast-nyr-training-camp-result.json `
  --lineup-set-out examples/icecast-nyr-training-camp-lineups.json `
  --blender-set-out examples/icecast-nyr-training-camp-blenders.json `
  --season-scenario-out examples/icecast-nyr-training-camp-season.json `
  --max-lineup-branches 5 --season-max-roster-branches 3000

target/debug/icelines.exe icecast season `
  --season 20262027 --stats-season 20252026 --team NYR `
  --trials 10000 --seed 20262027 `
  --scenario examples/icecast-nyr-tolvanen-signing-2026-27.json `
  --isolated-impacts --json `
  --out dist/tolvanen-rerun/nyr-tolvanen-isolated.json
```

The scenario is paired against an internally generated same-seed no-signing
baseline. The output is a model estimate, not a betting line or guarantee.
