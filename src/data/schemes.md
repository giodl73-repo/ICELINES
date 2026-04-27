# Fantasy Scoring Schemes

IceLines includes three built-in schemes and supports custom schemes.

<!-- proof:figure id="scheme-weights" kind="table.reference" -->
## Built-in scheme weights

| Stat | Yahoo standard | ESPN standard | Simple pts |
|------|---------------|---------------|------------|
| Goals | 3.0 | 6.0 | 1.0 |
| Assists | 2.0 | 4.0 | 1.0 |
| PP Goals (bonus) | 1.0 | 2.0 | 0.0 |
| PP Assists (bonus) | 0.5 | 2.0 | 0.0 |
| SH Goals (bonus) | 1.0 | 3.0 | 0.0 |
| SH Assists (bonus) | 0.5 | 3.0 | 0.0 |
| Game-Winning Goals | 0.5 | 1.0 | 0.0 |
| Hits | 0.5 | 1.0 | 0.0 |
| Blocked Shots | 0.5 | 1.0 | 0.0 |
| Shots on Goal | 0.0 | 1.0 | 0.0 |
| Plus/Minus | 0.0 | 2.0 | 0.0 |

<!-- proof:figure id="scheme-example-score" kind="table.key-value" -->
## Example: Matty Beniers 2025-26 (82 GP)

| Scheme | Components | Total |
|--------|-----------|-------|
| Yahoo standard | 20G×3 + 30A×2 + 6PPG×1 + 5PPA×0.5 + 1GWG×0.5 + 31HIT×0.5 + 69BLK×0.5 | **179.0** |
| ESPN standard | 20G×6 + 30A×4 + 6PPG×2 + 5PPA×2 + 1GWG×1 + 31HIT×1 + 69BLK×1 | **323.0** |
| Simple pts | 20G×1 + 30A×1 | **50.0** |

Note: PP bonus stacks ON TOP of the base goal/assist weight.

<!-- proof:figure id="scheme-commands" kind="table.reference" -->
## Scheme commands

| Command | Description |
|---------|-------------|
| `icelines scheme list` | List all available schemes |
| `icelines scheme show yahoo-standard` | Show weights for a specific scheme |
| `icelines fantasy league-create "My League" --scheme espn-standard` | Create league with specific scheme |
